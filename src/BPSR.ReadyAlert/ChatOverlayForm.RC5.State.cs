using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatOverlayForm
{
    private void ApplyWindowSettings(bool registerHotkeys)
    {
        TopMost = _settings.Chat.TopMost;
        Opacity = Math.Clamp(_settings.Chat.WindowOpacity, 25, 100) / 100d;

        var body = ChatColorUtil.Blend(Color.FromArgb(16, 19, 23), Color.FromArgb(49, 56, 67), _settings.Chat.BackgroundOpacity);
        var toolbar = ChatColorUtil.Blend(Color.FromArgb(20, 24, 29), Color.FromArgb(56, 65, 78), _settings.Chat.ToolbarOpacity);
        _messages.BackColor = body;
        _emptyState.BackColor = body;
        _topPanel.BackColor = toolbar;
        _tabBar.BackColor = toolbar;
        _actionBar.BackColor = toolbar;
        _collapsedHandle.BackColor = ChatColorUtil.Blend(toolbar, Color.White, 8);
        foreach (var button in new[] { _addTabButton, _gearButton, _collapseButton, _hideButton, _dragGrip })
        {
            button.BackColor = toolbar;
            button.FlatAppearance.MouseOverBackColor = ChatColorUtil.Blend(toolbar, Color.White, 9);
            button.FlatAppearance.MouseDownBackColor = ChatColorUtil.Blend(toolbar, Color.White, 14);
        }

        CreateFonts();
        UpdateCollapseButtonGlyph();
        PositionNewMessagesButton();
        _messages.Invalidate();
        UpdateEmptyState();

        if (registerHotkeys && IsHandleCreated) RegisterHotkeys(showErrors: true);
        ApplyClickThrough();
    }

    private void CreateFonts()
    {
        Font newMessage;
        try
        {
            newMessage = new Font(_settings.Chat.FontFamily, _settings.Chat.FontSize, FontStyle.Regular, GraphicsUnit.Point);
        }
        catch
        {
            newMessage = new Font("Segoe UI", _settings.Chat.FontSize, FontStyle.Regular, GraphicsUnit.Point);
        }

        var newBold = new Font(newMessage, FontStyle.Bold);
        var newSender = new Font(newMessage.FontFamily, newMessage.Size, FontStyle.Bold, GraphicsUnit.Point);
        var newMeta = new Font(newMessage.FontFamily, Math.Max(8F, newMessage.Size - 1F), FontStyle.Regular, GraphicsUnit.Point);
        _messageFont?.Dispose();
        _messageBoldFont?.Dispose();
        _senderFont?.Dispose();
        _metaFont?.Dispose();
        _messageFont = newMessage;
        _messageBoldFont = newBold;
        _senderFont = newSender;
        _metaFont = newMeta;
        // Never assign these transient render fonts to _messages.Font. The native
        // ListBox deliberately keeps ChatMessageListBox's stable system font.
    }

    private void RemoveOverflowHistoryFromView()
    {
        var cap = Math.Clamp(_settings.Chat.MaxHistory, 10, 500);
        if (_history.Count <= cap)
        {
            UpdateEmptyState();
            return;
        }

        // Capture the user's viewport before removing the oldest rows. Native
        // WinForms ListBox can reset TopIndex when item 0 is deleted; if that
        // happens before AddMessage checks follow-latest state, the overlay thinks
        // the user intentionally scrolled up and starts accumulating unread chat.
        var keepFollowing = _followLatest && IsNearBottom();
        ChatMessageEvent? viewportAnchor = null;
        if (!keepFollowing && _messages.TopIndex >= 0 && _messages.TopIndex < _messages.Items.Count &&
            _messages.Items[_messages.TopIndex] is ChatDisplayItem topItem)
            viewportAnchor = topItem.Message;

        while (_history.Count > cap)
        {
            var removed = _history[0];
            _history.RemoveAt(0);
            for (var i = 0; i < _messages.Items.Count; i++)
            {
                if (_messages.Items[i] is ChatDisplayItem item && item.Message.Equals(removed))
                {
                    _messages.Items.RemoveAt(i);
                    break;
                }
            }
        }

        if (keepFollowing)
        {
            _followLatest = true;
            ScrollToLatest();
        }
        else if (viewportAnchor is { } anchor)
        {
            for (var i = 0; i < _messages.Items.Count; i++)
            {
                if (_messages.Items[i] is ChatDisplayItem item && item.Message.Equals(anchor))
                {
                    _messages.TopIndex = i;
                    break;
                }
            }
        }

        UpdateEmptyState();
    }

    private ChatTabSettings SelectedTab =>
        _settings.Chat.Tabs.FirstOrDefault(t => t.Id == _settings.Chat.LastSelectedTabId) ?? _settings.Chat.Tabs[0];

    private bool IsVisibleForTab(ChatMessageEvent message, ChatTabSettings tab)
    {
        if (!tab.Channels.Contains((int)message.Channel)) return false;
        if (message.SenderLevel > 0 && message.SenderLevel < tab.MinLevel) return false;
        if (_settings.Chat.BlockedUsers.Any(x => x.Id != 0 && x.Id == message.SenderId)) return false;
        if (_settings.Chat.HideStickers && message.Kind == ChatMessageKind.Sticker) return false;
        if (!ChatTabFilter.PassesTextRules(message, tab)) return false;
        return true;
    }

    private void RebuildVisibleMessages(bool keepScroll)
    {
        if (IsDisposed || _settings.Chat.Tabs.Count == 0 || _collapsed) return;

        ChatMessageEvent? oldTop = null;
        if (keepScroll && !_followLatest && _messages.TopIndex >= 0 && _messages.TopIndex < _messages.Items.Count &&
            _messages.Items[_messages.TopIndex] is ChatDisplayItem oldTopItem)
            oldTop = oldTopItem.Message;

        var shouldFollow = !keepScroll || _followLatest || IsNearBottom();
        _messages.BeginUpdate();
        try
        {
            _messages.Items.Clear();
            var tab = SelectedTab;
            foreach (var message in _history)
                if (IsVisibleForTab(message, tab)) _messages.Items.Add(CreateDisplayItem(message));
        }
        finally
        {
            _messages.EndUpdate();
        }

        if (shouldFollow)
        {
            _followLatest = true;
            ScrollToLatest();
        }
        else if (oldTop is { } topMessage)
        {
            for (var i = 0; i < _messages.Items.Count; i++)
            {
                if (_messages.Items[i] is ChatDisplayItem item && item.Message.Equals(topMessage))
                {
                    _messages.TopIndex = i;
                    break;
                }
            }
        }
        _messages.Invalidate();
        UpdateEmptyState();
    }

    private ChatDisplayItem CreateDisplayItem(ChatMessageEvent message)
    {
        var highlight = false;
        if (!string.IsNullOrWhiteSpace(_settings.Chat.HighlightIfMatches))
        {
            highlight = ChatFilterExpression.IsMatch(
                ChatTabFilter.SearchableText(message),
                _settings.Chat.HighlightIfMatches);
        }
        return new ChatDisplayItem(message, highlight, _settings.Chat.PrivateHighlightEnabled && message.Channel == ChatChannel.Private);
    }

    private void UpdateEmptyState()
    {
        if (_collapsed) return;
        var empty = _messages.Items.Count == 0;
        _messages.Visible = !empty;
        _emptyState.Visible = empty;
        if (!empty) return;

        if (_history.Count == 0)
        {
            _emptyTitle.Text = "Waiting for chat";
            _emptyHint.Text = "ReadyAlert is listening for BPSR chat messages on the shared capture pipeline.";
            _emptyStatusButton.Visible = true;
        }
        else
        {
            _emptyTitle.Text = "No messages in this tab";
            _emptyHint.Text = "Recent chat exists, but none matches this tab's channels, level rule or filters.";
            _emptyStatusButton.Visible = false;
        }
        _emptyState.BringToFront();
        _topPanel.BringToFront();
    }

    private sealed record ChatDisplayItem(
        ChatMessageEvent Message,
        bool IsHighlighted,
        bool IsPrivateHighlighted);
}