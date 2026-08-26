using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed class ChatOverlayForm : Form
{
    private readonly AppSettings _settings;
    private readonly SettingsStore _settingsStore;
    private readonly Action<bool> _enabledChanged;
    private readonly List<ChatMessageEvent> _history = [];
    private readonly Dictionary<int, ChatMessageEvent> _lineMessageMap = [];
    private readonly FlowLayoutPanel _tabBar;
    private readonly RichTextBox _messages;
    private readonly Button _gearButton;
    private readonly System.Windows.Forms.Timer _relativeTimer;
    private ChatMessageEvent? _contextMessage;
    private bool _allowClose;

    internal ChatOverlayForm(
        AppSettings settings,
        SettingsStore settingsStore,
        string iconPath,
        Action<bool> enabledChanged)
    {
        _settings = settings;
        _settingsStore = settingsStore;
        _enabledChanged = enabledChanged;
        _settings.Chat.Normalize();

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

        var topPanel = new Panel
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
            TabStop = false
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

        topPanel.Controls.Add(_tabBar);
        topPanel.Controls.Add(_gearButton);

        _messages = new RichTextBox
        {
            Dock = DockStyle.Fill,
            BorderStyle = BorderStyle.None,
            BackColor = Color.FromArgb(24, 24, 24),
            ForeColor = Color.Gainsboro,
            ReadOnly = true,
            DetectUrls = false,
            HideSelection = true,
            ScrollBars = RichTextBoxScrollBars.Vertical,
            WordWrap = true,
            Font = new Font("Segoe UI", 9F, FontStyle.Regular, GraphicsUnit.Point)
        };

        var messageMenu = new ContextMenuStrip();
        var copyName = new ToolStripMenuItem("Copy Name");
        copyName.Click += (_, _) =>
        {
            if (_contextMessage is { } msg) Clipboard.SetText(msg.SenderName);
        };
        var copyUid = new ToolStripMenuItem("Copy UID");
        copyUid.Click += (_, _) =>
        {
            if (_contextMessage is { } msg) Clipboard.SetText(msg.SenderId.ToString());
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
            var has = _contextMessage.HasValue;
            copyName.Enabled = has;
            copyUid.Enabled = has;
            block.Enabled = has;
            if (!has) e.Cancel = true;
        };
        _messages.ContextMenuStrip = messageMenu;
        _messages.MouseDown += MessagesMouseDown;

        Controls.Add(_messages);
        Controls.Add(topPanel);

        _relativeTimer = new System.Windows.Forms.Timer { Interval = 1000 };
        _relativeTimer.Tick += (_, _) =>
        {
            if (Visible && _settings.Chat.ShowTime && _settings.Chat.ShowTimeAsAgo)
                RenderMessages(keepScroll: true);
        };
        _relativeTimer.Start();

        FormClosing += (_, e) =>
        {
            SaveWindowPlacement();
            if (!_allowClose && e.CloseReason == CloseReason.UserClosing)
            {
                e.Cancel = true;
                Hide();
                _enabledChanged(false);
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

        _history.Add(message);
        TrimHistory();
        if (Visible)
            RenderMessages(keepScroll: false);
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
            {
                var button = MakeTabButton(tab);
                _tabBar.Controls.Add(button);
            }

            var add = new Button
            {
                Text = "+",
                Width = 34,
                Height = 27,
                Margin = new Padding(2, 0, 0, 0),
                FlatStyle = FlatStyle.Flat,
                BackColor = Color.FromArgb(55, 55, 55),
                ForeColor = Color.White,
                TabStop = false
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
            Tag = tab
        };
        button.FlatAppearance.BorderSize = 0;
        button.Click += (_, _) => SelectTab(tab.Id);

        var menu = new ContextMenuStrip();
        var edit = new ToolStripMenuItem("Edit");
        edit.Click += (_, _) => EditTab(tab);
        var delete = new ToolStripMenuItem("Delete");
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
    }

    private void TrimHistory()
    {
        var cap = Math.Clamp(_settings.Chat.MaxHistory, 10, 500);
        if (_history.Count > cap)
            _history.RemoveRange(0, _history.Count - cap);
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

        // ZDPS applies the regex filters to text chat. ReadyAlert keeps that behavior,
        // but makes matching case-insensitive and supports explicit AND/OR expressions.
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
        var oldScrollAtEnd = _messages.SelectionStart >= Math.Max(0, _messages.TextLength - 2);
        _messages.SuspendLayout();
        try
        {
            _messages.Clear();
            _lineMessageMap.Clear();
            var tab = SelectedTab;

            foreach (var message in _history)
            {
                if (!IsVisibleForTab(message, tab)) continue;
                AppendMessage(message);
            }

            if (!keepScroll || oldScrollAtEnd)
            {
                _messages.SelectionStart = _messages.TextLength;
                _messages.ScrollToCaret();
            }
            else
            {
                _messages.SelectionStart = Math.Min(oldSelection, _messages.TextLength);
                _messages.ScrollToCaret();
            }
        }
        finally
        {
            _messages.ResumeLayout();
        }
    }

    private void AppendMessage(ChatMessageEvent message)
    {
        var line = _messages.GetLineFromCharIndex(_messages.TextLength);
        _lineMessageMap[line] = message;

        AppendColored($"[{GetChannelName(message.Channel)}] ", GetChannelColor(message.Channel));

        if (_settings.Chat.CompactMode)
        {
            if (_settings.Chat.ShowTime)
                AppendColored(GetTimeText(message.Timestamp) + " ", Color.FromArgb(155, 166, 190));
            AppendColored($"[{message.SenderName}] ", Color.FromArgb(102, 179, 255));
            AppendColored(message.Text + Environment.NewLine, Color.Gainsboro);
            return;
        }

        AppendColored($"[{message.SenderName}]", Color.FromArgb(102, 179, 255));
        if (_settings.Chat.ShowTime)
            AppendColored("  " + GetTimeText(message.Timestamp), Color.FromArgb(155, 166, 190));
        AppendColored(Environment.NewLine, Color.Gainsboro);

        var contentLine = _messages.GetLineFromCharIndex(_messages.TextLength);
        _lineMessageMap[contentLine] = message;
        AppendColored(message.Text + Environment.NewLine, Color.Gainsboro);
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
            var area = Screen.PrimaryScreen?.WorkingArea ?? new Rectangle(0, 0, 1920, 1080);
            Location = new Point(
                Math.Max(area.Left, area.Right - Width - 40),
                Math.Max(area.Top, area.Bottom - Height - 80));
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
