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

        var usableWidth = Math.Max(120, _messages.ClientSize.Width - 28);
        var lineHeight = Math.Max(_messageFont.Height, _senderFont.Height) + 3;
        if (_settings.Chat.CompactMode)
        {
            var prefix = CompactPrefix(item.Message);
            var prefixWidth = TextRenderer.MeasureText(e.Graphics, prefix, _metaFont, Size.Empty, TextFormatFlags.NoPadding).Width +
                              TextRenderer.MeasureText(e.Graphics, DisplaySenderName(item.Message) + " ", _senderFont, Size.Empty, TextFormatFlags.NoPadding).Width;
            var messageWidth = Math.Max(80, usableWidth - prefixWidth);
            var size = TextRenderer.MeasureText(e.Graphics, item.Message.Text,
                _settings.Chat.BoldMessageText ? _messageBoldFont : _messageFont,
                new Size(messageWidth, int.MaxValue), TextFormatFlags.WordBreak | TextFormatFlags.NoPadding);
            e.ItemHeight = Math.Max(lineHeight, size.Height) + 10;
        }
        else
        {
            var size = TextRenderer.MeasureText(e.Graphics, item.Message.Text,
                _settings.Chat.BoldMessageText ? _messageBoldFont : _messageFont,
                new Size(usableWidth, int.MaxValue), TextFormatFlags.WordBreak | TextFormatFlags.NoPadding);
            e.ItemHeight = lineHeight + size.Height + 14;
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

        var x = e.Bounds.Left + (_settings.Chat.ShowColorBand ? 10 : 7);
        var y = e.Bounds.Top + 5;
        var right = e.Bounds.Right - 10;
        var textColor = ChatColorUtil.Blend(ChatUiTheme.Text, back, _settings.Chat.TextOpacity);
        var senderColor = ChatColorUtil.Blend(ChatSenderColor.ForMessage(item.Message), back, _settings.Chat.TextOpacity);
        var metaColor = ChatColorUtil.Blend(Color.FromArgb(157, 170, 188), back, _settings.Chat.TextOpacity);
        var messageFont = _settings.Chat.BoldMessageText ? _messageBoldFont : _messageFont;

        if (_settings.Chat.CompactMode)
        {
            x = DrawInline(e.Graphics, $"{GetChannelName(item.Message.Channel)} · ", _metaFont, channelColor, back, x, y);
            if (_settings.Chat.ShowTime) x = DrawInline(e.Graphics, GetTimeText(item.Message.Timestamp) + "  ", _metaFont, metaColor, back, x, y);
            x = DrawInline(e.Graphics, DisplaySenderName(item.Message) + "  ", _senderFont, senderColor, back, x, y);
            var rect = new Rectangle(x, y, Math.Max(20, right - x), Math.Max(18, e.Bounds.Bottom - y - 4));
            DrawWrapped(e.Graphics, item.Message.Text, messageFont, textColor, back, rect);
        }
        else
        {
            x = DrawInline(e.Graphics, GetChannelName(item.Message.Channel) + "  ", _metaFont, channelColor, back, x, y);
            x = DrawInline(e.Graphics, DisplaySenderName(item.Message), _senderFont, senderColor, back, x, y);
            if (_settings.Chat.ShowTime) _ = DrawInline(e.Graphics, "   " + GetTimeText(item.Message.Timestamp), _metaFont, metaColor, back, x, y);
            var messageY = y + Math.Max(_senderFont.Height, _metaFont.Height) + 4;
            var rect = new Rectangle(e.Bounds.Left + (_settings.Chat.ShowColorBand ? 10 : 7), messageY,
                Math.Max(20, right - e.Bounds.Left - 7), Math.Max(18, e.Bounds.Bottom - messageY - 4));
            DrawWrapped(e.Graphics, item.Message.Text, messageFont, textColor, back, rect);
        }

        if (_settings.Chat.ShowSeparators)
        {
            using var pen = new Pen(ChatColorUtil.Blend(Color.White, back, 10));
            e.Graphics.DrawLine(pen, e.Bounds.Left + 9, e.Bounds.Bottom - 1, e.Bounds.Right - 9, e.Bounds.Bottom - 1);
        }
    }

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
        ChatListScrollMath.ScrollToBottom(_messages);
        _unseenMessages = 0;
        UpdateNewMessagesButton();
    }

    private void UpdateNewMessagesButton()
    {
        _newMessagesButton.Visible = !_collapsed && !_followLatest && _unseenMessages > 0;
        _newMessagesButton.Text = _unseenMessages <= 1 ? "↓ 1 new message" : $"↓ {_unseenMessages} new messages";
        if (_newMessagesButton.Visible) _newMessagesButton.BringToFront();
    }

    private void PositionNewMessagesButton()
    {
        _newMessagesButton.Location = new Point(
            Math.Max(6, ClientSize.Width - _newMessagesButton.Width - 18),
            Math.Max(_topPanel.Bottom + 6, ClientSize.Height - _newMessagesButton.Height - 16));
    }
}