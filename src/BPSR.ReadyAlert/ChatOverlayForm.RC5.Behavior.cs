using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatOverlayForm
{
    internal void AddMessage(ChatMessageEvent message)
    {
        if (InvokeRequired)
        {
            BeginInvoke(new Action(() => AddMessage(message)));
            return;
        }

        message = SanitizeForDisplay(message);
        _history.Add(message);
        RemoveOverflowHistoryFromView();
        HandleMessageNotification(message);

        if (!Visible || _collapsed || _settings.Chat.Tabs.Count == 0)
            return;
        if (!IsVisibleForTab(message, SelectedTab))
            return;

        // _followLatest is the user's intent. Re-checking IsNearBottom here while
        // OwnerDrawVariable is inserting/measuring a new row can transiently report
        // false and flip Smart Scroll off, which caused the viewport to jump away
        // from the newest chat after every incoming message.
        var wasFollowing = _followLatest;
        _messages.Items.Add(CreateDisplayItem(message));
        UpdateEmptyState();

        if (wasFollowing)
        {
            _followLatest = true;
            ScrollToLatest();
        }
        else
        {
            _followLatest = false;
            _unseenMessages++;
            UpdateNewMessagesButton();
        }
    }

    internal void ShowOverlay()
    {
        if (InvokeRequired)
        {
            BeginInvoke(new Action(ShowOverlay));
            return;
        }
        if (_collapsed) ExpandFromEdge();
        ApplyWindowSettings(registerHotkeys: true);
        if (!Visible) Show();
        else BringToFront();
        RebuildVisibleMessages(keepScroll: false);
        UpdateEmptyState();
    }

    internal void HideOverlay()
    {
        if (InvokeRequired)
        {
            BeginInvoke(new Action(HideOverlay));
            return;
        }
        SaveWindowPlacement();
        Hide();
    }

    internal void OpenSettingsDialog()
    {
        var oldClickThrough = _settings.Chat.ClickThrough;
        using var dialog = new ChatGeneralSettingsForm(_settings.Chat);
        if (dialog.ShowDialog(this) != DialogResult.OK) return;

        _settings.Chat.Normalize();
        RemoveOverflowHistoryFromView();
        _settingsStore.Save(_settings);
        ApplyWindowSettings(registerHotkeys: true);
        RebuildTabBar();
        RebuildVisibleMessages(keepScroll: true);
        UpdateEmptyState();

        if (!oldClickThrough && _settings.Chat.ClickThrough)
            AppLog.Write("chat: click-through enabled; use " + _settings.Chat.ClickThroughHotkey + " to toggle it");
    }

    internal void Shutdown()
    {
        if (IsDisposed) return;
        SaveWindowPlacement();
        _relativeTimer.Stop();
        _resizeTimer.Stop();
        UnregisterHotkeys();
        _allowClose = true;
        Close();
        Dispose();
    }

    private void TryLoadIcon(string iconPath)
    {
        try
        {
            if (!File.Exists(iconPath)) return;
            using var stream = new FileStream(iconPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite);
            using var icon = new Icon(stream, 48, 48);
            Icon = (Icon)icon.Clone();
        }
        catch (Exception ex)
        {
            AppLog.Write("chat: icon load failed " + ex.Message);
        }
    }

    private Button MakeToolbarButton(string text, int width, string tooltip)
    {
        var button = new Button
        {
            Text = text,
            Width = width,
            Height = 42,
            FlatStyle = FlatStyle.Flat,
            ForeColor = ChatUiTheme.Text,
            BackColor = ChatUiTheme.Surface,
            TabStop = false,
            Margin = Padding.Empty,
            Cursor = Cursors.Hand,
            UseVisualStyleBackColor = false
        };
        button.FlatAppearance.BorderSize = 0;
        button.FlatAppearance.MouseOverBackColor = ChatUiTheme.SurfaceHover;
        button.FlatAppearance.MouseDownBackColor = ChatUiTheme.SurfaceRaised;
        _toolTip.SetToolTip(button, tooltip);
        return button;
    }

    private ContextMenuStrip BuildMessageMenu()
    {
        var menu = new ContextMenuStrip();
        var copyName = new ToolStripMenuItem("Copy player name");
        copyName.Click += (_, _) =>
        {
            if (_contextMessage is { } msg && !string.IsNullOrEmpty(msg.SenderName)) Clipboard.SetText(msg.SenderName);
        };
        var copyUid = new ToolStripMenuItem("Copy UID");
        copyUid.Click += (_, _) =>
        {
            if (_contextMessage is { } msg && msg.SenderId != 0) Clipboard.SetText(msg.SenderId.ToString());
        };
        var block = new ToolStripMenuItem("Block player in overlay");
        block.Click += (_, _) =>
        {
            if (_contextMessage is { } msg) BlockUser(msg);
        };
        menu.Items.Add(copyName);
        menu.Items.Add(copyUid);
        menu.Items.Add(new ToolStripSeparator());
        menu.Items.Add(block);
        menu.Opening += (_, e) =>
        {
            if (_contextMessage is not { } current)
            {
                e.Cancel = true;
                return;
            }
            copyName.Enabled = !string.IsNullOrEmpty(current.SenderName);
            copyUid.Enabled = current.SenderId != 0;
            block.Enabled = current.SenderId != 0;
        };
        return menu;
    }

    private static (TableLayoutPanel Panel, Label Title, Label Hint, Button StatusButton) BuildEmptyState()
    {
        var panel = new TableLayoutPanel
        {
            Dock = DockStyle.Fill,
            BackColor = ChatUiTheme.Window,
            ColumnCount = 1,
            RowCount = 5,
            Visible = true
        };
        panel.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        panel.RowStyles.Add(new RowStyle(SizeType.Percent, 45F));
        panel.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        panel.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        panel.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        panel.RowStyles.Add(new RowStyle(SizeType.Percent, 55F));

        var title = ChatUiTheme.Heading("Waiting for chat", 15F);
        title.Anchor = AnchorStyles.None;
        title.Margin = new Padding(12, 0, 12, 6);
        var hint = ChatUiTheme.Subheading("ReadyAlert is listening for BPSR chat messages on the shared capture pipeline.");
        hint.Anchor = AnchorStyles.None;
        hint.TextAlign = ContentAlignment.MiddleCenter;
        hint.MaximumSize = new Size(470, 0);
        hint.Margin = new Padding(12, 0, 12, 14);
        var status = new Button { Text = "Open capture status", Width = 160, Height = 34, Anchor = AnchorStyles.None, Margin = new Padding(0, 0, 0, 4) };
        ChatUiTheme.StyleSecondaryButton(status);
        status.Click += (_, _) =>
        {
            using var dialog = new ChatDebugStatusForm();
            dialog.ShowDialog();
        };

        panel.Controls.Add(new Panel(), 0, 0);
        panel.Controls.Add(title, 0, 1);
        panel.Controls.Add(hint, 0, 2);
        panel.Controls.Add(status, 0, 3);
        panel.Controls.Add(new Panel(), 0, 4);
        return (panel, title, hint, status);
    }

    private static ChatMessageEvent SanitizeForDisplay(ChatMessageEvent message)
    {
        var name = (message.SenderName ?? string.Empty)
            .Replace('\r', ' ')
            .Replace('\n', ' ')
            .Replace('\0', ' ')
            .Trim();
        if (name.Length > MaxSenderNameLength) name = name[..MaxSenderNameLength];
        var text = (message.Text ?? string.Empty).Replace("\0", string.Empty, StringComparison.Ordinal);
        if (text.Length > MaxDisplayedMessageLength) text = text[..MaxDisplayedMessageLength] + "…";
        return message with { SenderName = name, Text = text };
    }

    private void MessagesMouseDown(object? sender, MouseEventArgs e)
    {
        if (e.Button != MouseButtons.Right)
        {
            _contextMessage = null;
            return;
        }
        var index = _messages.IndexFromPoint(e.Location);
        if (index < 0 || index >= _messages.Items.Count || _messages.Items[index] is not ChatDisplayItem item)
        {
            _contextMessage = null;
            return;
        }
        _messages.SelectedIndex = index;
        _contextMessage = item.Message;
    }

    private void BlockUser(ChatMessageEvent message)
    {
        if (message.SenderId == 0 || _settings.Chat.BlockedUsers.Any(x => x.Id == message.SenderId)) return;
        _settings.Chat.BlockedUsers.Add(new ChatBlockedUser
        {
            Id = message.SenderId,
            Name = message.SenderName,
            BlockedAtUtc = DateTime.UtcNow
        });
        _settingsStore.Save(_settings);
        RebuildVisibleMessages(keepScroll: true);
    }

    private void RebuildTabBar()
    {
        _tabBar.SuspendLayout();
        try
        {
            _tabBar.Controls.Clear();
            foreach (var tab in _settings.Chat.Tabs)
                _tabBar.Controls.Add(MakeTabButton(tab));
        }
        finally
        {
            _tabBar.ResumeLayout();
        }
    }

    private Button MakeTabButton(ChatTabSettings tab)
    {
        var button = new ChatTabButton
        {
            Text = tab.Name,
            Selected = tab.Id == _settings.Chat.LastSelectedTabId,
            Tag = tab,
            AccessibleName = $"Chat tab {tab.Name}"
        };
        button.Click += (_, _) => SelectTab(tab.Id);

        var menu = new ContextMenuStrip();
        var edit = new ToolStripMenuItem("Edit tab…");
        edit.Click += (_, _) => EditTab(tab);
        var delete = new ToolStripMenuItem("Delete tab…");
        delete.Click += (_, _) => DeleteTab(tab);
        menu.Items.Add(edit);
        menu.Items.Add(delete);
        button.ContextMenuStrip = menu;
        return button;
    }

    private void SelectTab(long id)
    {
        if (!_settings.Chat.Tabs.Any(t => t.Id == id)) return;
        _settings.Chat.LastSelectedTabId = id;
        _settingsStore.Save(_settings);
        RebuildTabBar();
        _followLatest = true;
        _unseenMessages = 0;
        UpdateNewMessagesButton();
        RebuildVisibleMessages(keepScroll: false);
        UpdateEmptyState();
    }

    private void AddTab()
    {
        var tab = new ChatTabSettings { Name = "New Tab", MinLevel = 1, Channels = [(int)ChatChannel.World] };
        using var editor = new ChatTabEditorForm(tab, isNew: true);
        if (editor.ShowDialog(this) != DialogResult.OK) return;
        _settings.Chat.Tabs.Add(tab);
        _settings.Chat.LastSelectedTabId = tab.Id;
        _settingsStore.Save(_settings);
        RebuildTabBar();
        RebuildVisibleMessages(keepScroll: false);
        UpdateEmptyState();
    }

    private void EditTab(ChatTabSettings tab)
    {
        var working = tab.Clone();
        using var editor = new ChatTabEditorForm(working, isNew: false);
        if (editor.ShowDialog(this) != DialogResult.OK) return;
        tab.Name = working.Name;
        tab.Channels = working.Channels;
        tab.MinLevel = working.MinLevel;
        tab.ShowIfMatches = working.ShowIfMatches;
        tab.HideIfMatches = working.HideIfMatches;
        _settingsStore.Save(_settings);
        RebuildTabBar();
        RebuildVisibleMessages(keepScroll: true);
        UpdateEmptyState();
    }

    private void DeleteTab(ChatTabSettings tab)
    {
        if (_settings.Chat.Tabs.Count <= 1)
        {
            MessageBox.Show(this, "At least one chat tab is required.", "Chat tab", MessageBoxButtons.OK, MessageBoxIcon.Information);
            return;
        }
        if (MessageBox.Show(this, $"Delete '{tab.Name}'?", "Delete chat tab", MessageBoxButtons.YesNo, MessageBoxIcon.Question) != DialogResult.Yes)
            return;
        _settings.Chat.Tabs.Remove(tab);
        if (_settings.Chat.LastSelectedTabId == tab.Id) _settings.Chat.LastSelectedTabId = _settings.Chat.Tabs[0].Id;
        _settingsStore.Save(_settings);
        RebuildTabBar();
        RebuildVisibleMessages(keepScroll: false);
        UpdateEmptyState();
    }
}
