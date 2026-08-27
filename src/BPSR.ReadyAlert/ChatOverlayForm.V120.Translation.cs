using System.Collections.Concurrent;
using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatOverlayForm
{
    private readonly ConcurrentQueue<ChatTranslationResult> _v120TranslationQueue = new();
    private readonly Dictionary<long, ChatTranslationResult> _v120Translations = new();
    private System.Windows.Forms.Timer? _v120TranslationTimer;

    private void AttachV120SpeechTranslation()
    {
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

        // OwnerDrawVariable heights are measured when rows are inserted. Rebuild only
        // when a visible row actually gained a translation so the extra English line
        // gets the correct height while preserving the user's scroll anchor.
        if (changedVisibleRow)
            RebuildVisibleMessages(keepScroll: true);
    }

    private string GetV120TranslationText(ChatMessageEvent message)
    {
        if (!_settings.SpeechTranslation.ShowTranslationInOverlay || message.SequenceId == 0)
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

    private void ClearV120SpeechTranslationUi()
    {
        try { _v120TranslationTimer?.Stop(); } catch { }
        try { _v120TranslationTimer?.Dispose(); } catch { }
        _v120TranslationTimer = null;
        while (_v120TranslationQueue.TryDequeue(out _)) { }
        _v120Translations.Clear();
    }
}
