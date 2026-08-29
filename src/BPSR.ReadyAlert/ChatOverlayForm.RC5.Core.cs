using System.Drawing;
using System.Media;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatOverlayForm : Form
{
    private const int MaxSenderNameLength = 128;
    private const int MaxDisplayedMessageLength = 8 * 1024;
    private const int ClickThroughHotkeyId = 0x5141;
    private const int CollapseHotkeyId = 0x5142;
    private const int ResizeGrip = 12;
    private const int FrameBorderThickness = 2;
    private const int CollapsedThickness = 24;
    private const double CollapsedOpacityCap = 0.58d;

    private readonly AppSettings _settings;
    private readonly SettingsStore _settingsStore;
    private readonly string _defaultSoundPath;
    private readonly List<ChatMessageEvent> _history = [];
    private readonly Panel _topPanel;
    private readonly FlowLayoutPanel _tabBar;
    private readonly FlowLayoutPanel _actionBar;
    private readonly ChatMessageListBox _messages;
    private readonly Button _addTabButton;
    private readonly Button _gearButton;
    private readonly Button _collapseButton;
    private readonly Button _hideButton;
    private readonly Button _dragGrip;
    private readonly Button _newMessagesButton;
    private readonly Button _collapsedHandle;
    private readonly TableLayoutPanel _emptyState;
    private readonly Label _emptyTitle;
    private readonly Label _emptyHint;
    private readonly Button _emptyStatusButton;
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

        ChatUiTheme.ApplyForm(this);
        Text = string.Empty;
        ShowInTaskbar = false;
        ShowIcon = false;
        ControlBox = false;
        StartPosition = FormStartPosition.Manual;
        MinimumSize = new Size(420, 220);
        Size = new Size(_settings.Chat.WindowWidth, _settings.Chat.WindowHeight);
        FormBorderStyle = FormBorderStyle.None;
        Padding = new Padding(FrameBorderThickness);
        BackColor = ChatUiTheme.BorderStrong;
        DoubleBuffered = true;

        TryLoadIcon(iconPath);
        RestoreWindowPlacement();
        _expandedBounds = Bounds;

        _topPanel = new Panel
        {
            Dock = DockStyle.Top,
            Height = 42,
            Padding = Padding.Empty,
            BackColor = ChatUiTheme.Surface
        };

        _dragGrip = MakeToolbarButton("⋮⋮", 38, "Drag chat overlay");
        _dragGrip.Dock = DockStyle.Left;
        _dragGrip.Cursor = Cursors.SizeAll;
        _dragGrip.Font = ChatUiTheme.UiFont(10F, FontStyle.Bold);
        _dragGrip.MouseDown += (_, e) =>
        {
            if (e.Button == MouseButtons.Left && !_collapsed && !_settings.Chat.ClickThrough)
                ChatNativeMethods.BeginWindowDrag(Handle);
        };

        _actionBar = new FlowLayoutPanel
        {
            Dock = DockStyle.Right,
            Width = 184,
            FlowDirection = FlowDirection.LeftToRight,
            WrapContents = false,
            BackColor = ChatUiTheme.Surface,
            Padding = Padding.Empty,
            Margin = Padding.Empty
        };

        _addTabButton = MakeToolbarButton("+ Tab", 66, "Add a custom chat tab");
        _addTabButton.Font = ChatUiTheme.UiFont(8.5F, FontStyle.Bold);
        _addTabButton.Click += (_, _) => AddTab();

        _gearButton = MakeToolbarButton("⚙", 40, "Chat Overlay settings");
        _gearButton.AccessibleName = "Chat Overlay settings";
        _gearButton.Font = new Font("Segoe UI Symbol", 10F, FontStyle.Regular, GraphicsUnit.Point);
        _gearButton.Click += (_, _) => OpenV124CachedSettingsDialog();

        _collapseButton = MakeToolbarButton("▶", 38, "Collapse chat to the selected screen edge");
        _collapseButton.Click += (_, _) => ToggleCollapsed();

        _hideButton = MakeToolbarButton("×", 38, "Hide chat — Chat Overlay stays enabled");
        _hideButton.Font = ChatUiTheme.UiFont(12F);
        _hideButton.Click += (_, _) => HideOverlay();

        _actionBar.Controls.Add(_addTabButton);
        _actionBar.Controls.Add(_gearButton);
        _actionBar.Controls.Add(_collapseButton);
        _actionBar.Controls.Add(_hideButton);

        _tabBar = new FlowLayoutPanel
        {
            Dock = DockStyle.Fill,
            FlowDirection = FlowDirection.LeftToRight,
            WrapContents = false,
            AutoScroll = true,
            Padding = new Padding(8, 0, 4, 0),
            Margin = Padding.Empty,
            BackColor = ChatUiTheme.Surface
        };

        _topPanel.Controls.Add(_tabBar);
        _topPanel.Controls.Add(_actionBar);
        _topPanel.Controls.Add(_dragGrip);

        _messages = new ChatMessageListBox
        {
            Dock = DockStyle.Fill,
            BackColor = ChatUiTheme.Window,
            ForeColor = ChatUiTheme.Text,
            AccessibleName = "BPSR chat messages"
        };
        _messages.MeasureItem += MessagesMeasureItem;
        _messages.DrawItem += MessagesDrawItem;
        _messages.MouseDown += MessagesMouseDown;
        _messages.ViewportChanged += (_, _) => UpdateFollowLatestFromViewport();
        _messages.ContextMenuStrip = BuildMessageMenu();

        (_emptyState, _emptyTitle, _emptyHint, _emptyStatusButton) = BuildEmptyState();

        _newMessagesButton = new Button
        {
            Text = "↓ New messages",
            Width = 160,
            Height = 32,
            FlatStyle = FlatStyle.Flat,
            Visible = false,
            TabStop = false,
            Anchor = AnchorStyles.Right | AnchorStyles.Bottom,
            BackColor = ChatUiTheme.Accent,
            ForeColor = Color.White,
            Cursor = Cursors.Hand
        };
        _newMessagesButton.FlatAppearance.BorderSize = 0;
        _newMessagesButton.FlatAppearance.MouseOverBackColor = ChatUiTheme.AccentHover;
        _newMessagesButton.Click += (_, _) => ResumeFollowingLatest();

        _collapsedHandle = new Button
        {
            Dock = DockStyle.Fill,
            FlatStyle = FlatStyle.Flat,
            Text = "◀",
            Visible = false,
            TabStop = false,
            BackColor = ChatUiTheme.SurfaceRaised,
            ForeColor = ChatUiTheme.Text,
            AccessibleName = "Expand chat overlay",
            Cursor = Cursors.Hand,
            UseVisualStyleBackColor = false
        };
        _collapsedHandle.FlatAppearance.BorderSize = 0;
        _collapsedHandle.FlatAppearance.MouseOverBackColor = ChatUiTheme.SurfaceHover;
        _collapsedHandle.FlatAppearance.MouseDownBackColor = ChatUiTheme.SurfaceRaised;
        _collapsedHandle.Click += (_, _) => ExpandFromEdge();
        _collapsedHandle.VisibleChanged += (_, _) =>
        {
            if (_collapsedHandle.Visible)
                Opacity = GetCollapsedOpacityTarget();
            else
                Opacity = Math.Clamp(_settings.Chat.WindowOpacity, 25, 100) / 100d;
        };

        Controls.Add(_messages);
        Controls.Add(_emptyState);
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

        _resizeTimer = new System.Windows.Forms.Timer { Interval = 140 };
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

        // Register only the Settings-cache lifetime hook after first paint. v1.3.2
        // deliberately does not construct or realize the Settings UI during startup;
        // the measured WinForms tree was expensive enough to cause a visible delayed
        // UI-thread hitch. The actual dialog is created on the first explicit gear click
        // and then cached so later opens and page navigation stay fast.
        Shown += (_, _) => QueueV124SettingsPrewarm();

        ApplyWindowSettings(registerHotkeys: false);
        RebuildTabBar();
        PositionNewMessagesButton();
        UpdateEmptyState();
    }

    private double GetCollapsedOpacityTarget() =>
        Math.Min(Math.Clamp(_settings.Chat.WindowOpacity, 25, 100) / 100d, CollapsedOpacityCap);

    internal (int BorderThickness, int ResizeHitZone, double CollapsedOpacity, bool NativeCollapsedThemeDisabled) GetRc7UxMetricsForSelfTest() =>
        (FrameBorderThickness, ResizeGrip, GetCollapsedOpacityTarget(), !_collapsedHandle.UseVisualStyleBackColor);

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
}
