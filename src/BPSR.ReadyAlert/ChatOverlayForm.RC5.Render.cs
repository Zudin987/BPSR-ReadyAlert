using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatOverlayForm
{
    private void MessagesMeasureItem(object? sender, MeasureItemEventArgs e)
    {
        if (e.Index < 0 || e.Index >= _messages.Items.Count || _messages.Items[e.Index] is not ChatDisplayItem item ||
            _messageFont is null || _messageBoldFont is null || _senderFont is null || _metaFont is null)
        {
            e.ItemHeight = Math.Max(24, Font.Height + 10);
            return;
        }

        var scrollInset = _v111ScrollBar is { Visible: true } ? _v111ScrollBar.Width : 0;
        var usableWidth = Math.Max(120, _messages.ClientSize.Width - 28 - scrollInset);
        var lineHeight = Math.Max(_messageFont.Height, _senderFont.Height) + 3;
        var translationLabel = GetV120TranslationLabel(item.Message);
        var translationHeight = 0;
        if (translationLabel.Length > 0)
        {
            var translatedSize = TextRenderer.MeasureText(
                e.Graphics,
                translationLabel,
                _metaFont,
                new Size(usableWidth, int.MaxValue),
                TextFormatFlags.WordBreak | TextFormatFlags.NoPadding);
            translationHeight = translatedSize.Height + 4;
        }

        if (_settings.Chat.CompactMode)
        {
            var prefix = CompactPrefix(item.Message);
            var prefixWidth = TextRenderer.MeasureText(e.Graphics, prefix, _metaFont, Size.Empty, TextFormatFlags.NoPadding).Width +
                              TextRenderer.MeasureText(e.Graphics, DisplaySenderName(item.Message) + " ", _senderFont, Size.Empty, TextFormatFlags.NoPadding).Width;
            var messageWidth = Math.Max(80, usableWidth - prefixWidth);
            var size = TextRenderer.MeasureText(e.Graphics, item.Message.Text,
                _settings.Chat.BoldMessageText ? _messageBoldFont : _messageFont,
                new Size(messageWidth, int.MaxValue), TextFormatFlags.WordBreak | TextFormatFlags.NoPadding);
            e.ItemHeight = Math.Max(lineHeight, size.Height) + 10 + translationHeight;
        }
        else
        {
            var size = TextRenderer.MeasureText(e.Graphics, item.Message.Text,
                _settings.Chat.BoldMessageText ? _messageBoldFont : _messageFont,
                new Size(usableWidth, int.MaxValue), TextFormatFlags.WordBreak | TextFormatFlags.NoPadding);
            e.ItemHeight = lineHeight + size.Height + 14 + translationHeight;
        }
    }

    private void MessagesDrawItem(object? sender, DrawItemEventArgs e)
    {
        if (e.Index < 0 || e.Index >= _messages.Items.Count || _messages.Items[e.Index] is not ChatDisplayItem item ||
            _messageFont is null || _messageBoldFont is null || _senderFont is null || _metaFont is null)
            return;

        var baseBack = _messages.BackColor;
        var back = baseBack;
        if (_settings.Chat.ShowZebraStripes && (e.Index & 1) == 1)
            back = ChatColorUtil.Blend(Color.FromArgb(52, 58, 68), baseBack, 15);
        if (item.IsPrivateHighlighted)
            back = ChatColorUtil.Blend(ChatColorUtil.Parse(_settings.Chat.PrivateHighlightColor, Color.MediumPurple), back, 40);
        else if (item.IsHighlighted)
            back = ChatColorUtil.Blend(ChatColorUtil.Parse(_settings.Chat.HighlightColor, Color.DarkGoldenrod), back, 36);

        using (var brush = new SolidBrush(back)) e.Graphics.FillRectangle(brush, e.Bounds);

        var channelColor = GetChannelColor(item.Message.Channel);
        if (_settings.Chat.ShowColorBand)
        {
            using var band = new SolidBrush(channelColor);
            e.Graphics.FillRectangle(band, new Rectangle(e.Bounds.Left, e.Bounds.Top + 2, 3, Math.Max(1, e.Bounds.Height - 4)));
        }

        var contentLeft = e.Bounds.Left + (_settings.Chat.ShowColorBand ? 10 : 7);
        var x = contentLeft;
        var y = e.Bounds.Top + 5;
        var scrollInset = _v111ScrollBar is { Visible: true } ? _v111ScrollBar.Width : 0;
        var right = e.Bounds.Right - 10 - scrollInset;
        var usableWidth = Math.Max(20, right - contentLeft);
        var textColor = ChatColorUtil.Blend(ChatUiTheme.Text, back, _settings.Chat.TextOpacity);
        var senderColor = ChatColorUtil.Blend(ChatSenderColor.ForMessage(item.Message), back, _settings.Chat.TextOpacity);
        var metaColor = ChatColorUtil.Blend(Color.FromArgb(157, 170, 188), back, _settings.Chat.TextOpacity);
        var messageFont = _settings.Chat.BoldMessageText ? _messageBoldFont : _messageFont;
        var translationLabel = GetV120TranslationLabel(item.Message);
        var translationHeight = 0;
        if (translationLabel.Length > 0)
        {
            var translationSize = TextRenderer.MeasureText(
                e.Graphics,
                translationLabel,
                _metaFont,
                new Size(usableWidth, int.MaxValue),
                TextFormatFlags.WordBreak | TextFormatFlags.NoPadding);
            translationHeight = translationSize.Height + 4;
        }

        if (_settings.Chat.CompactMode)
        {
            x = DrawInline(e.Graphics, $"{GetChannelName(item.Message.Channel)} · ", _metaFont, channelColor, back, x, y);
            if (_settings.Chat.ShowTime) x = DrawInline(e.Graphics, GetTimeText(item.Message.Timestamp) + "  ", _metaFont, metaColor, back, x, y);
            x = DrawInline(e.Graphics, DisplaySenderName(item.Message) + "  ", _senderFont, senderColor, back, x, y);
            var messageBottom = Math.Max(y + 18, e.Bounds.Bottom - 4 - translationHeight);
            var rect = new Rectangle(x, y, Math.Max(20, right - x), Math.Max(18, messageBottom - y));
            DrawWrapped(e.Graphics, item.Message.Text, messageFont, textColor, back, rect);
        }
        else
        {
            x = DrawInline(e.Graphics, GetChannelName(item.Message.Channel) + "  ", _metaFont, channelColor, back, x, y);
            x = DrawInline(e.Graphics, DisplaySenderName(item.Message), _senderFont, senderColor, back, x, y);
            if (_settings.Chat.ShowTime) _ = DrawInline(e.Graphics, "   " + GetTimeText(item.Message.Timestamp), _metaFont, metaColor, back, x, y);
            var messageY = y + Math.Max(_senderFont.Height, _metaFont.Height) + 4;
            var messageBottom = Math.Max(messageY + 18, e.Bounds.Bottom - 4 - translationHeight);
            var rect = new Rectangle(contentLeft, messageY, usableWidth, Math.Max(18, messageBottom - messageY));
            DrawWrapped(e.Graphics, item.Message.Text, messageFont, textColor, back, rect);
        }

        if (translationLabel.Length > 0)
        {
            var translationY = Math.Max(e.Bounds.Top + 5, e.Bounds.Bottom - translationHeight - 2);
            var translationRect = new Rectangle(
                contentLeft,
                translationY,
                usableWidth,
                Math.Max(_metaFont.Height + 2, translationHeight));
            DrawWrapped(e.Graphics, translationLabel, _metaFont, GetV120TranslationColor(back), back, translationRect);
        }

        if (_settings.Chat.ShowSeparators)
        {
            using var pen = new Pen(ChatColorUtil.Blend(Color.White, back, 10));
            e.Graphics.DrawLine(
                pen,
                e.Bounds.Left + 9,
                e.Bounds.Bottom - 1,
                GetMessageSeparatorRight(e.Bounds),
                e.Bounds.Bottom - 1);
        }
    }

    // The custom dark scrollbar is overlaid on top of the ListBox rather than
    // consuming ListBox client width. Text intentionally keeps a scrollbar inset,
    // but the row divider should continue underneath that overlay so the visible
    // line reaches the scrollbar edge instead of stopping ~one scrollbar too early.
    private static int GetMessageSeparatorRight(Rectangle bounds) =>
        Math.Max(bounds.Left + 9, bounds.Right - 2);

    internal static int GetV125MessageSeparatorRightForSelfTest(Rectangle bounds) =>
        GetMessageSeparatorRight(bounds);

    private int DrawInline(Graphics graphics, string text, Font font, Color color, Color background, int x, int y)
    {
        var size = TextRenderer.MeasureText(graphics, text, font, Size.Empty, TextFormatFlags.NoPadding);
        var rect = new Rectangle(x, y, size.Width + 1, size.Height + 2);
        DrawText(graphics, text, font, color, background, rect, TextFormatFlags.NoPadding | TextFormatFlags.SingleLine);
        return x + size.Width;
    }

    private void DrawWrapped(Graphics graphics, string text, Font font, Color color, Color background, Rectangle rect) =>
        DrawText(graphics, text, font, color, background, rect, TextFormatFlags.WordBreak | TextFormatFlags.NoPadding | TextFormatFlags.TextBoxControl);

    private void DrawText(Graphics graphics, string text, Font font, Color color, Color background, Rectangle rect, TextFormatFlags flags)
    {
        if (_settings.Chat.TextShadow)
        {
            var shadow = ChatColorUtil.Blend(Color.Black, background, 56);
            var shadowRect = new Rectangle(rect.X + 1, rect.Y + 1, rect.Width, rect.Height);
            TextRenderer.DrawText(graphics, text, font, shadowRect, shadow, flags);
        }
        TextRenderer.DrawText(graphics, text, font, rect, color, flags);
    }

    private string CompactPrefix(ChatMessageEvent message)
    {
        var value = GetChannelName(message.Channel) + " · ";
        if (_settings.Chat.ShowTime) value += GetTimeText(message.Timestamp) + "  ";
        return value;
    }

    private bool IsNearBottom() => ChatListScrollMath.IsAtBottom(_messages);

    private void UpdateFollowLatestFromViewport()
    {
        if (_collapsed) return;
        if (IsNearBottom())
        {
            _followLatest = true;
            _unseenMessages = 0;
        }
        else
        {
            _followLatest = false;
        }
        UpdateNewMessagesButton();
    }

    private void ResumeFollowingLatest()
    {
        _followLatest = true;
        _unseenMessages = 0;
        ScrollToLatest();
        UpdateNewMessagesButton();
    }

    private void ScrollToLatest()
    {
        // A pending smooth-wheel target must never pull the viewport away again
        // after the user clicks "new messages" or ReadyAlert follows a fresh row.
        CancelV111SmoothScroll();
        ChatListScrollMath.ScrollToBottom(_messages);
        CancelV111SmoothScroll();
        _unseenMessages = 0;
        UpdateNewMessagesButton();
    }

    private void UpdateNewMessagesButton()
    {
        _newMessagesButton.Visible = !_collapsed && !_followLatest && _unseenMessages > 0;
        _newMessagesButton.Text = _unseenMessages <= 1 ? "↓ 1 new message" : $"↓ {_unseenMessages} new messages";
        if (_newMessagesButton.Visible) _newMessagesButton.BringToFront();
        SyncV111ScrollUx();
    }

    private void PositionNewMessagesButton()
    {
        var inset = _v111ScrollBar is { Visible: true } ? _v111ScrollBar.Width : 0;
        _newMessagesButton.Location = new Point(
            Math.Max(6, ClientSize.Width - _newMessagesButton.Width - 18 - inset),
            Math.Max(_topPanel.Bottom + 6, ClientSize.Height - _newMessagesButton.Height - 16));
        PositionV111ScrollBar();
    }
}
