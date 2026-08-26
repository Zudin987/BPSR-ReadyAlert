using System.Drawing;
using System.Media;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed class ChatOverlayForm : Form
{
    private const int MaxSenderNameLength = 128;
    private const int MaxDisplayedMessageLength = 8 * 1024;
    private const int ClickThroughHotkeyId = 0x5141;
    private const int CollapseHotkeyId = 0x5142;
    private const int ResizeGrip = 6;
    private const int CollapsedThickness = 22;

    private readonly AppSettings _settings;
    private readonly SettingsStore _settingsStore;
    private readonly string _defaultSoundPath;
    private readonly List<ChatMessageEvent> _history = [];
    private readonly FlowLayoutPanel _tabBar;
    private readonly ChatMessageListBox _messages;
    private readonly Button _gearButton;
    private readonly Button _collapseButton;
    private readonly Button _hideButton;
    private readonly Button _dragGrip;
    private readonly Panel _topPanel;
    private readonly Button _newMessagesButton;
    private readonly Button _collapsedHandle;
    private readonly System.Windows.Forms.Timer _relativeTimer;
    private readonly System.Windows.Forms.Timer _resizeTimer;
    private readonly SoundPlayer _chatSoundPlayer = new();
    private readonly ToolTip _toolTip = new();

    private ChatMessageEvent? _contextMessage;
    private Font? _messageFont;
    private Font? _messageBoldFont;
    private Font? _senderFont;
    private Font? _metaFont;
    private bool _allowClose;
    private bool _followLatest = true;
    private int _unseenMessages;
    private bool _clickThroughRegistered;
    private bool _collapseRegistered;
    private bool _collapsed;
    private Rectangle _expandedBounds;
    private DateTime _lastSoundUtc = DateTime.MinValue;
    private bool _disposedResources;

    internal ChatOverlayForm(
        AppSettings settings,
        SettingsStore settingsStore,
        string iconPath,
        string defaultSoundPath)
    {
        _settings = settings;
        _settingsStore = settingsStore;
        _defaultSoundPath = defaultSoundPath;
        _settings.Chat.Normalize();

        AutoScaleMode = AutoScaleMode.Dpi;
        AutoScaleDimensions = new SizeF(96F, 96F);
        Text = string.Empty;
        ShowInTaskbar = false;
        ShowIcon = false;
        ControlBox = false;
        StartPosition = FormStartPosition.Manual;
        MinimumSize = new Size(360, 180);
        Size = new Size(_settings.Chat.WindowWidth, _settings.Chat.WindowHeight);
        BackColor = Color.FromArgb(28, 30, 34);
        ForeColor = Color.Gainsboro;
        Font = new Font("Segoe UI", 9F, FontStyle.Regular, GraphicsUnit.Point);
        FormBorderStyle = FormBorderStyle.None;
        Padding = new Padding(1);
        DoubleBuffered = true;

        try
        {
            if (File.Exists(iconPath))
            {
                using var stream = new FileStream(iconPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite);
                using var icon = new Icon(stream, 48, 48);
                Icon = (Icon)icon.Clone();
            }
        }
        catch (Exception ex)
        {
            AppLog.Write("chat: icon load failed " + ex.Message);
        }

        RestoreWindowPlacement();
        _expandedBounds = Bounds;

        _topPanel = new Panel
        {
            Dock = DockStyle.Top,
            Height = 34,
            Padding = new Padding(0),
            BackColor = Color.FromArgb(36, 39, 44)
        };

        _dragGrip = MakeToolbarButton("≡", 30, "Drag chat window");
        _dragGrip.Dock = DockStyle.Left;
        _dragGrip.Cursor = Cursors.SizeAll;
        _dragGrip.MouseDown += (_, e) =>
        {
            if (e.Button == MouseButtons.Left && !_collapsed && !_settings.Chat.ClickThrough)
                ChatNativeMethods.BeginWindowDrag(Handle);
        };

        _hideButton = MakeToolbarButton("×", 34, "Hide chat (Chat Overlay stays enabled)");
        _hideButton.Dock = DockStyle.Right;
        _hideButton.Font = new Font("Segoe UI", 11F, FontStyle.Regular, GraphicsUnit.Point);
        _hideButton.Click += (_, _) => HideOverlay();

        _collapseButton = MakeToolbarButton("◀", 34, "Collapse chat to a screen edge");
        _collapseButton.Dock = DockStyle.Right;
        _collapseButton.Click += (_, _) => ToggleCollapsed();

        _gearButton = MakeToolbarButton("⚙", 38, "Chat settings");
        _gearButton.Dock = DockStyle.Right;
        _gearButton.AccessibleName = "Chat settings";
        _gearButton.Click += (_, _) => OpenSettingsDialog();

        _tabBar = new FlowLayoutPanel
        {
            Dock = DockStyle.Fill,
            FlowDirection = FlowDirection.LeftToRight,
            WrapContents = false,
            AutoScroll = true,
            Padding = new Padding(3, 3, 0, 0),
            Margin = Padding.Empty,
            BackColor = _topPanel.BackColor
        };

        _topPanel.Controls.Add(_tabBar);
        _topPanel.Controls.Add(_gearButton);
        _topPanel.Controls.Add(_collapseButton);
        _topPanel.Controls.Add(_hideButton);
        _topPanel.Controls.Add(_dragGrip);

        _messages = new ChatMessageListBox
        {
            Dock = DockStyle.Fill,
            BackColor = Color.FromArgb(22, 24, 28),
            ForeColor = Color.Gainsboro,
            AccessibleName = "BPSR chat messages"
        };
        _messages.MeasureItem += MessagesMeasureItem;
        _messages.DrawItem += MessagesDrawItem;
        _messages.MouseDown += MessagesMouseDown;
        _messages.ViewportChanged += (_, _) => UpdateFollowLatestFromViewport();

        var messageMenu = new ContextMenuStrip();
        var copyName = new ToolStripMenuItem("Copy Name");
        copyName.Click += (_, _) =>
        {
            if (_contextMessage is { } msg && !string.IsNullOrEmpty(msg.SenderName))
                Clipboard.SetText(msg.SenderName);
        };
        var copyUid = new ToolStripMenuItem("Copy UID");
        copyUid.Click += (_, _) =>
        {
            if (_contextMessage is { } msg && msg.SenderId != 0)
                Clipboard.SetText(msg.SenderId.ToString());
        };
        var block = new ToolStripMenuItem("Block User");
        block.Click += (_, _) =>
        {
            if (_contextMessage is { } msg) BlockUser(msg);
        };
        messageMenu.Items.Add(copyName);
        messageMenu.Items.Add(copyUid);
        messageMenu.Items.Add(new ToolStripSeparator());
        messageMenu.Items.Add(block);
        messageMenu.Opening += (_, e) =>
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
        _messages.ContextMenuStrip = messageMenu;

        _newMessagesButton = new Button
        {
            Text = "↓ New messages",
            Width = 150,
            Height = 28,
            FlatStyle = FlatStyle.Flat,
            Visible = false,
            TabStop = false,
            Anchor = AnchorStyles.Right | AnchorStyles.Bottom,
            BackColor = Color.FromArgb(57, 65, 76),
            ForeColor = Color.WhiteSmoke
        };
        _newMessagesButton.FlatAppearance.BorderColor = Color.FromArgb(90, 100, 112);
        _newMessagesButton.Click += (_, _) => ResumeFollowingLatest();

        _collapsedHandle = new Button
        {
            Dock = DockStyle.Fill,
            FlatStyle = FlatStyle.Flat,
            Text = "◀",
            Visible = false,
            TabStop = false,
            BackColor = Color.FromArgb(36, 39, 44),
            ForeColor = Color.WhiteSmoke,
            AccessibleName = "Expand chat overlay"
        };
        _collapsedHandle.FlatAppearance.BorderSize = 0;
        _collapsedHandle.Click += (_, _) => ExpandFromEdge();

        Controls.Add(_messages);
        Controls.Add(_topPanel);
        Controls.Add(_newMessagesButton);
        Controls.Add(_collapsedHandle);
        _newMessagesButton.BringToFront();

        _relativeTimer = new System.Windows.Forms.Timer { Interval = 15_000 };
        _relativeTimer.Tick += (_, _) =>
        {
            if (Visible && !_collapsed && _settings.Chat.ShowTime && _settings.Chat.ShowTimeAsAgo)
                _messages.Invalidate();
        };
        _relativeTimer.Start();

        _resizeTimer = new System.Windows.Forms.Timer { Interval = 120 };
        _resizeTimer.Tick += (_, _) =>
        {
            _resizeTimer.Stop();
            if (!_collapsed) RebuildVisibleMessages(keepScroll: true);
        };

        Resize += (_, _) =>
        {
            PositionNewMessagesButton();
            if (!_collapsed)
            {
                _resizeTimer.Stop();
                _resizeTimer.Start();
            }
        };

        FormClosing += (_, e) =>
        {
            SaveWindowPlacement();
            if (!_allowClose && e.CloseReason == CloseReason.UserClosing)
            {
                e.Cancel = true;
                Hide();
            }
        };

        ApplyWindowSettings(registerHotkeys: false);
        RebuildTabBar();
        PositionNewMessagesButton();
    }

    protected override void OnHandleCreated(EventArgs e)
    {
        base.OnHandleCreated(e);
        RegisterHotkeys(showErrors: false);
        ApplyClickThrough();
    }

    protected override void OnHandleDestroyed(EventArgs e)
    {
        UnregisterHotkeys();
        base.OnHandleDestroyed(e);
    }

    protected override void WndProc(ref Message m)
    {
        if (m.Msg == ChatNativeMethods.WmHotKey)
        {
            var id = m.WParam.ToInt32();
            if (id == ClickThroughHotkeyId)
            {
                ToggleClickThrough();
                return;
            }
            if (id == CollapseHotkeyId)
            {
                ToggleCollapsed();
                return;
            }
        }

        if (m.Msg == ChatNativeMethods.WmNcHitTest && !_collapsed && !_settings.Chat.ClickThrough)
        {
            base.WndProc(ref m);
            if ((int)m.Result == 1)
            {
                var value = m.LParam.ToInt64();
                var screenPoint = new Point(unchecked((short)(value & 0xFFFF)), unchecked((short)((value >> 16) & 0xFFFF)));
                var p = PointToClient(screenPoint);
                var left = p.X <= ResizeGrip;
                var right = p.X >= ClientSize.Width - ResizeGrip;
                var top = p.Y <= ResizeGrip;
                var bottom = p.Y >= ClientSize.Height - ResizeGrip;

                if (left && top) m.Result = new IntPtr(ChatNativeMethods.HtTopLeft);
                else if (right && top) m.Result = new IntPtr(ChatNativeMethods.HtTopRight);
                else if (left && bottom) m.Result = new IntPtr(ChatNativeMethods.HtBottomLeft);
                else if (right && bottom) m.Result = new IntPtr(ChatNativeMethods.HtBottomRight);
                else if (left) m.Result = new IntPtr(ChatNativeMethods.HtLeft);
                else if (right) m.Result = new IntPtr(ChatNativeMethods.HtRight);
                else if (top) m.Result = new IntPtr(ChatNativeMethods.HtTop);
                else if (bottom) m.Result = new IntPtr(ChatNativeMethods.HtBottom);
            }
            return;
        }

        base.WndProc(ref m);
    }

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

        var wasFollowing = _followLatest && IsNearBottom();
        _messages.Items.Add(CreateDisplayItem(message));

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

    private Button MakeToolbarButton(string text, int width, string tooltip)
    {
        var button = new Button
        {
            Text = text,
            Width = width,
            FlatStyle = FlatStyle.Flat,
            ForeColor = Color.WhiteSmoke,
            BackColor = Color.FromArgb(45, 49, 55),
            TabStop = false,
            Margin = Padding.Empty
        };
        button.FlatAppearance.BorderSize = 0;
        _toolTip.SetToolTip(button, tooltip);
        return button;
    }

    private static ChatMessageEvent SanitizeForDisplay(ChatMessageEvent message)
    {
        var name = (message.SenderName ?? string.Empty)
            .Replace('\r', ' ')
            .Replace('\n', ' ')
            .Replace('\0', ' ')
            .Trim();
        if (name.Length > MaxSenderNameLength)
            name = name[..MaxSenderNameLength];

        var text = (message.Text ?? string.Empty).Replace("\0", string.Empty, StringComparison.Ordinal);
        if (text.Length > MaxDisplayedMessageLength)
            text = text[..MaxDisplayedMessageLength] + "…";

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
        if (message.SenderId == 0) return;
        if (_settings.Chat.BlockedUsers.Any(x => x.Id == message.SenderId)) return;

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

            var add = new Button
            {
                Text = "+",
                Width = 34,
                Height = 27,
                Margin = new Padding(2, 0, 0, 0),
                FlatStyle = FlatStyle.Flat,
                BackColor = Color.FromArgb(55, 59, 66),
                ForeColor = Color.White,
                TabStop = false,
                AccessibleName = "Add chat tab"
            };
            add.FlatAppearance.BorderSize = 0;
            add.Click += (_, _) => AddTab();
            _tabBar.Controls.Add(add);
        }
        finally
        {
            _tabBar.ResumeLayout();
        }
    }

    private Button MakeTabButton(ChatTabSettings tab)
    {
        var selected = tab.Id == _settings.Chat.LastSelectedTabId;
        var button = new Button
        {
            Text = tab.Name,
            AutoSize = true,
            Height = 27,
            Margin = new Padding(0, 0, 3, 0),
            Padding = new Padding(7, 0, 7, 0),
            FlatStyle = FlatStyle.Flat,
            BackColor = selected ? Color.FromArgb(79, 86, 98) : Color.FromArgb(48, 52, 59),
            ForeColor = Color.WhiteSmoke,
            TabStop = false,
            Tag = tab,
            AccessibleName = $"Chat tab {tab.Name}"
        };
        button.FlatAppearance.BorderSize = 0;
        button.Click += (_, _) => SelectTab(tab.Id);

        var menu = new ContextMenuStrip();
        var edit = new ToolStripMenuItem("Edit Tab...");
        edit.Click += (_, _) => EditTab(tab);
        var delete = new ToolStripMenuItem("Delete Tab...");
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
    }

    private void AddTab()
    {
        var tab = new ChatTabSettings
        {
            Name = "New Tab",
            MinLevel = 1,
            Channels = [(int)ChatChannel.World]
        };
        using var editor = new ChatTabEditorForm(tab, isNew: true);
        if (editor.ShowDialog(this) != DialogResult.OK) return;

        _settings.Chat.Tabs.Add(tab);
        _settings.Chat.LastSelectedTabId = tab.Id;
        _settingsStore.Save(_settings);
        RebuildTabBar();
        RebuildVisibleMessages(keepScroll: false);
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
    }

    private void DeleteTab(ChatTabSettings tab)
    {
        if (_settings.Chat.Tabs.Count <= 1)
        {
            MessageBox.Show(this, "At least one chat tab is required.", "BPSR Chat", MessageBoxButtons.OK, MessageBoxIcon.Information);
            return;
        }

        if (MessageBox.Show(
                this,
                $"Delete chat tab '{tab.Name}'?",
                "BPSR Chat",
                MessageBoxButtons.YesNo,
                MessageBoxIcon.Question) != DialogResult.Yes)
            return;

        _settings.Chat.Tabs.Remove(tab);
        if (_settings.Chat.LastSelectedTabId == tab.Id)
            _settings.Chat.LastSelectedTabId = _settings.Chat.Tabs[0].Id;
        _settingsStore.Save(_settings);
        RebuildTabBar();
        RebuildVisibleMessages(keepScroll: false);
    }

    private void ApplyWindowSettings(bool registerHotkeys)
    {
        TopMost = _settings.Chat.TopMost;
        Opacity = Math.Clamp(_settings.Chat.WindowOpacity, 25, 100) / 100d;

        var body = ChatColorUtil.Blend(Color.FromArgb(20, 22, 26), Color.FromArgb(52, 56, 64), _settings.Chat.BackgroundOpacity);
        var toolbar = ChatColorUtil.Blend(Color.FromArgb(28, 31, 36), Color.FromArgb(68, 74, 84), _settings.Chat.ToolbarOpacity);
        BackColor = Color.FromArgb(55, 60, 68);
        _messages.BackColor = body;
        _topPanel.BackColor = toolbar;
        _tabBar.BackColor = toolbar;
        _collapsedHandle.BackColor = toolbar;
        foreach (var button in new[] { _gearButton, _collapseButton, _hideButton, _dragGrip })
            button.BackColor = ChatColorUtil.Blend(toolbar, Color.White, 8);

        CreateFonts();
        _messages.Font = _messageFont!;

        UpdateCollapseButtonGlyph();
        PositionNewMessagesButton();
        _messages.Invalidate();

        if (registerHotkeys && IsHandleCreated)
            RegisterHotkeys(showErrors: true);
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
    }

    private void RemoveOverflowHistoryFromView()
    {
        var cap = Math.Clamp(_settings.Chat.MaxHistory, 10, 500);
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
    }

    private ChatTabSettings SelectedTab =>
        _settings.Chat.Tabs.FirstOrDefault(t => t.Id == _settings.Chat.LastSelectedTabId)
        ?? _settings.Chat.Tabs[0];

    private bool IsVisibleForTab(ChatMessageEvent message, ChatTabSettings tab)
    {
        if (!tab.Channels.Contains((int)message.Channel)) return false;
        if (message.SenderLevel > 0 && message.SenderLevel < tab.MinLevel) return false;
        if (_settings.Chat.BlockedUsers.Any(x => x.Id != 0 && x.Id == message.SenderId)) return false;
        if (_settings.Chat.HideStickers && message.Kind == ChatMessageKind.Sticker) return false;

        if (message.Kind is ChatMessageKind.Text or ChatMessageKind.TextNotice)
        {
            if (!string.IsNullOrWhiteSpace(tab.ShowIfMatches) &&
                !ChatFilterExpression.IsMatch(message.Text, tab.ShowIfMatches))
                return false;

            if (!string.IsNullOrWhiteSpace(tab.HideIfMatches) &&
                ChatFilterExpression.IsMatch(message.Text, tab.HideIfMatches))
                return false;
        }

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
            {
                if (IsVisibleForTab(message, tab))
                    _messages.Items.Add(CreateDisplayItem(message));
            }
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
    }

    private ChatDisplayItem CreateDisplayItem(ChatMessageEvent message)
    {
        var highlight = false;
        if (!string.IsNullOrWhiteSpace(_settings.Chat.HighlightIfMatches) &&
            message.Kind is ChatMessageKind.Text or ChatMessageKind.TextNotice)
        {
            var searchable = DisplaySenderName(message) + "\n" + message.Text;
            highlight = ChatFilterExpression.IsMatch(searchable, _settings.Chat.HighlightIfMatches);
        }

        return new ChatDisplayItem(
            message,
            highlight,
            _settings.Chat.PrivateHighlightEnabled && message.Channel == ChatChannel.Private);
    }

    private void MessagesMeasureItem(object? sender, MeasureItemEventArgs e)
    {
        if (e.Index < 0 || e.Index >= _messages.Items.Count || _messages.Items[e.Index] is not ChatDisplayItem item ||
            _messageFont is null || _messageBoldFont is null || _senderFont is null || _metaFont is null)
        {
            e.ItemHeight = Math.Max(22, Font.Height + 8);
            return;
        }

        var usableWidth = Math.Max(120, _messages.ClientSize.Width - 22);
        var lineHeight = Math.Max(_messageFont.Height, _senderFont.Height) + 3;
        if (_settings.Chat.CompactMode)
        {
            var prefix = CompactPrefix(item.Message);
            var prefixWidth = TextRenderer.MeasureText(e.Graphics, prefix, _metaFont, Size.Empty, TextFormatFlags.NoPadding).Width +
                              TextRenderer.MeasureText(e.Graphics, DisplaySenderName(item.Message) + " ", _senderFont, Size.Empty, TextFormatFlags.NoPadding).Width;
            var messageWidth = Math.Max(80, usableWidth - prefixWidth);
            var size = TextRenderer.MeasureText(
                e.Graphics,
                item.Message.Text,
                _settings.Chat.BoldMessageText ? _messageBoldFont : _messageFont,
                new Size(messageWidth, int.MaxValue),
                TextFormatFlags.WordBreak | TextFormatFlags.NoPadding);
            e.ItemHeight = Math.Max(lineHeight, size.Height) + 7;
        }
        else
        {
            var size = TextRenderer.MeasureText(
                e.Graphics,
                item.Message.Text,
                _settings.Chat.BoldMessageText ? _messageBoldFont : _messageFont,
                new Size(usableWidth, int.MaxValue),
                TextFormatFlags.WordBreak | TextFormatFlags.NoPadding);
            e.ItemHeight = lineHeight + size.Height + 10;
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
            back = ChatColorUtil.Blend(Color.FromArgb(52, 58, 68), baseBack, 18);
        if (item.IsPrivateHighlighted)
            back = ChatColorUtil.Blend(ChatColorUtil.Parse(_settings.Chat.PrivateHighlightColor, Color.MediumPurple), back, 42);
        else if (item.IsHighlighted)
            back = ChatColorUtil.Blend(ChatColorUtil.Parse(_settings.Chat.HighlightColor, Color.DarkGoldenrod), back, 38);

        using (var brush = new SolidBrush(back))
            e.Graphics.FillRectangle(brush, e.Bounds);

        var channelColor = GetChannelColor(item.Message.Channel);
        if (_settings.Chat.ShowColorBand)
        {
            using var band = new SolidBrush(channelColor);
            e.Graphics.FillRectangle(band, new Rectangle(e.Bounds.Left, e.Bounds.Top, 3, e.Bounds.Height));
        }

        var x = e.Bounds.Left + (_settings.Chat.ShowColorBand ? 8 : 5);
        var y = e.Bounds.Top + 3;
        var right = e.Bounds.Right - 8;
        var textColor = ChatColorUtil.Blend(Color.Gainsboro, back, _settings.Chat.TextOpacity);
        var senderColor = ChatColorUtil.Blend(Color.FromArgb(102, 179, 255), back, _settings.Chat.TextOpacity);
        var metaColor = ChatColorUtil.Blend(Color.FromArgb(155, 166, 190), back, _settings.Chat.TextOpacity);
        var messageFont = _settings.Chat.BoldMessageText ? _messageBoldFont : _messageFont;

        if (_settings.Chat.CompactMode)
        {
            var channel = $"[{GetChannelName(item.Message.Channel)}] ";
            x = DrawInline(e.Graphics, channel, _metaFont, channelColor, back, x, y);
            if (_settings.Chat.ShowTime)
                x = DrawInline(e.Graphics, GetTimeText(item.Message.Timestamp) + " ", _metaFont, metaColor, back, x, y);
            x = DrawInline(e.Graphics, $"[{DisplaySenderName(item.Message)}] ", _senderFont, senderColor, back, x, y);
            var rect = new Rectangle(x, y, Math.Max(20, right - x), Math.Max(18, e.Bounds.Bottom - y - 3));
            DrawWrapped(e.Graphics, item.Message.Text, messageFont, textColor, back, rect);
        }
        else
        {
            x = DrawInline(e.Graphics, $"[{GetChannelName(item.Message.Channel)}] ", _metaFont, channelColor, back, x, y);
            x = DrawInline(e.Graphics, $"[{DisplaySenderName(item.Message)}]", _senderFont, senderColor, back, x, y);
            if (_settings.Chat.ShowTime)
                _ = DrawInline(e.Graphics, "  " + GetTimeText(item.Message.Timestamp), _metaFont, metaColor, back, x, y);

            var messageY = y + Math.Max(_senderFont.Height, _metaFont.Height) + 3;
            var rect = new Rectangle(e.Bounds.Left + (_settings.Chat.ShowColorBand ? 8 : 5), messageY,
                Math.Max(20, right - e.Bounds.Left - 5), Math.Max(18, e.Bounds.Bottom - messageY - 3));
            DrawWrapped(e.Graphics, item.Message.Text, messageFont, textColor, back, rect);
        }

        if (_settings.Chat.ShowSeparators)
        {
            using var pen = new Pen(ChatColorUtil.Blend(Color.White, back, 12));
            e.Graphics.DrawLine(pen, e.Bounds.Left + 6, e.Bounds.Bottom - 1, e.Bounds.Right - 6, e.Bounds.Bottom - 1);
        }
    }

    private int DrawInline(Graphics graphics, string text, Font font, Color color, Color background, int x, int y)
    {
        var size = TextRenderer.MeasureText(graphics, text, font, Size.Empty, TextFormatFlags.NoPadding);
        var rect = new Rectangle(x, y, size.Width + 1, size.Height + 2);
        DrawText(graphics, text, font, color, background, rect, TextFormatFlags.NoPadding | TextFormatFlags.SingleLine);
        return x + size.Width;
    }

    private void DrawWrapped(Graphics graphics, string text, Font font, Color color, Color background, Rectangle rect)
    {
        DrawText(graphics, text, font, color, background, rect,
            TextFormatFlags.WordBreak | TextFormatFlags.NoPadding | TextFormatFlags.TextBoxControl);
    }

    private void DrawText(Graphics graphics, string text, Font font, Color color, Color background, Rectangle rect, TextFormatFlags flags)
    {
        if (_settings.Chat.TextShadow)
        {
            var shadow = ChatColorUtil.Blend(Color.Black, background, 58);
            var shadowRect = new Rectangle(rect.X + 1, rect.Y + 1, rect.Width, rect.Height);
            TextRenderer.DrawText(graphics, text, font, shadowRect, shadow, flags);
        }
        TextRenderer.DrawText(graphics, text, font, rect, color, flags);
    }

    private string CompactPrefix(ChatMessageEvent message)
    {
        var value = $"[{GetChannelName(message.Channel)}] ";
        if (_settings.Chat.ShowTime) value += GetTimeText(message.Timestamp) + " ";
        return value;
    }

    private bool IsNearBottom()
    {
        if (_messages.Items.Count == 0) return true;
        var index = _messages.IndexFromPoint(new Point(Math.Max(1, _messages.ClientSize.Width / 2), Math.Max(1, _messages.ClientSize.Height - 3)));
        if (index == ListBox.NoMatches)
            return _messages.TopIndex >= _messages.Items.Count - 1;
        return index >= _messages.Items.Count - 1;
    }

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
        if (_messages.Items.Count > 0)
            _messages.TopIndex = _messages.Items.Count - 1;
        _unseenMessages = 0;
        UpdateNewMessagesButton();
    }

    private void UpdateNewMessagesButton()
    {
        _newMessagesButton.Visible = !_collapsed && !_followLatest && _unseenMessages > 0;
        _newMessagesButton.Text = _unseenMessages <= 1
            ? "↓ 1 new message"
            : $"↓ {_unseenMessages} new messages";
        if (_newMessagesButton.Visible) _newMessagesButton.BringToFront();
    }

    private void PositionNewMessagesButton()
    {
        _newMessagesButton.Location = new Point(
            Math.Max(4, ClientSize.Width - _newMessagesButton.Width - 16),
            Math.Max(_topPanel.Bottom + 4, ClientSize.Height - _newMessagesButton.Height - 14));
    }

    private void HandleMessageNotification(ChatMessageEvent message)
    {
        if (_settings.Chat.BlockedUsers.Any(x => x.Id != 0 && x.Id == message.SenderId)) return;
        if (_settings.Chat.HideStickers && message.Kind == ChatMessageKind.Sticker) return;

        var isPrivate = message.Channel == ChatChannel.Private;
        if (isPrivate && _settings.Chat.PrivateSoundEnabled)
        {
            PlayChatSound(_settings.Chat.PrivateSoundPath);
            return;
        }

        if (!_settings.Chat.HighlightSoundEnabled || string.IsNullOrWhiteSpace(_settings.Chat.HighlightIfMatches)) return;
        if (message.Kind is not (ChatMessageKind.Text or ChatMessageKind.TextNotice)) return;
        var searchable = DisplaySenderName(message) + "\n" + message.Text;
        if (ChatFilterExpression.IsMatch(searchable, _settings.Chat.HighlightIfMatches))
            PlayChatSound(_settings.Chat.HighlightSoundPath);
    }

    private void PlayChatSound(string configuredPath)
    {
        if ((DateTime.UtcNow - _lastSoundUtc).TotalMilliseconds < 150) return;
        _lastSoundUtc = DateTime.UtcNow;

        var path = !string.IsNullOrWhiteSpace(configuredPath) && File.Exists(configuredPath)
            ? configuredPath
            : _defaultSoundPath;
        try
        {
            if (File.Exists(path))
            {
                _chatSoundPlayer.SoundLocation = path;
                _chatSoundPlayer.Play();
            }
            else
            {
                SystemSounds.Asterisk.Play();
            }
        }
        catch (Exception ex)
        {
            AppLog.Write("chat: notification sound failed " + ex.Message);
            try { SystemSounds.Asterisk.Play(); } catch { }
        }
    }

    private void RegisterHotkeys(bool showErrors)
    {
        if (!IsHandleCreated) return;
        UnregisterHotkeys();

        var problems = new List<string>();
        if (ChatHotkey.TryParse(_settings.Chat.ClickThroughHotkey, out var clickGesture, out var clickError))
        {
            _clickThroughRegistered = ChatNativeMethods.RegisterHotKey(Handle, ClickThroughHotkeyId, clickGesture.NativeModifiers, (uint)clickGesture.Key);
            if (!_clickThroughRegistered) problems.Add("Click-through hotkey is already in use by another app.");
        }
        else
        {
            problems.Add("Click-through hotkey: " + clickError);
        }

        if (ChatHotkey.TryParse(_settings.Chat.CollapseHotkey, out var collapseGesture, out var collapseError))
        {
            if (clickGesture.Equals(collapseGesture))
            {
                problems.Add("Click-through and collapse hotkeys cannot be the same.");
            }
            else
            {
                _collapseRegistered = ChatNativeMethods.RegisterHotKey(Handle, CollapseHotkeyId, collapseGesture.NativeModifiers, (uint)collapseGesture.Key);
                if (!_collapseRegistered) problems.Add("Collapse hotkey is already in use by another app.");
            }
        }
        else
        {
            problems.Add("Collapse hotkey: " + collapseError);
        }

        if (!_clickThroughRegistered && _settings.Chat.ClickThrough)
        {
            _settings.Chat.ClickThrough = false;
            _settingsStore.Save(_settings);
            ApplyClickThrough();
            problems.Add("Click-through was turned OFF so the window cannot become mouse-locked without a working hotkey.");
        }

        foreach (var problem in problems) AppLog.Write("chat: hotkey " + problem);
        if (showErrors && problems.Count > 0)
        {
            MessageBox.Show(this,
                string.Join(Environment.NewLine, problems) + Environment.NewLine + Environment.NewLine +
                "Change the hotkeys in Chat Settings > Interaction.",
                "BPSR Chat - Hotkey",
                MessageBoxButtons.OK,
                MessageBoxIcon.Warning);
        }
    }

    private void UnregisterHotkeys()
    {
        if (!IsHandleCreated) return;
        if (_clickThroughRegistered)
        {
            _ = ChatNativeMethods.UnregisterHotKey(Handle, ClickThroughHotkeyId);
            _clickThroughRegistered = false;
        }
        if (_collapseRegistered)
        {
            _ = ChatNativeMethods.UnregisterHotKey(Handle, CollapseHotkeyId);
            _collapseRegistered = false;
        }
    }

    private void ToggleClickThrough()
    {
        _settings.Chat.ClickThrough = !_settings.Chat.ClickThrough;
        _settingsStore.Save(_settings);
        ApplyClickThrough();
        AppLog.Write("chat: click-through=" + _settings.Chat.ClickThrough);
    }

    private void ApplyClickThrough()
    {
        if (!IsHandleCreated) return;
        if (!ChatNativeMethods.SetClickThrough(Handle, _settings.Chat.ClickThrough))
            AppLog.Write("chat: failed to change click-through window style");
    }

    private void ToggleCollapsed()
    {
        if (_collapsed) ExpandFromEdge();
        else CollapseToEdge();
    }

    private void CollapseToEdge()
    {
        if (_collapsed) return;
        _expandedBounds = Bounds;
        SaveWindowPlacement();

        var screen = Screen.FromRectangle(Bounds).WorkingArea;
        var side = _settings.Chat.CollapseSide;
        _topPanel.Visible = false;
        _messages.Visible = false;
        _newMessagesButton.Visible = false;
        _collapsedHandle.Visible = true;
        MinimumSize = Size.Empty;

        if (side == "Left" || side == "Right")
        {
            var height = Math.Min(_expandedBounds.Height, screen.Height);
            var y = Math.Clamp(_expandedBounds.Top, screen.Top, Math.Max(screen.Top, screen.Bottom - height));
            var x = side == "Left" ? screen.Left : screen.Right - CollapsedThickness;
            Bounds = new Rectangle(x, y, CollapsedThickness, height);
        }
        else
        {
            var width = Math.Min(_expandedBounds.Width, screen.Width);
            var x = Math.Clamp(_expandedBounds.Left, screen.Left, Math.Max(screen.Left, screen.Right - width));
            var y = side == "Top" ? screen.Top : screen.Bottom - CollapsedThickness;
            Bounds = new Rectangle(x, y, width, CollapsedThickness);
        }

        _collapsed = true;
        _collapsedHandle.Text = side switch
        {
            "Left" => "▶",
            "Top" => "▼",
            "Bottom" => "▲",
            _ => "◀"
        };
        AppLog.Write("chat: collapsed side=" + side);
    }

    private void ExpandFromEdge()
    {
        if (!_collapsed) return;
        _collapsed = false;
        _collapsedHandle.Visible = false;
        MinimumSize = new Size(360, 180);
        Bounds = _expandedBounds;
        _topPanel.Visible = true;
        _messages.Visible = true;
        UpdateNewMessagesButton();
        RebuildVisibleMessages(keepScroll: true);
        AppLog.Write("chat: expanded");
    }

    private void UpdateCollapseButtonGlyph()
    {
        _collapseButton.Text = _settings.Chat.CollapseSide switch
        {
            "Left" => "◀",
            "Top" => "▲",
            "Bottom" => "▼",
            _ => "▶"
        };
    }

    private string GetTimeText(DateTime timestamp)
    {
        if (!_settings.Chat.ShowTimeAsAgo)
            return timestamp.ToString("HH:mm");

        var age = DateTime.Now - timestamp;
        if (age.TotalSeconds < 0) age = TimeSpan.Zero;
        if (age.TotalSeconds < 60) return $"{Math.Max(0, (int)age.TotalSeconds)}s";
        if (age.TotalMinutes < 60) return $"{(int)age.TotalMinutes}m";
        if (age.TotalHours < 24) return $"{(int)age.TotalHours}h";
        return timestamp.ToString("MM-dd HH:mm");
    }

    private static string DisplaySenderName(ChatMessageEvent message)
    {
        if (!string.IsNullOrWhiteSpace(message.SenderName)) return message.SenderName;
        return message.SenderId != 0 ? message.SenderId.ToString() : "System";
    }

    private static string GetChannelName(ChatChannel channel) => channel switch
    {
        ChatChannel.Null => "Other",
        ChatChannel.World => "World",
        ChatChannel.Local => "Local",
        ChatChannel.Team => "Team",
        ChatChannel.Union => "Guild",
        ChatChannel.Private => "Private",
        ChatChannel.Group => "Group",
        ChatChannel.TopNotice => "Notice",
        ChatChannel.Play => "Play",
        ChatChannel.Newbie => "Newbie",
        ChatChannel.System => "System",
        _ => "Other"
    };

    private Color GetChannelColor(ChatChannel channel)
    {
        var defaults = ChatOverlaySettings.CreateDefaultChannelColors();
        var key = (int)channel;
        var fallback = ChatColorUtil.Parse(defaults.TryGetValue(key, out var defaultHex) ? defaultHex : "#D3D3D3", Color.LightGray);
        return _settings.Chat.ChannelColors.TryGetValue(key, out var value)
            ? ChatColorUtil.Parse(value, fallback)
            : fallback;
    }

    private void RestoreWindowPlacement()
    {
        if (_settings.Chat.WindowX == int.MinValue || _settings.Chat.WindowY == int.MinValue)
        {
            PlaceAtPrimaryScreenBottomRight();
            return;
        }

        var desired = new Rectangle(
            _settings.Chat.WindowX,
            _settings.Chat.WindowY,
            _settings.Chat.WindowWidth,
            _settings.Chat.WindowHeight);
        var visible = Screen.AllScreens.Any(s => s.WorkingArea.IntersectsWith(desired));
        if (visible)
            Bounds = desired;
        else
            PlaceAtPrimaryScreenBottomRight();
    }

    private void PlaceAtPrimaryScreenBottomRight()
    {
        var area = Screen.PrimaryScreen?.WorkingArea ?? new Rectangle(0, 0, 1920, 1080);
        Location = new Point(
            Math.Max(area.Left, area.Right - Width - 40),
            Math.Max(area.Top, area.Bottom - Height - 80));
    }

    private void SaveWindowPlacement()
    {
        var bounds = _collapsed ? _expandedBounds : Bounds;
        if (WindowState == FormWindowState.Normal && bounds.Width >= 360 && bounds.Height >= 180)
        {
            _settings.Chat.WindowX = bounds.Left;
            _settings.Chat.WindowY = bounds.Top;
            _settings.Chat.WindowWidth = bounds.Width;
            _settings.Chat.WindowHeight = bounds.Height;
        }
        _settingsStore.Save(_settings);
    }

    protected override void Dispose(bool disposing)
    {
        if (disposing && !_disposedResources)
        {
            _disposedResources = true;
            try { _relativeTimer.Stop(); } catch { }
            try { _resizeTimer.Stop(); } catch { }
            _messageFont?.Dispose();
            _messageBoldFont?.Dispose();
            _senderFont?.Dispose();
            _metaFont?.Dispose();
            _chatSoundPlayer.Dispose();
            _toolTip.Dispose();
        }
        base.Dispose(disposing);
    }

    private sealed record ChatDisplayItem(
        ChatMessageEvent Message,
        bool IsHighlighted,
        bool IsPrivateHighlighted);
}
