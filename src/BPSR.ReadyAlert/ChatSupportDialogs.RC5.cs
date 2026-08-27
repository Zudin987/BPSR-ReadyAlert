using System.Drawing;
using System.Drawing.Text;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed class HotkeyCaptureTextBox : TextBox
{
    internal HotkeyCaptureTextBox()
    {
        ReadOnly = true;
        Width = 250;
        ShortcutsEnabled = false;
        ChatUiTheme.StyleTextBox(this);
        KeyDown += CaptureKeyDown;
        PreviewKeyDown += (_, e) => e.IsInputKey = true;
    }

    private void CaptureKeyDown(object? sender, KeyEventArgs e)
    {
        if (e.KeyCode is Keys.Back or Keys.Delete)
        {
            Text = string.Empty;
            e.SuppressKeyPress = true;
            return;
        }
        var text = ChatHotkey.FromKeyData(e.KeyData);
        if (!string.IsNullOrWhiteSpace(text)) Text = text;
        e.SuppressKeyPress = true;
    }
}

internal sealed class ChannelColorsForm : Form
{
    private readonly Dictionary<int, string> _working;
    internal Dictionary<int, string> Result => new(_working);

    internal ChannelColorsForm(Dictionary<int, string> current)
    {
        _working = new Dictionary<int, string>(current);
        foreach (var pair in ChatOverlaySettings.CreateDefaultChannelColors())
            if (!_working.ContainsKey(pair.Key)) _working[pair.Key] = pair.Value;

        ChatUiTheme.ApplyForm(this);
        Text = "Channel Colors";
        StartPosition = FormStartPosition.CenterParent;
        ShowInTaskbar = false;
        ClientSize = new Size(600, 620);
        MinimumSize = new Size(520, 480);

        var footer = new Panel { Dock = DockStyle.Bottom, Height = 64, BackColor = ChatUiTheme.Surface, Padding = new Padding(16, 14, 16, 14) };
        var buttons = new FlowLayoutPanel { Dock = DockStyle.Fill, FlowDirection = FlowDirection.RightToLeft, WrapContents = false };
        var save = new Button { Text = "Save colors", Width = 112, Height = 36, DialogResult = DialogResult.OK };
        var cancel = new Button { Text = "Cancel", Width = 92, Height = 36, DialogResult = DialogResult.Cancel, Margin = new Padding(0, 0, 8, 0) };
        var reset = new Button { Text = "Reset defaults", Width = 116, Height = 36, Margin = new Padding(0, 0, 8, 0) };
        ChatUiTheme.StylePrimaryButton(save);
        ChatUiTheme.StyleSecondaryButton(cancel);
        ChatUiTheme.StyleSecondaryButton(reset);
        buttons.Controls.Add(save);
        buttons.Controls.Add(cancel);
        buttons.Controls.Add(reset);
        footer.Controls.Add(buttons);

        var host = new Panel { Dock = DockStyle.Fill, AutoScroll = true, Padding = new Padding(24), BackColor = ChatUiTheme.Window };
        var stack = new TableLayoutPanel { Dock = DockStyle.Top, AutoSize = true, ColumnCount = 1, RowCount = 0 };
        stack.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        AddStack(stack, ChatUiTheme.Heading("Channel colors", 17F));
        var sub = ChatUiTheme.Subheading("Choose the color used for each channel label and optional side strip.");
        sub.Margin = new Padding(0, 6, 0, 18);
        AddStack(stack, sub);
        var table = new TableLayoutPanel { Dock = DockStyle.Top, AutoSize = true, ColumnCount = 2, RowCount = 0, BackColor = ChatUiTheme.Surface, Padding = new Padding(16) };
        table.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 48F));
        table.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 52F));

        void Rebuild()
        {
            table.SuspendLayout();
            table.Controls.Clear();
            table.RowStyles.Clear();
            table.RowCount = 0;
            foreach (var channel in Enum.GetValues<ChatChannel>())
            {
                var row = table.RowCount++;
                table.RowStyles.Add(new RowStyle(SizeType.Absolute, 44F));
                var key = (int)channel;
                var label = new Label { Text = ChannelLabel(channel), Dock = DockStyle.Fill, TextAlign = ContentAlignment.MiddleLeft, ForeColor = ChatUiTheme.Text };
                var button = new Button { Text = _working[key], Dock = DockStyle.Fill, Margin = new Padding(8, 6, 0, 6), BackColor = ChatColorUtil.Parse(_working[key], Color.Gray) };
                button.ForeColor = Contrast(button.BackColor);
                button.FlatStyle = FlatStyle.Flat;
                button.FlatAppearance.BorderColor = ChatUiTheme.BorderStrong;
                button.Click += (_, _) =>
                {
                    using var color = new ColorDialog { FullOpen = true, Color = ChatColorUtil.Parse(_working[key], Color.Gray) };
                    if (color.ShowDialog(this) != DialogResult.OK) return;
                    _working[key] = ChatColorUtil.ToHtml(color.Color);
                    button.Text = _working[key];
                    button.BackColor = color.Color;
                    button.ForeColor = Contrast(color.Color);
                };
                table.Controls.Add(label, 0, row);
                table.Controls.Add(button, 1, row);
            }
            table.ResumeLayout();
        }

        reset.Click += (_, _) =>
        {
            _working.Clear();
            foreach (var pair in ChatOverlaySettings.CreateDefaultChannelColors()) _working[pair.Key] = pair.Value;
            Rebuild();
        };
        Rebuild();
        AddStack(stack, table);
        host.Controls.Add(stack);
        Controls.Add(host);
        Controls.Add(footer);
        AcceptButton = save;
        CancelButton = cancel;
    }

    private static string ChannelLabel(ChatChannel channel) => channel switch
    {
        ChatChannel.Null => "Null / Other",
        ChatChannel.Local => "Local / Scene",
        ChatChannel.Union => "Union / Guild",
        ChatChannel.TopNotice => "Top Notice",
        _ => channel.ToString()
    };

    private static Color Contrast(Color color) =>
        color.R * 299 + color.G * 587 + color.B * 114 >= 150_000 ? Color.Black : Color.White;

    private static void AddStack(TableLayoutPanel stack, Control control)
    {
        var row = stack.RowCount++;
        stack.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        control.Dock = DockStyle.Top;
        stack.Controls.Add(control, 0, row);
    }
}

internal sealed class BlockedUsersForm : Form
{
    internal const string ScopeText = "Blocked players are ignored by ReadyAlert chat: hidden from the overlay and skipped by keyword/private sounds, translation and TTS. Ready / Queue alerts are unaffected.";

    private readonly List<ChatBlockedUser> _blockedUsers;
    private readonly ListBox _list = new();
    private readonly Label _empty = new();

    internal BlockedUsersForm(List<ChatBlockedUser> blockedUsers)
    {
        _blockedUsers = blockedUsers;
        ChatUiTheme.ApplyForm(this);
        Text = "Blocked Chat Users";
        StartPosition = FormStartPosition.CenterParent;
        ShowInTaskbar = false;
        ClientSize = new Size(620, 490);
        MinimumSize = new Size(520, 400);

        var footer = new Panel { Dock = DockStyle.Bottom, Height = 64, BackColor = ChatUiTheme.Surface, Padding = new Padding(16, 14, 16, 14) };
        var buttons = new FlowLayoutPanel { Dock = DockStyle.Fill, FlowDirection = FlowDirection.RightToLeft, WrapContents = false };
        var close = new Button { Text = "Done", Width = 92, Height = 36, DialogResult = DialogResult.OK };
        var unblock = new Button { Text = "Unblock selected", Width = 130, Height = 36, Margin = new Padding(0, 0, 8, 0) };
        ChatUiTheme.StylePrimaryButton(close);
        ChatUiTheme.StyleSecondaryButton(unblock);
        unblock.Click += (_, _) => UnblockSelected();
        buttons.Controls.Add(close);
        buttons.Controls.Add(unblock);
        footer.Controls.Add(buttons);

        var body = new Panel { Dock = DockStyle.Fill, Padding = new Padding(24), BackColor = ChatUiTheme.Window };
        var header = new Panel { Dock = DockStyle.Top, Height = 106 };
        var title = ChatUiTheme.Heading("Blocked users", 17F); title.Location = new Point(0, 0);
        var hint = ChatUiTheme.Subheading(ScopeText); hint.Location = new Point(0, 34); hint.MaximumSize = new Size(540, 0);
        header.Controls.Add(title); header.Controls.Add(hint);

        _list.Dock = DockStyle.Fill;
        _list.BackColor = ChatUiTheme.Input;
        _list.ForeColor = ChatUiTheme.Text;
        _list.BorderStyle = BorderStyle.FixedSingle;
        _list.IntegralHeight = false;
        _list.DoubleClick += (_, _) => UnblockSelected();

        _empty.Dock = DockStyle.Fill;
        _empty.Text = "No blocked users\r\nRight-click a chat message to block a player.";
        _empty.TextAlign = ContentAlignment.MiddleCenter;
        _empty.ForeColor = ChatUiTheme.Muted;
        _empty.Font = ChatUiTheme.UiFont(10F);
        body.Controls.Add(_empty);
        body.Controls.Add(_list);
        body.Controls.Add(header);
        Controls.Add(body);
        Controls.Add(footer);
        AcceptButton = close;
        RefreshList();
    }

    private void RefreshList()
    {
        _list.Items.Clear();
        foreach (var user in _blockedUsers.OrderBy(x => x.Name, StringComparer.OrdinalIgnoreCase))
            _list.Items.Add(new BlockedUserItem(user));
        _list.Visible = _list.Items.Count > 0;
        _empty.Visible = !_list.Visible;
    }

    private void UnblockSelected()
    {
        if (_list.SelectedItem is not BlockedUserItem item) return;
        _blockedUsers.Remove(item.User);
        RefreshList();
    }

    private sealed class BlockedUserItem(ChatBlockedUser user)
    {
        internal ChatBlockedUser User { get; } = user;
        public override string ToString() => $"{User.Name}    UID {User.Id}";
    }
}

internal sealed class ChatDebugStatusForm : Form
{
    internal const string LiveUpdateHint = "Live counters refresh every 0.5 seconds. Updates pause while the status box has focus so scrolling and text selection stay in place.";

    private readonly TextBox _status = new();
    private readonly System.Windows.Forms.Timer _timer = new() { Interval = 500 };

    internal ChatDebugStatusForm()
    {
        ChatUiTheme.ApplyForm(this);
        Text = "Chat Capture Status";
        StartPosition = FormStartPosition.CenterParent;
        ShowInTaskbar = false;
        ClientSize = new Size(700, 630);
        MinimumSize = new Size(580, 460);

        var footer = new Panel { Dock = DockStyle.Bottom, Height = 64, BackColor = ChatUiTheme.Surface, Padding = new Padding(16, 14, 16, 14) };
        var buttons = new FlowLayoutPanel { Dock = DockStyle.Fill, FlowDirection = FlowDirection.RightToLeft, WrapContents = false };
        var close = new Button { Text = "Done", Width = 92, Height = 36, DialogResult = DialogResult.OK };
        var copy = new Button { Text = "Copy status", Width = 108, Height = 36, Margin = new Padding(0, 0, 8, 0) };
        ChatUiTheme.StylePrimaryButton(close);
        ChatUiTheme.StyleSecondaryButton(copy);
        copy.Click += (_, _) =>
        {
            if (!string.IsNullOrEmpty(_status.Text))
                ChatClipboard.TrySetText(this, _status.Text, "copy-chat-status");
        };
        buttons.Controls.Add(close);
        buttons.Controls.Add(copy);
        footer.Controls.Add(buttons);

        var body = new Panel { Dock = DockStyle.Fill, Padding = new Padding(24), BackColor = ChatUiTheme.Window };
        var header = new Panel { Dock = DockStyle.Top, Height = 100 };
        var title = ChatUiTheme.Heading("Chat capture status", 17F); title.Location = new Point(0, 0);
        var hint = ChatUiTheme.Subheading(LiveUpdateHint); hint.Location = new Point(0, 34); hint.MaximumSize = new Size(590, 0);
        header.Controls.Add(title); header.Controls.Add(hint);

        _status.Dock = DockStyle.Fill;
        _status.Multiline = true;
        _status.ReadOnly = true;
        _status.ScrollBars = ScrollBars.Vertical;
        _status.BackColor = ChatUiTheme.Input;
        _status.ForeColor = ChatUiTheme.Text;
        _status.BorderStyle = BorderStyle.FixedSingle;
        _status.Font = new Font(FontFamily.GenericMonospace, 9F);
        body.Controls.Add(_status);
        body.Controls.Add(header);
        Controls.Add(body);
        Controls.Add(footer);
        AcceptButton = close;

        _timer.Tick += (_, _) => RefreshStatus();
        FormClosed += (_, _) =>
        {
            _timer.Stop();
            _timer.Dispose();
        };
        RefreshStatus();
        _timer.Start();
    }

    private void RefreshStatus()
    {
        // Replacing Text every 500 ms resets selection/scroll. Treat focus as an
        // explicit user-interaction pause; updates resume as soon as focus moves to
        // Copy status, Done, or another window.
        if (_status.Focused && _status.TextLength > 0) return;

        var capture = ChatCaptureBridge.GetStatus();
        var notify = ChatNotificationEngine.GetStatus();
        var speech = ChatSpeechTranslationEngine.GetStatus();

        _status.Text =
            $"BPSR ReadyAlert {AppVersion.Current}\r\n" +
            "================================================\r\n" +
            $"Chat enabled          {capture.Enabled}\r\n" +
            "Capture pipeline       Shared ReadyAlert CaptureEngine\r\n" +
            "Second Npcap capture   No\r\n" +
            $"Service ID            {ChatProtocol.ServiceId}\r\n" +
            $"Method                0x{ChatProtocol.NotifyNewestChitChatMsgs:X2}\r\n\r\n" +
            "CAPTURE / PARSER\r\n" +
            $"Matching notifies     {capture.MatchingNotifies}\r\n" +
            $"Parsed messages       {capture.ParsedMessages}\r\n" +
            $"Parse failures        {capture.ParseFailures}\r\n" +
            $"Queue drops           {capture.DroppedQueuedMessages}\r\n" +
            $"Pending UI queue      {capture.QueueCount}\r\n" +
            $"Last payload bytes    {capture.LastPayloadLength}\r\n" +
            $"Last message UTC      {Utc(capture.LastMessageUtc)}\r\n\r\n" +
            "KEYWORD / PRIVATE SOUND\r\n" +
            $"Engine enabled        {notify.Enabled}\r\n" +
            $"Pending queue         {notify.QueueCount}\r\n" +
            $"Enqueued / processed  {notify.Enqueued} / {notify.Processed}\r\n" +
            $"Matched / played      {notify.Matched} / {notify.Played}\r\n" +
            $"Failed / dropped      {notify.Failed} / {notify.Dropped}\r\n" +
            $"Last reason           {OneLine(notify.LastReason)}\r\n" +
            $"Last attempt UTC      {Utc(notify.LastAttemptUtc)}\r\n" +
            $"Last success UTC      {Utc(notify.LastSuccessUtc)}\r\n" +
            $"Last error            {OneLine(notify.LastError)}\r\n\r\n" +
            "TRANSLATION / TTS\r\n" +
            $"Engine enabled        {speech.Enabled}\r\n" +
            $"Pending queue         {speech.QueueCount}\r\n" +
            $"Processed             {speech.Processed}\r\n" +
            $"Translations shown    {speech.Translated}\r\n" +
            $"Messages spoken       {speech.Spoken}\r\n" +
            $"Translation failures  {speech.TranslationFailures}\r\n" +
            $"TTS failures          {speech.TtsFailures}\r\n" +
            $"Dropped / stale       {speech.Dropped}\r\n" +
            $"Last success UTC      {Utc(speech.LastSuccessUtc)}\r\n\r\n" +
            "TESTING TIP\r\n" +
            "If Parsed messages stops increasing, inspect capture/parser.\r\n" +
            "If Parsed increases but TTS Processed does not, inspect channel/TTS settings.\r\n" +
            "If Processed increases but Messages spoken does not, inspect Google/audio or TTS failures.";
    }

    private static string Utc(DateTime? value) =>
        value?.ToString("yyyy-MM-dd HH:mm:ss") ?? "Never";

    private static string OneLine(string? value)
    {
        if (string.IsNullOrWhiteSpace(value)) return "-";
        var text = value.Replace('\r', ' ').Replace('\n', ' ').Trim();
        return text.Length <= 120 ? text : text[..120] + "…";
    }
}
