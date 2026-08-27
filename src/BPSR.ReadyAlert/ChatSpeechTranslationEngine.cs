using System.Collections.Concurrent;
using System.Runtime.InteropServices;
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
///
/// It deliberately does not own any capture state and never blocks the Npcap/parser
/// thread or WinForms UI. Google access uses the same undocumented Google Translate
/// web functionality commonly used by gTTS-style clients: no Cloud project/API key.
/// Upstream behavior can change, so all failures are soft and leave normal chat intact.
/// </summary>
internal static class ChatSpeechTranslationEngine
{
    private const int MaxQueuedJobs = 24;
    private const int MaxTranslationChars = 1_000;
    private const int MaxSpeechChars = 500;
    private const int MaxCacheEntries = 256;
    private static readonly TimeSpan MaxJobAge = TimeSpan.FromSeconds(20);

    private static readonly ConcurrentQueue<SpeechJob> Jobs = new();
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
                while (Jobs.TryDequeue(out _))
                    Interlocked.Increment(ref _dropped);
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
        Volatile.Write(ref _snapshot, SpeechSnapshot.From(settings));
        if (translationResults is not null)
            Volatile.Write(ref _translationResults, translationResults);
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

        while (Jobs.Count >= MaxQueuedJobs && Jobs.TryDequeue(out _))
            Interlocked.Increment(ref _dropped);

        Jobs.Enqueue(new SpeechJob(message, DateTime.UtcNow));
        EnsureWorker();
        TryWake();
    }

    internal static ChatSpeechTranslationStatus GetStatus()
    {
        var ticks = Interlocked.Read(ref _lastSuccessUtcTicks);
        return new ChatSpeechTranslationStatus(
            Enabled,
            Jobs.Count,
            Interlocked.Read(ref _processed),
            Interlocked.Read(ref _translated),
            Interlocked.Read(ref _spoken),
            Interlocked.Read(ref _translationFailures),
            Interlocked.Read(ref _ttsFailures),
            Interlocked.Read(ref _dropped),
            ticks > 0 ? new DateTime(ticks, DateTimeKind.Utc) : null);
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

            while (!cancellationToken.IsCancellationRequested && Jobs.TryDequeue(out var job))
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

        if (!wantsSpeech || snapshot.TtsVolume <= 0) return;

        var speechText = outcome.Success ? outcome.EnglishText : sourceText;
        speechText = CleanText(speechText, MaxSpeechChars);
        if (speechText.Length == 0) return;

        if (snapshot.ReadSenderName && !string.IsNullOrWhiteSpace(message.SenderName))
        {
            var sender = CleanText(message.SenderName, 80);
            if (sender.Length > 0) speechText = sender + ". " + speechText;
        }

        try
        {
            var audio = await DownloadGoogleTtsAsync(speechText, cancellationToken).ConfigureAwait(false);
            await MciMp3Player.PlayAsync(audio, snapshot.TtsVolume, cancellationToken).ConfigureAwait(false);
            Interlocked.Increment(ref _spoken);
            Interlocked.Exchange(ref _lastSuccessUtcTicks, DateTime.UtcNow.Ticks);
        }
        catch (Exception ex) when (ex is not OperationCanceledException)
        {
            Interlocked.Increment(ref _ttsFailures);
            AppLog.Write("tts: google playback failed " + ex.Message);
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

    private static async Task<byte[]> DownloadGoogleTtsAsync(string text, CancellationToken cancellationToken)
    {
        var parts = SplitForTts(text, 180);
        if (parts.Count == 0) throw new InvalidDataException("There is no text to speak.");

        using var output = new MemoryStream();
        for (var i = 0; i < parts.Count; i++)
        {
            var part = parts[i];
            var url = "https://translate.google.com/translate_tts?ie=UTF-8&client=tw-ob&tl=en" +
                      $"&total={parts.Count}&idx={i}&textlen={part.Length}&q={Uri.EscapeDataString(part)}";
            using var request = new HttpRequestMessage(HttpMethod.Get, url);
            request.Headers.Referrer = new Uri("https://translate.google.com/");
            using var response = await Http.SendAsync(request, HttpCompletionOption.ResponseHeadersRead, cancellationToken).ConfigureAwait(false);
            response.EnsureSuccessStatusCode();
            var bytes = await response.Content.ReadAsByteArrayAsync(cancellationToken).ConfigureAwait(false);
            if (bytes.Length < 64 || LooksLikeHtml(bytes))
                throw new InvalidDataException("Google TTS returned a non-audio response.");
            await output.WriteAsync(bytes, cancellationToken).ConfigureAwait(false);
        }

        return output.ToArray();
    }

    internal static List<string> SplitForTts(string text, int maxChars)
    {
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
        var client = new HttpClient
        {
            Timeout = TimeSpan.FromSeconds(8)
        };
        client.DefaultRequestHeaders.UserAgent.ParseAdd(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/126 Safari/537.36");
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
            false, true, true, true, false, true, true, false, string.Empty, 70);

        internal static SpeechSnapshot From(ChatSpeechTranslationSettings settings) => new(
            settings.TranslationEnabled,
            settings.TranslationGuild,
            settings.TranslationPartyTeam,
            settings.ShowTranslationInOverlay,
            settings.TtsEnabled,
            settings.TtsGuild,
            settings.TtsPartyTeam,
            settings.ReadSenderName,
            settings.IgnoreOwnUsername,
            settings.TtsVolume);

        internal bool TranslationEnabledFor(ChatChannel channel) =>
            TranslationEnabled && ChatSpeechTranslationSettings.ChannelEnabled(channel, TranslationGuild, TranslationPartyTeam);

        internal bool TtsEnabledFor(ChatChannel channel) =>
            TtsEnabled && ChatSpeechTranslationSettings.ChannelEnabled(channel, TtsGuild, TtsPartyTeam);

        internal bool IsOwnUsername(string? senderName) =>
            IgnoreOwnUsername.Length > 0 &&
            string.Equals(IgnoreOwnUsername, senderName?.Trim(), StringComparison.OrdinalIgnoreCase);
    }
}

internal static class MciMp3Player
{
    private static int _aliasCounter;

    [DllImport("winmm.dll", CharSet = CharSet.Unicode)]
    private static extern int mciSendStringW(string command, StringBuilder? returnValue, int returnLength, IntPtr callback);

    [DllImport("winmm.dll", CharSet = CharSet.Unicode)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool mciGetErrorStringW(int errorCode, StringBuilder errorText, int errorTextSize);

    internal static async Task PlayAsync(byte[] mp3Bytes, int volume, CancellationToken cancellationToken)
    {
        if (mp3Bytes.Length == 0) return;
        var folder = Path.Combine(Path.GetTempPath(), "BPSR-ReadyAlert", "tts");
        Directory.CreateDirectory(folder);
        var path = Path.Combine(folder, $"speech-{Environment.ProcessId}-{Interlocked.Increment(ref _aliasCounter)}.mp3");
        var alias = "readyalerttts" + _aliasCounter;

        try
        {
            await File.WriteAllBytesAsync(path, mp3Bytes, cancellationToken).ConfigureAwait(false);
            Send($"open \"{path}\" type mpegvideo alias {alias}");
            Send($"setaudio {alias} volume to {Math.Clamp(volume, 0, 100) * 10}");
            Send($"play {alias}");

            while (!cancellationToken.IsCancellationRequested)
            {
                var mode = Query($"status {alias} mode");
                if (mode.Equals("stopped", StringComparison.OrdinalIgnoreCase) ||
                    mode.Equals("not ready", StringComparison.OrdinalIgnoreCase))
                    break;
                await Task.Delay(40, cancellationToken).ConfigureAwait(false);
            }
        }
        finally
        {
            try { _ = mciSendStringW($"stop {alias}", null, 0, IntPtr.Zero); } catch { }
            try { _ = mciSendStringW($"close {alias}", null, 0, IntPtr.Zero); } catch { }
            try { File.Delete(path); } catch { }
        }
    }

    private static void Send(string command)
    {
        var code = mciSendStringW(command, null, 0, IntPtr.Zero);
        if (code != 0) throw new InvalidOperationException("MCI: " + GetError(code));
    }

    private static string Query(string command)
    {
        var result = new StringBuilder(128);
        var code = mciSendStringW(command, result, result.Capacity, IntPtr.Zero);
        if (code != 0) throw new InvalidOperationException("MCI: " + GetError(code));
        return result.ToString().Trim();
    }

    private static string GetError(int code)
    {
        var text = new StringBuilder(256);
        return mciGetErrorStringW(code, text, text.Capacity) ? text.ToString() : "error " + code;
    }
}
