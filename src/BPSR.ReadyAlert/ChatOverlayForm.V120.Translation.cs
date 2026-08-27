using System.Collections.Concurrent;
using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatOverlayForm
{
    private const int V120PendingTranslationSlack = 64;

    private readonly ConcurrentQueue<ChatTranslationResult> _v120TranslationQueue = new();
    private readonly Dictionary<long, ChatTranslationResult> _v120Translations = new();
    private System.Windows.Forms.Timer? _v120TranslationTimer;

    private void AttachV120SpeechTranslation()
    {
        if (_settings.SpeechTranslation.TranslationEnabled || _settings.SpeechTranslation.TtsEnabled)
            ChatSpeechTranslationEngine.Configure(_settings.SpeechTranslation, _v120TranslationQueue);

        if (_v120TranslationTimer is null)
        {
            _v120TranslationTimer = new System.Windows.Forms.Timer { Interval = 80 };
            _v120TranslationTimer.Tick += (_, _) => DrainV120Translations();
        }
        _v120TranslationTimer.Start();
    }

    private void DrainV120Translations()
    {
        if (IsDisposed) return;
        var changedVisibleRow = false;
        while (_v120TranslationQueue.TryDequeue(out var result))
        {
            if (result.SequenceId == 0 || string.IsNullOrWhiteSpace(result.EnglishText)) continue;

            // Google/cache completion can race ahead of TrayApplicationContext's UI
            // queue. Keep a result even when its original row has not reached history
            // yet; monotonic SequenceId pruning below distinguishes a future row from
            // an already-aged-out row.
            _v120Translations[result.SequenceId] = result;

            if (!Visible || _collapsed) continue;
            for (var i = 0; i < _messages.Items.Count; i++)
            {
                if (_messages.Items[i] is ChatDisplayItem item && item.Message.SequenceId == result.SequenceId)
                {
                    changedVisibleRow = true;
                    break;
                }
            }
        }

        PruneV120Translations();

        // OwnerDrawVariable heights are measured when rows are inserted. Rebuild only
        // when a visible row actually gained a translation so the extra English line
        // gets the correct height while preserving the user's scroll anchor.
        if (changedVisibleRow)
            RebuildVisibleMessages(keepScroll: true);
    }

    private void PruneV120Translations()
    {
        if (_v120Translations.Count == 0) return;

        var live = _history
            .Where(x => x.SequenceId != 0)
            .Select(x => x.SequenceId)
            .ToHashSet();
        var maxSeenSequence = live.Count == 0 ? 0 : live.Max();

        // Results at or below the newest sequence already seen by the UI cannot be
        // waiting for a future row. If they are not in bounded history, they are stale.
        if (maxSeenSequence > 0)
        {
            foreach (var sequenceId in _v120Translations.Keys
                         .Where(x => x <= maxSeenSequence && !live.Contains(x))
                         .ToArray())
                _v120Translations.Remove(sequenceId);
        }

        // The engine result queue is bounded too, but retain a small amount of future
        // SequenceId slack here for the UI-ordering race. Prefer keeping translations
        // for live rows and the nearest future rows.
        var cap = Math.Clamp(_settings.Chat.MaxHistory, 10, 500) + V120PendingTranslationSlack;
        if (_v120Translations.Count <= cap) return;

        foreach (var sequenceId in _v120Translations.Keys
                     .Where(x => !live.Contains(x))
                     .OrderByDescending(x => x)
                     .ToArray())
        {
            if (_v120Translations.Count <= cap) break;
            _v120Translations.Remove(sequenceId);
        }

        if (_v120Translations.Count <= cap) return;
        foreach (var sequenceId in _v120Translations.Keys.OrderBy(x => x).ToArray())
        {
            if (_v120Translations.Count <= cap) break;
            _v120Translations.Remove(sequenceId);
        }
    }

    private string GetV120TranslationText(ChatMessageEvent message)
    {
        if (message.SequenceId == 0 ||
            !_settings.SpeechTranslation.ShowTranslationInOverlay ||
            !_settings.SpeechTranslation.TranslationEnabledFor(message.Channel))
            return string.Empty;

        return _v120Translations.TryGetValue(message.SequenceId, out var result)
            ? result.EnglishText
            : string.Empty;
    }

    private string GetV120TranslationLabel(ChatMessageEvent message)
    {
        var translation = GetV120TranslationText(message);
        return translation.Length == 0 ? string.Empty : "↳ EN: " + translation;
    }

    private Color GetV120TranslationColor(Color background) =>
        ChatColorUtil.Blend(Color.FromArgb(142, 196, 255), background, _settings.Chat.TextOpacity);

    private void RemoveV120Translation(ChatMessageEvent message)
    {
        if (message.SequenceId != 0)
            _v120Translations.Remove(message.SequenceId);
    }

    internal void EnqueueV120TranslationForSelfTest(ChatTranslationResult result) =>
        _v120TranslationQueue.Enqueue(result);

    internal void DrainV120TranslationsForSelfTest() =>
        DrainV120Translations();

    internal string GetV120TranslationTextForSelfTest(ChatMessageEvent message) =>
        GetV120TranslationText(message);

    private void ClearV120SpeechTranslationUi()
    {
        try { _v120TranslationTimer?.Stop(); } catch { }
        try { _v120TranslationTimer?.Dispose(); } catch { }
        _v120TranslationTimer = null;
        while (_v120TranslationQueue.TryDequeue(out _)) { }
        _v120Translations.Clear();
    }
}
