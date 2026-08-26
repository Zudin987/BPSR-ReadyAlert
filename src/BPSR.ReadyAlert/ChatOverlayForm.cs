using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed class ChatOverlayForm : Form
{
    private const int MaxSenderNameLength = 128;
    private const int MaxDisplayedMessageLength = 8 * 1024;

    private readonly AppSettings _settings;
    private readonly SettingsStore _settingsStore;
    private readonly List<ChatMessageEvent> _history = [];
    private readonly Dictionary<int, ChatMessageEvent> _lineMessageMap = [];
    private readonly FlowLayoutPanel _tabBar;
    private readonly RichTextBox _messages;
    private readonly Button _gearButton;
    private readonly Panel _topPanel;
    private readonly System.Windows.Forms.Timer _relativeTimer;
    private ChatMessageEvent? _contextMessage;
    private bool _allowClose;
    private int _trimsSinceFullRender;

    internal ChatOverlayForm(
        AppSettings settings,
        SettingsStore settingsStore,
        string iconPath)
    {
        _settings = settings;
        _settingsStore = settingsStore;
        _settings.Chat.Normalize();

        AutoScaleMode = AutoScaleMode.Dpi;
        AutoScaleDimensions = new SizeF(96F, 96F);
        Text = "BPSR Chat";
        ShowInTaskbar = false;
        StartPosition = FormStartPosition.Manual;
        MinimumSize = new Size(420, 260);
        Size = new Size(_settings.Chat.WindowWidth, _settings.Chat.WindowHeight);
        BackColor = Color.FromArgb(30, 30, 30);
        ForeColor = Color.Gainsboro;
        Font = new Font("Segoe UI", 9F, FontStyle.Regular, GraphicsUnit.Point);
        FormBorderStyle = FormBorderStyle.SizableToolWindow;

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

        _topPanel = new Panel
        {
            Dock = DockStyle.Top,
            Height = 34,
            BackColor = Color.FromArgb(36, 36, 36)
        };

        _gearButton = new Button
        {
            Text = "⚙",
            Dock = DockStyle.Right,
            Width = 38,
            FlatStyle = FlatStyle.Flat,
            ForeColor = Color.WhiteSmoke,
            BackColor = Color.FromArgb(52, 52, 52),
            TabStop = false,
            AccessibleName = "Chat settings"
        };
        _gearButton.FlatAppearance.BorderSize = 0;
        _gearButton.Click += (_, _) => OpenSettingsDialog();

        _tabBar = new FlowLayoutPanel
        {
            Dock = DockStyle.Fill,
            FlowDirection = FlowDirection.LeftToRight,
            WrapContents = false,
            AutoScroll = true,
            Padding = new Padding(3, 3, 0, 0),
            BackColor = Color.FromArgb(36, 36, 36)
        };

        _topPanel.Controls.Add(_tabBar);
        _topPanel.Controls.Add(_gearButton);

        _messages = new RichTextBox
        {
            Dock = DockStyle.Fill,
            BorderStyle = BorderStyle.None,
            BackColor = Color.FromArgb(24, 24, 24),
            ForeColor = Color.Gainsboro,
            ReadOnly = true,
            DetectUrls = false,
            HideSelection = false,
            ScrollBars = RichTextBoxScrollBars.Vertical,
            WordWrap = true,
            Font = new Font("Segoe UI", 9F, FontStyle.Regular, GraphicsUnit.Point),
            AccessibleName = "BPSR chat messages"
        };

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
            if (_contextMessage is { } current)
            {
                copyName.Enabled = !string.IsNullOrEmpty(current.SenderName);
                copyUid.Enabled = current.SenderId != 0;
                block.Enabled = current.SenderId != 0;
                return;
            }

            copyName.Enabled = false;
            copyUid.Enabled = false;
            block.Enabled = false;
            e.Cancel = true;
        };
        _messages.ContextMenuStrip = messageMenu;
        _messages.MouseDown += MessagesMouseDown;

        Controls.Add(_messages);
        Controls.Add(_topPanel);

        // Relative times do not need a 1 Hz full RichTextBox rebuild. A 15-second
        // refresh keeps the display useful while avoiding needless CPU/scroll churn.
        _relativeTimer = new System.Windows.Forms.Timer { Interval = 15_000 };
        _relativeTimer.Tick += (_, _) =>
        {
            if (Visible && !_messages.Focused && _settings.Chat.ShowTime && _settings.Chat.ShowTimeAsAgo)
                RenderMessages(keepScroll: true);
        };
        _relativeTimer.Start();

        FormClosing += (_, e) =>
        {
            SaveWindowPlacement();
            if (!_allowClose && e.CloseReason == CloseReason.UserClosing)
            {
                // X means "hide chat". The tray Chat Overlay check box is the
                // explicit control for turning packet processing fully off.
                e.Cancel = true;
                Hide();
            }
        };

        ApplyWindowSettings();
        RebuildTabBar();
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
        var trimmed = TrimHistory();

        if (!Visible || _settings.Chat.Tabs.Count == 0)
            return;

        if (trimmed)
        {
            _trimsSinceFullRender++;
            if (_trimsSinceFullRender >= 20)
            {
                RenderMessages(keepScroll: true);
                return;
            }
        }

        if (!IsVisibleForTab(message, SelectedTab))
            return;

        var wasAtBottom = IsScrolledNearBottom();
        var oldTopChar = GetFirstVisibleCharIndex();
        var oldSelection = _messages.SelectionStart;
        var oldSelectionLength = _messages.SelectionLength;

        AppendMessage(message);

        if (wasAtBottom)
        {
            ScrollToEnd();
        }
        else
        {
            RestoreScrollToChar(oldTopChar);
            RestoreSelection(oldSelection, oldSelectionLength);
        }
    }

    internal void ShowOverlay()
    {
        if (InvokeRequired)
        {
            BeginInvoke(new Action(ShowOverlay));
            return;
        }

        ApplyWindowSettings();
        if (!Visible) Show();
        else BringToFront();
        RenderMessages(keepScroll: false);
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
        using var dialog = new ChatGeneralSettingsForm(_settings.Chat);
        if (dialog.ShowDialog(this) != DialogResult.OK) return;

        _settings.Chat.Normalize();
        TrimHistory();
        ApplyWindowSettings();
        _settingsStore.Save(_settings);
        RebuildTabBar();
        RenderMessages(keepScroll: true);
    }

    internal void Shutdown()
    {
        if (IsDisposed) return;
        SaveWindowPlacement();
        _relativeTimer.Stop();
        _allowClose = true;
        Close();
        Dispose();
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

        var charIndex = _messages.GetCharIndexFromPosition(e.Location);
        var line = _messages.GetLineFromCharIndex(charIndex);
        _contextMessage = _lineMessageMap.TryGetValue(line, out var message) ? message : null;
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
        RenderMessages(keepScroll: true);
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
                BackColor = Color.FromArgb(55, 55, 55),
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
            BackColor = selected ? Color.FromArgb(85, 85, 85) : Color.FromArgb(55, 55, 55),
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
        RenderMessages(keepScroll: false);
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
        RenderMessages(keepScroll: false);
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
        RenderMessages(keepScroll: true);
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
        RenderMessages(keepScroll: false);
    }

    private void ApplyWindowSettings()
    {
        TopMost = _settings.Chat.TopMost;
        Opacity = Math.Clamp(_settings.Chat.WindowOpacity, 25, 100) / 100d;

        // WinForms child controls cannot independently alpha-blend with the desktop
        // without switching the whole overlay to a custom layered-window renderer.
        // Keep text crisp and use BackgroundOpacity as the surface darkness control.
        var strength = Math.Clamp(_settings.Chat.BackgroundOpacity, 10, 100) / 100d;
        var messageShade = (int)Math.Round(66 - (42 * strength));
        var chromeShade = Math.Min(78, messageShade + 12);
        var buttonShade = Math.Min(92, chromeShade + 16);
        BackColor = Color.FromArgb(chromeShade, chromeShade, chromeShade);
        _topPanel.BackColor = Color.FromArgb(chromeShade, chromeShade, chromeShade);
        _tabBar.BackColor = _topPanel.BackColor;
        _messages.BackColor = Color.FromArgb(messageShade, messageShade, messageShade);
        _gearButton.BackColor = Color.FromArgb(buttonShade, buttonShade, buttonShade);
    }

    private bool TrimHistory()
    {
        var cap = Math.Clamp(_settings.Chat.MaxHistory, 10, 500);
        if (_history.Count <= cap) return false;
        _history.RemoveRange(0, _history.Count - cap);
        return true;
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

    private void RenderMessages(bool keepScroll)
    {
        if (IsDisposed || _settings.Chat.Tabs.Count == 0) return;

        var oldSelection = _messages.SelectionStart;
        var oldSelectionLength = _messages.SelectionLength;
        var oldTopChar = GetFirstVisibleCharIndex();
        var wasAtBottom = IsScrolledNearBottom();

        _messages.SuspendLayout();
        try
        {
            _messages.Clear();
            _lineMessageMap.Clear();
            _trimsSinceFullRender = 0;
            var tab = SelectedTab;

            foreach (var message in _history)
            {
                if (IsVisibleForTab(message, tab))
                    AppendMessage(message);
            }

            if (!keepScroll || wasAtBottom)
            {
                ScrollToEnd();
            }
            else
            {
                RestoreScrollToChar(oldTopChar);
                RestoreSelection(oldSelection, oldSelectionLength);
            }
        }
        finally
        {
            _messages.ResumeLayout();
        }
    }

    private void AppendMessage(ChatMessageEvent message)
    {
        var startLine = _messages.GetLineFromCharIndex(_messages.TextLength);

        AppendColored($"[{GetChannelName(message.Channel)}] ", GetChannelColor(message.Channel));

        if (_settings.Chat.CompactMode)
        {
            if (_settings.Chat.ShowTime)
                AppendColored(GetTimeText(message.Timestamp) + " ", Color.FromArgb(155, 166, 190));
            AppendColored($"[{DisplaySenderName(message)}] ", Color.FromArgb(102, 179, 255));
            AppendColored(message.Text + Environment.NewLine, Color.Gainsboro);
        }
        else
        {
            AppendColored($"[{DisplaySenderName(message)}]", Color.FromArgb(102, 179, 255));
            if (_settings.Chat.ShowTime)
                AppendColored("  " + GetTimeText(message.Timestamp), Color.FromArgb(155, 166, 190));
            AppendColored(Environment.NewLine, Color.Gainsboro);
            AppendColored(message.Text + Environment.NewLine, Color.Gainsboro);
        }

        var endLine = _messages.GetLineFromCharIndex(Math.Max(0, _messages.TextLength - 1));
        for (var line = startLine; line <= endLine; line++)
            _lineMessageMap[line] = message;
    }

    private static string DisplaySenderName(ChatMessageEvent message)
    {
        if (!string.IsNullOrWhiteSpace(message.SenderName)) return message.SenderName;
        return message.SenderId != 0 ? message.SenderId.ToString() : "System";
    }

    private bool IsScrolledNearBottom()
    {
        if (_messages.TextLength == 0) return true;
        var point = new Point(
            Math.Max(0, _messages.ClientSize.Width - 4),
            Math.Max(0, _messages.ClientSize.Height - 4));
        var lastVisibleChar = _messages.GetCharIndexFromPosition(point);
        return lastVisibleChar >= Math.Max(0, _messages.TextLength - 3);
    }

    private int GetFirstVisibleCharIndex()
    {
        if (_messages.TextLength == 0) return 0;
        return _messages.GetCharIndexFromPosition(new Point(3, 3));
    }

    private void RestoreScrollToChar(int charIndex)
    {
        if (_messages.TextLength == 0) return;
        _messages.SelectionStart = Math.Clamp(charIndex, 0, _messages.TextLength);
        _messages.SelectionLength = 0;
        _messages.ScrollToCaret();
    }

    private void RestoreSelection(int start, int length)
    {
        var safeStart = Math.Clamp(start, 0, _messages.TextLength);
        var safeLength = Math.Clamp(length, 0, _messages.TextLength - safeStart);
        _messages.Select(safeStart, safeLength);
    }

    private void ScrollToEnd()
    {
        _messages.SelectionStart = _messages.TextLength;
        _messages.SelectionLength = 0;
        _messages.ScrollToCaret();
    }

    private void AppendColored(string text, Color color)
    {
        _messages.SelectionStart = _messages.TextLength;
        _messages.SelectionLength = 0;
        _messages.SelectionColor = color;
        _messages.AppendText(text);
        _messages.SelectionColor = _messages.ForeColor;
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

    private static string GetChannelName(ChatChannel channel) => channel switch
    {
        ChatChannel.World => "World",
        ChatChannel.Local => "Local",
        ChatChannel.Team => "Team",
        ChatChannel.Union => "Union",
        ChatChannel.Private => "Private",
        ChatChannel.Group => "Group",
        ChatChannel.TopNotice => "Notice",
        ChatChannel.Play => "Play",
        ChatChannel.Newbie => "Newbie",
        ChatChannel.System => "System",
        _ => "Other"
    };

    private static Color GetChannelColor(ChatChannel channel) => channel switch
    {
        ChatChannel.World => Color.FromArgb(99, 199, 255),
        ChatChannel.Local => Color.FromArgb(143, 237, 143),
        ChatChannel.Team => Color.FromArgb(255, 181, 194),
        ChatChannel.Union => Color.FromArgb(255, 214, 0),
        ChatChannel.Private => Color.FromArgb(255, 161, 255),
        ChatChannel.Group => Color.FromArgb(173, 216, 230),
        ChatChannel.TopNotice => Color.FromArgb(255, 140, 0),
        ChatChannel.System => Color.FromArgb(255, 99, 71),
        _ => Color.LightGray
    };

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
        if (WindowState == FormWindowState.Normal)
        {
            _settings.Chat.WindowX = Left;
            _settings.Chat.WindowY = Top;
            _settings.Chat.WindowWidth = Width;
            _settings.Chat.WindowHeight = Height;
        }
        _settingsStore.Save(_settings);
    }
}
