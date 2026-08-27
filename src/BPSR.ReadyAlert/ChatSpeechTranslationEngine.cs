using System.Collections.Concurrent;
using System.Text;
using System.Text.Json;

namespace BPSR.ReadyAlert;

internal readonly record struct ChatTranslationResult(
    long SequenceId,
    string EnglishText,
    string SourceLanguage);

internal readonly record struct ChatSpeechTranslationStatus(
    bool Enabled,
    int QueueCount,
    long Processed,
    long Translated,
    long Spoken,
    long TranslationFailures,
    long TtsFailures,
    long Dropped,
    DateTime? LastSuccessUtc);

/// <summary>
/// Optional background translation/TTS pipeline for parsed BPSR chat.
/// Google access intentionally uses the no-key Translate/gTTS-style web endpoints,
/// not Google Cloud. All network and audio work stays off the Npcap/parser/UI paths.
/// </summary>
internal static class ChatSpeechTranslationEngine
{
    internal const string GoogleTtsLanguage = "en";

    private const int MaxSpeechQueuedJobs = 12;
    private const int MaxTranslationQueuedJobs = 24;
    private const int MaxTranslationChars = 1_000;
    private const int MaxSpeechChars = 500;
    private const int MaxCacheEntries = 256;
    private const int MaxTtsChunkChars = 200;
    private const int MaxTtsAudioBytes = 4 * 1024 * 1024;
    private static readonly TimeSpan MaxJobAge = TimeSpan.FromSeconds(20);

    // Keep a single worker so translation cache access and speech playback remain
    // serialized, but give jobs that can produce audible Guild/Party speech their
    // own priority queue. Busy World translation can therefore never sit ahead of
    // an entire burst of time-sensitive TTS jobs.
    private static readonly ConcurrentQueue<SpeechJob> SpeechJobs = new();
    private static readonly ConcurrentQueue<SpeechJob> TranslationJobs = new();
    private static readonly SemaphoreSlim Wake = new(0, int.MaxValue);
    private static readonly CancellationTokenSource ShutdownCts = new();
    private static readonly HttpClient Http = CreateHttpClient();
    private static readonly object StartLock = new();
    private static readonly Dictionary<string, TranslationOutcome> TranslationCache = new(StringComparer.Ordinal);
    private static readonly Queue<string> TranslationCacheOrder = new();

    private static volatile SpeechSnapshot _snapshot = SpeechSnapshot.Disabled;
    private static ConcurrentQueue<ChatTranslationResult>? _translationResults;
    private static Task? _worker;
    private static volatile bool _enabled;
    private static long _processed;
    private static long _translated;
    private static long _spoken;
    private static long _translationFailures;
    private static long _ttsFailures;
    private static long _dropped;
    private static long _lastSuccessUtcTicks;

    internal static bool Enabled
    {
        get => _enabled;
        set
        {
            _enabled = value;
            if (!value)
            {
                DrainQueue(SpeechJobs);
                DrainQueue(TranslationJobs);
            }
            else if (Volatile.Read(ref _snapshot).HasAnyFeature)
            {
                EnsureWorker();
            }
            TryWake();
        }
    }

    internal static void Configure(
        ChatSpeechTranslationSettings settings,
        ConcurrentQueue<ChatTranslationResult>? translationResults = null)
    {
        ArgumentNullException.ThrowIfNull(settings);
        settings.Normalize();
        var snapshot = SpeechSnapshot.From(settings);
        Volatile.Write(ref _snapshot, snapshot);
        if (translationResults is not null)
            Volatile.Write(ref _translationResults, translationResults);
        if (_enabled && snapshot.HasAnyFeature)
            EnsureWorker();
        TryWake();
    }

    internal static void Enqueue(ChatMessageEvent message)
    {
        if (!_enabled || message.SequenceId == 0 || string.IsNullOrWhiteSpace(message.Text)) return;
        if (message.Kind is ChatMessageKind.Sticker or ChatMessageKind.Picture) return;

        var snapshot = Volatile.Read(ref _snapshot);
        var wantsTranslation = snapshot.TranslationEnabledFor(message.Channel);
        var wantsSpeech = snapshot.TtsEnabledFor(message.Channel) && !snapshot.IsOwnUsername(message.SenderName);
        if (!wantsTranslation && !wantsSpeech) return;

        var job = new SpeechJob(message, DateTime.UtcNow);
        if (wantsSpeech)
            EnqueueBounded(SpeechJobs, job, MaxSpeechQueuedJobs);
        else
            EnqueueBounded(TranslationJobs, job, MaxTranslationQueuedJobs);

        EnsureWorker();
        TryWake();
    }

    internal static ChatSpeechTranslationStatus GetStatus()
    {
        var ticks = Interlocked.Read(ref _lastSuccessUtcTicks);
        return new ChatSpeechTranslationStatus(
            Enabled,
            SpeechJobs.Count + TranslationJobs.Count,
            Interlocked.Read(ref _processed),
            Interlocked.Read(ref _translated),
            Interlocked.Read(ref _spoken),
            Interlocked.Read(ref _translationFailures),
            Interlocked.Read(ref _ttsFailures),
            Interlocked.Read(ref _dropped),
            ticks > 0 ? new DateTime(ticks, DateTimeKind.Utc) : null);
    }

    internal static async Task TestTtsAsync(int volume, CancellationToken cancellationToken = default)
    {
        var chunks = await DownloadGoogleTtsChunksAsync(
            "ReadyAlert text to speech test.", cancellationToken).ConfigureAwait(false);
        foreach (var audio in chunks)
            await ChatTtsAudioPlayer.PlayAsync(audio, Math.Clamp(volume, 0, 100), cancellationToken).ConfigureAwait(false);
    }

    internal static void Shutdown()
    {
        try { ShutdownCts.Cancel(); } catch { }
        TryWake();
    }

    private static void EnsureWorker()
    {
        if (_worker is not null) return;
        lock (StartLock)
        {
            if (_worker is not null) return;
            _worker = Task.Run(() => WorkerLoopAsync(ShutdownCts.Token));
        }
    }

    private static void TryWake()
    {
        try { Wake.Release(); }
        catch (SemaphoreFullException) { }
        catch (ObjectDisposedException) { }
    }

    private static void EnqueueBounded(ConcurrentQueue<SpeechJob> queue, SpeechJob job, int maxCount)
    {
        while (queue.Count >= maxCount && queue.TryDequeue(out _))
            Interlocked.Increment(ref _dropped);
        queue.Enqueue(job);
    }

    private static void DrainQueue(ConcurrentQueue<SpeechJob> queue)
    {
        while (queue.TryDequeue(out _))
            Interlocked.Increment(ref _dropped);
    }

    private static bool TryDequeueNext(out SpeechJob job) =>
        TryDequeueNext(SpeechJobs, TranslationJobs, out job);

    private static bool TryDequeueNext(
        ConcurrentQueue<SpeechJob> speech,
        ConcurrentQueue<SpeechJob> translation,
        out SpeechJob job)
    {
        if (speech.TryDequeue(out job)) return true;
        return translation.TryDequeue(out job);
    }

    private static async Task WorkerLoopAsync(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                await Wake.WaitAsync(cancellationToken).ConfigureAwait(false);
            }
            catch (OperationCanceledException)
            {
                break;
            }

            while (!cancellationToken.IsCancellationRequested && TryDequeueNext(out var job))
            {
                if (!_enabled || DateTime.UtcNow - job.QueuedUtc > MaxJobAge)
                {
                    Interlocked.Increment(ref _dropped);
                    continue;
                }

                try
                {
                    await ProcessJobAsync(job, cancellationToken).ConfigureAwait(false);
                }
                catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
                {
                    return;
                }
                catch (Exception ex)
                {
                    AppLog.Write("speech: unexpected worker error " + ex.Message);
                }
            }
        }
    }

    private static async Task ProcessJobAsync(SpeechJob job, CancellationToken cancellationToken)
    {
        var message = job.Message;
        var snapshot = Volatile.Read(ref _snapshot);
        var wantsOverlayTranslation = snapshot.TranslationEnabledFor(message.Channel);
        var wantsSpeech = snapshot.TtsEnabledFor(message.Channel) && !snapshot.IsOwnUsername(message.SenderName);
        if (!wantsOverlayTranslation && !wantsSpeech) return;

        Interlocked.Increment(ref _processed);
        var sourceText = CleanText(message.Text, MaxTranslationChars);
        if (sourceText.Length == 0) return;

        TranslationOutcome outcome;
        try
        {
            outcome = await TranslateToEnglishAsync(sourceText, cancellationToken).ConfigureAwait(false);
        }
        catch (Exception ex) when (ex is not OperationCanceledException)
        {
            Interlocked.Increment(ref _translationFailures);
            AppLog.Write("translate: request failed " + ex.Message);
            outcome = new TranslationOutcome(sourceText, string.Empty, false, false);
        }

        if (wantsOverlayTranslation && snapshot.ShowTranslationInOverlay && outcome.WasTranslated)
        {
            var results = Volatile.Read(ref _translationResults);
            if (results is not null)
            {
                results.Enqueue(new ChatTranslationResult(message.SequenceId, outcome.EnglishText, outcome.SourceLanguage));
                Interlocked.Increment(ref _translated);
                Interlocked.Exchange(ref _lastSuccessUtcTicks, DateTime.UtcNow.Ticks);
            }
        }

        // TTS may have been switched off while the translation request was in flight.
        // Re-read live settings before any TTS download/playback so the toolbar OFF
        // action takes effect for the next not-yet-playing phrase.
        var liveSpeech = Volatile.Read(ref _snapshot);
        if (!liveSpeech.TtsEnabledFor(message.Channel) ||
            liveSpeech.IsOwnUsername(message.SenderName) ||
            liveSpeech.TtsVolume <= 0)
            return;

        var speechText = outcome.Success ? outcome.EnglishText : sourceText;
        speechText = CleanText(speechText, MaxSpeechChars);
        if (speechText.Length == 0) return;

        if (liveSpeech.ReadSenderName && !string.IsNullOrWhiteSpace(message.SenderName))
        {
            var sender = CleanText(message.SenderName, 80);
            if (sender.Length > 0) speechText = sender + ". " + speechText;
        }

        try
        {
            // Decode/play each Google MP3 response independently. Concatenating
            // separately-generated MP3 streams can produce invalid container/header
            // transitions on some Media Foundation versions.
            var audioChunks = await DownloadGoogleTtsChunksAsync(speechText, cancellationToken).ConfigureAwait(false);
            foreach (var audio in audioChunks)
            {
                liveSpeech = Volatile.Read(ref _snapshot);
                if (!liveSpeech.TtsEnabledFor(message.Channel) ||
                    liveSpeech.IsOwnUsername(message.SenderName) ||
                    liveSpeech.TtsVolume <= 0)
                    return;

                await ChatTtsAudioPlayer.PlayAsync(audio, liveSpeech.TtsVolume, cancellationToken).ConfigureAwait(false);
            }

            Interlocked.Increment(ref _spoken);
            Interlocked.Exchange(ref _lastSuccessUtcTicks, DateTime.UtcNow.Ticks);
        }
        catch (Exception ex) when (ex is not OperationCanceledException)
        {
            Interlocked.Increment(ref _ttsFailures);
            AppLog.Write("tts: google en playback failed " + ex.Message);
        }
    }

    private static async Task<TranslationOutcome> TranslateToEnglishAsync(string text, CancellationToken cancellationToken)
    {
        if (TranslationCache.TryGetValue(text, out var cached)) return cached;

        var url = "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=en&dt=t&q=" +
                  Uri.EscapeDataString(text);
        using var request = new HttpRequestMessage(HttpMethod.Get, url);
        request.Headers.Referrer = new Uri("https://translate.google.com/");
        using var response = await Http.SendAsync(request, HttpCompletionOption.ResponseHeadersRead, cancellationToken).ConfigureAwait(false);
        response.EnsureSuccessStatusCode();
        var json = await response.Content.ReadAsStringAsync(cancellationToken).ConfigureAwait(false);

        using var document = JsonDocument.Parse(json);
        var root = document.RootElement;
        if (root.ValueKind != JsonValueKind.Array || root.GetArrayLength() == 0)
            throw new InvalidDataException("Google Translate returned an unexpected response.");

        var builder = new StringBuilder();
        var segments = root[0];
        if (segments.ValueKind == JsonValueKind.Array)
        {
            foreach (var segment in segments.EnumerateArray())
            {
                if (segment.ValueKind == JsonValueKind.Array && segment.GetArrayLength() > 0 &&
                    segment[0].ValueKind == JsonValueKind.String)
                    builder.Append(segment[0].GetString());
            }
        }

        var translated = builder.ToString().Trim();
        if (translated.Length == 0)
            throw new InvalidDataException("Google Translate returned no translated text.");

        var sourceLanguage = root.GetArrayLength() > 2 && root[2].ValueKind == JsonValueKind.String
            ? root[2].GetString() ?? string.Empty
            : string.Empty;
        var isEnglish = sourceLanguage.Equals("en", StringComparison.OrdinalIgnoreCase) ||
                        sourceLanguage.StartsWith("en-", StringComparison.OrdinalIgnoreCase);
        var wasTranslated = !isEnglish && !string.Equals(translated, text, StringComparison.Ordinal);
        var outcome = new TranslationOutcome(translated, sourceLanguage, wasTranslated, true);
        AddTranslationCache(text, outcome);
        return outcome;
    }

    private static void AddTranslationCache(string source, TranslationOutcome outcome)
    {
        if (TranslationCache.ContainsKey(source)) return;
        TranslationCache[source] = outcome;
        TranslationCacheOrder.Enqueue(source);
        while (TranslationCacheOrder.Count > MaxCacheEntries)
        {
            var old = TranslationCacheOrder.Dequeue();
            TranslationCache.Remove(old);
        }
    }

    private static async Task<List<byte[]>> DownloadGoogleTtsChunksAsync(
        string text,
        CancellationToken cancellationToken)
    {
        var parts = SplitForTts(text, MaxTtsChunkChars);
        if (parts.Count == 0) throw new InvalidDataException("There is no text to speak.");

        var output = new List<byte[]>(parts.Count);
        long totalBytes = 0;
        for (var i = 0; i < parts.Count; i++)
        {
            var bytes = await FetchGoogleTtsChunkWithRetryAsync(parts[i], i, parts.Count, cancellationToken)
                .ConfigureAwait(false);
            totalBytes += bytes.Length;
            if (totalBytes > MaxTtsAudioBytes)
                throw new InvalidDataException("Google TTS response exceeded the 4 MiB safety limit.");
            output.Add(bytes);
        }
        return output;
    }

    private static async Task<byte[]> FetchGoogleTtsChunkWithRetryAsync(
        string text,
        int index,
        int total,
        CancellationToken cancellationToken)
    {
        Exception? lastError = null;
        for (var attempt = 0; attempt < 2; attempt++)
        {
            try
            {
                var url = "https://translate.google.com/translate_tts?ie=UTF-8&client=tw-ob" +
                          $"&tl={GoogleTtsLanguage}&total={total}&idx={index}&textlen={text.Length}&q={Uri.EscapeDataString(text)}";
                using var request = new HttpRequestMessage(HttpMethod.Get, url);
                request.Headers.Accept.ParseAdd("audio/mpeg,*/*;q=0.8");
                using var response = await Http.SendAsync(request, HttpCompletionOption.ResponseHeadersRead, cancellationToken)
                    .ConfigureAwait(false);

                if (!response.IsSuccessStatusCode)
                {
                    var status = (int)response.StatusCode;
                    var retryable = status == 408 || status == 429 || status >= 500;
                    if (attempt == 0 && retryable)
                    {
                        await Task.Delay(150, cancellationToken).ConfigureAwait(false);
                        continue;
                    }
                    throw new HttpRequestException($"Google English TTS returned HTTP {status}.");
                }

                var mediaType = response.Content.Headers.ContentType?.MediaType ?? string.Empty;
                if (!mediaType.StartsWith("audio/", StringComparison.OrdinalIgnoreCase))
                    throw new InvalidDataException("Google English TTS returned unexpected content type: " +
                                                   (mediaType.Length == 0 ? "unknown" : mediaType));

                var bytes = await response.Content.ReadAsByteArrayAsync(cancellationToken).ConfigureAwait(false);
                if (bytes.Length < 200 || LooksLikeHtml(bytes))
                    throw new InvalidDataException($"Google English TTS returned invalid audio ({bytes.Length} bytes).");
                return bytes;
            }
            catch (HttpRequestException ex) when (attempt == 0 && !cancellationToken.IsCancellationRequested)
            {
                lastError = ex;
                await Task.Delay(150, cancellationToken).ConfigureAwait(false);
            }
        }

        throw lastError ?? new HttpRequestException("Google English TTS request failed.");
    }

    internal static List<string> SplitForTts(string text, int maxChars)
    {
        maxChars = Math.Clamp(maxChars, 40, MaxTtsChunkChars);
        text = CleanText(text, Math.Max(maxChars, MaxSpeechChars));
        var parts = new List<string>();
        while (text.Length > maxChars)
        {
            var split = text.LastIndexOfAny([' ', ',', '.', ';', ':', '!', '?'], maxChars - 1, maxChars);
            if (split < maxChars / 2) split = maxChars;
            if (split < text.Length && split > 0 && char.IsHighSurrogate(text[split - 1])) split--;
            var part = text[..split].Trim();
            if (part.Length > 0) parts.Add(part);
            text = text[split..].TrimStart();
        }
        if (text.Length > 0) parts.Add(text);
        return parts;
    }

    internal static bool SpeechPriorityPrecedesTranslationForSelfTest(
        ChatMessageEvent translationOnly,
        ChatMessageEvent speechPriority)
    {
        var high = new ConcurrentQueue<SpeechJob>();
        var normal = new ConcurrentQueue<SpeechJob>();
        normal.Enqueue(new SpeechJob(translationOnly, DateTime.UtcNow));
        high.Enqueue(new SpeechJob(speechPriority, DateTime.UtcNow));
        return TryDequeueNext(high, normal, out var first) &&
               first.Message.SequenceId == speechPriority.SequenceId;
    }

    private static string CleanText(string? value, int maxChars)
    {
        if (string.IsNullOrWhiteSpace(value)) return string.Empty;
        var builder = new StringBuilder(Math.Min(value.Length, maxChars));
        var previousWhitespace = false;
        foreach (var ch in value)
        {
            if (ch == '\0') continue;
            if (char.IsWhiteSpace(ch))
            {
                if (previousWhitespace) continue;
                builder.Append(' ');
                previousWhitespace = true;
            }
            else
            {
                builder.Append(ch);
                previousWhitespace = false;
            }
            if (builder.Length >= maxChars) break;
        }
        return builder.ToString().Trim();
    }

    private static bool LooksLikeHtml(byte[] bytes)
    {
        var length = Math.Min(bytes.Length, 24);
        var prefix = Encoding.ASCII.GetString(bytes, 0, length).TrimStart();
        return prefix.StartsWith("<", StringComparison.Ordinal) ||
               prefix.StartsWith("{", StringComparison.Ordinal) ||
               prefix.StartsWith("[", StringComparison.Ordinal);
    }

    private static HttpClient CreateHttpClient()
    {
        var client = new HttpClient { Timeout = TimeSpan.FromSeconds(8) };
        client.DefaultRequestHeaders.UserAgent.ParseAdd("Mozilla/5.0 BPSR-ReadyAlert/1.2");
        client.DefaultRequestHeaders.AcceptLanguage.ParseAdd("en-US,en;q=0.9");
        return client;
    }

    private readonly record struct SpeechJob(ChatMessageEvent Message, DateTime QueuedUtc);

    private readonly record struct TranslationOutcome(
        string EnglishText,
        string SourceLanguage,
        bool WasTranslated,
        bool Success);

    private sealed record SpeechSnapshot(
        bool TranslationEnabled,
        bool TranslationWorld,
        bool TranslationGuild,
        bool TranslationPartyTeam,
        bool ShowTranslationInOverlay,
        bool TtsEnabled,
        bool TtsGuild,
        bool TtsPartyTeam,
        bool ReadSenderName,
        string IgnoreOwnUsername,
        int TtsVolume)
    {
        internal static readonly SpeechSnapshot Disabled = new(
            false, false, true, true, true,
            false, true, true, false, string.Empty, 70);

        internal static SpeechSnapshot From(ChatSpeechTranslationSettings settings) => new(
            settings.TranslationEnabled,
            settings.TranslationWorld,
            settings.TranslationGuild,
            settings.TranslationPartyTeam,
            settings.ShowTranslationInOverlay,
            settings.TtsEnabled,
            settings.TtsGuild,
            settings.TtsPartyTeam,
            settings.ReadSenderName,
            settings.IgnoreOwnUsername,
            settings.TtsVolume);

        internal bool HasAnyFeature =>
            (TranslationEnabled && (TranslationWorld || TranslationGuild || TranslationPartyTeam)) ||
            (TtsEnabled && (TtsGuild || TtsPartyTeam));

        internal bool TranslationEnabledFor(ChatChannel channel) =>
            TranslationEnabled && ChatSpeechTranslationSettings.TranslationChannelEnabled(
                channel, TranslationWorld, TranslationGuild, TranslationPartyTeam);

        internal bool TtsEnabledFor(ChatChannel channel) =>
            TtsEnabled && ChatSpeechTranslationSettings.TtsChannelEnabled(channel, TtsGuild, TtsPartyTeam);

        internal bool IsOwnUsername(string? senderName) =>
            IgnoreOwnUsername.Length > 0 &&
            string.Equals(IgnoreOwnUsername, senderName?.Trim(), StringComparison.OrdinalIgnoreCase);
    }
}
