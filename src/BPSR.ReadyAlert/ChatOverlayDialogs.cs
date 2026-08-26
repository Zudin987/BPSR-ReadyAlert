using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed class ChatTabEditorForm : Form
{
    private static readonly (string Label, ChatChannel Channel)[] ChannelChoices =
    [
        ("World", ChatChannel.World),
        ("Local", ChatChannel.Local),
        ("Group", ChatChannel.Group),
        ("Team", ChatChannel.Team),
        ("Private", ChatChannel.Private),
        ("Union", ChatChannel.Union),
        ("System", ChatChannel.System),
        ("Top Notice", ChatChannel.TopNotice),
        ("Newbie", ChatChannel.Newbie),
        ("Play", ChatChannel.Play)
    ];

    private readonly ChatTabSettings _tab;
    private readonly TextBox _name = new();
    private readonly CheckedListBox _channels = new();
    private readonly NumericUpDown _minLevel = new();
    private readonly TextBox _show = new();
    private readonly TextBox _hide = new();

    internal ChatTabEditorForm(ChatTabSettings tab, bool isNew)
    {
        _tab = tab;
        Text = isNew ? "Add Chat Tab" : $"Edit Chat Tab - {tab.Name}";
        StartPosition = FormStartPosition.CenterParent;
        FormBorderStyle = FormBorderStyle.FixedDialog;
        MaximizeBox = false;
        MinimizeBox = false;
        ShowInTaskbar = false;
        ClientSize = new Size(570, 510);
        BackColor = Color.FromArgb(35, 35, 35);
        ForeColor = Color.Gainsboro;
        Font = new Font("Segoe UI", 9F);

        var table = new TableLayoutPanel
        {
            Dock = DockStyle.Fill,
            ColumnCount = 2,
            RowCount = 8,
            Padding = new Padding(12),
            AutoSize = false
        };
        table.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 125));
        table.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100));
        table.RowStyles.Add(new RowStyle(SizeType.Absolute, 34));
        table.RowStyles.Add(new RowStyle(SizeType.Absolute, 150));
        table.RowStyles.Add(new RowStyle(SizeType.Absolute, 38));
        table.RowStyles.Add(new RowStyle(SizeType.Absolute, 78));
        table.RowStyles.Add(new RowStyle(SizeType.Absolute, 78));
        table.RowStyles.Add(new RowStyle(SizeType.Absolute, 62));
        table.RowStyles.Add(new RowStyle(SizeType.Percent, 100));
        table.RowStyles.Add(new RowStyle(SizeType.Absolute, 42));

        _name.Text = tab.Name;
        _name.Dock = DockStyle.Fill;
        AddLabel(table, "Name:", 0);
        table.Controls.Add(_name, 1, 0);

        _channels.Dock = DockStyle.Fill;
        _channels.CheckOnClick = true;
        _channels.BackColor = Color.FromArgb(45, 45, 45);
        _channels.ForeColor = Color.Gainsboro;
        for (var i = 0; i < ChannelChoices.Length; i++)
        {
            _channels.Items.Add(ChannelChoices[i].Label);
            if (tab.Channels.Contains((int)ChannelChoices[i].Channel))
                _channels.SetItemChecked(i, true);
        }
        AddLabel(table, "Channels:", 1);
        table.Controls.Add(_channels, 1, 1);

        _minLevel.Minimum = 1;
        _minLevel.Maximum = 100;
        _minLevel.Value = Math.Clamp(tab.MinLevel, 1, 100);
        _minLevel.Width = 100;
        AddLabel(table, "Min Level:", 2);
        table.Controls.Add(_minLevel, 1, 2);

        ConfigureFilterBox(_show, tab.ShowIfMatches);
        AddLabel(table, "Show If Matches:", 3);
        table.Controls.Add(_show, 1, 3);

        ConfigureFilterBox(_hide, tab.HideIfMatches);
        AddLabel(table, "Hide If Matches:", 4);
        table.Controls.Add(_hide, 1, 4);

        var help = new Label
        {
            Dock = DockStyle.Fill,
            AutoSize = false,
            ForeColor = Color.Silver,
            Text = "Case-insensitive regex. Combine multiple filters with AND / OR (also && / ||).\r\n" +
                   "Examples:  food OR serum    |    boss AND hard    |    (raid|dungeon)"
        };
        table.SetColumnSpan(help, 2);
        table.Controls.Add(help, 0, 5);

        var buttons = new FlowLayoutPanel
        {
            FlowDirection = FlowDirection.RightToLeft,
            Dock = DockStyle.Fill,
            Padding = new Padding(0, 4, 0, 0)
        };
        var save = new Button { Text = "Save", Width = 90, DialogResult = DialogResult.None };
        var cancel = new Button { Text = "Cancel", Width = 90, DialogResult = DialogResult.Cancel };
        save.Click += (_, _) => SaveAndClose();
        buttons.Controls.Add(save);
        buttons.Controls.Add(cancel);
        table.SetColumnSpan(buttons, 2);
        table.Controls.Add(buttons, 0, 7);

        Controls.Add(table);
        AcceptButton = save;
        CancelButton = cancel;
    }

    private static void AddLabel(TableLayoutPanel table, string text, int row)
    {
        table.Controls.Add(new Label
        {
            Text = text,
            Dock = DockStyle.Fill,
            TextAlign = ContentAlignment.MiddleLeft,
            AutoSize = false
        }, 0, row);
    }

    private static void ConfigureFilterBox(TextBox box, string value)
    {
        box.Text = value;
        box.Multiline = true;
        box.ScrollBars = ScrollBars.Vertical;
        box.Dock = DockStyle.Fill;
        box.BackColor = Color.FromArgb(45, 45, 45);
        box.ForeColor = Color.Gainsboro;
    }

    private void SaveAndClose()
    {
        var name = _name.Text.Trim();
        if (name.Length == 0)
        {
            MessageBox.Show(this, "Enter a tab name.", "BPSR Chat", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }

        var channels = new List<int>();
        for (var i = 0; i < ChannelChoices.Length; i++)
        {
            if (_channels.GetItemChecked(i))
                channels.Add((int)ChannelChoices[i].Channel);
        }
        if (channels.Count == 0)
        {
            MessageBox.Show(this, "Select at least one chat channel.", "BPSR Chat", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }

        if (!ChatFilterExpression.TryValidate(_show.Text, out var showError))
        {
            MessageBox.Show(this, showError, "Show filter is invalid", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }
        if (!ChatFilterExpression.TryValidate(_hide.Text, out var hideError))
        {
            MessageBox.Show(this, hideError, "Hide filter is invalid", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }

        _tab.Name = name;
        _tab.Channels = channels;
        _tab.MinLevel = (int)_minLevel.Value;
        _tab.ShowIfMatches = _show.Text.Trim();
        _tab.HideIfMatches = _hide.Text.Trim();
        DialogResult = DialogResult.OK;
        Close();
    }
}

internal sealed class ChatGeneralSettingsForm : Form
{
    private readonly ChatOverlaySettings _settings;
    private readonly CheckBox _topMost = new() { Text = "Top Most" };
    private readonly CheckBox _compact = new() { Text = "Compact Mode" };
    private readonly CheckBox _showTime = new() { Text = "Show Time" };
    private readonly CheckBox _timeAgo = new() { Text = "Show time as X seconds ago" };
    private readonly CheckBox _hideStickers = new() { Text = "Hide stickers" };
    private readonly TrackBar _opacity = new();
    private readonly Label _opacityValue = new();
    private readonly NumericUpDown _maxHistory = new();

    internal ChatGeneralSettingsForm(ChatOverlaySettings settings)
    {
        _settings = settings;
        Text = "BPSR Chat Settings";
        StartPosition = FormStartPosition.CenterParent;
        FormBorderStyle = FormBorderStyle.FixedDialog;
        MaximizeBox = false;
        MinimizeBox = false;
        ShowInTaskbar = false;
        ClientSize = new Size(440, 330);
        BackColor = Color.FromArgb(35, 35, 35);
        ForeColor = Color.Gainsboro;
        Font = new Font("Segoe UI", 9F);

        var panel = new FlowLayoutPanel
        {
            Dock = DockStyle.Fill,
            FlowDirection = FlowDirection.TopDown,
            WrapContents = false,
            Padding = new Padding(14),
            AutoScroll = true
        };

        _topMost.Checked = settings.TopMost;
        _compact.Checked = settings.CompactMode;
        _showTime.Checked = settings.ShowTime;
        _timeAgo.Checked = settings.ShowTimeAsAgo;
        _hideStickers.Checked = settings.HideStickers;
        foreach (var box in new[] { _topMost, _compact, _showTime, _timeAgo, _hideStickers })
        {
            box.AutoSize = true;
            box.Margin = new Padding(3, 3, 3, 7);
            panel.Controls.Add(box);
        }

        var opacityRow = new FlowLayoutPanel { Width = 390, Height = 48, FlowDirection = FlowDirection.LeftToRight };
        opacityRow.Controls.Add(new Label { Text = "Window Opacity:", Width = 105, TextAlign = ContentAlignment.MiddleLeft, Height = 35 });
        _opacity.Minimum = 25;
        _opacity.Maximum = 100;
        _opacity.TickFrequency = 10;
        _opacity.Value = Math.Clamp(settings.WindowOpacity, 25, 100);
        _opacity.Width = 220;
        _opacity.ValueChanged += (_, _) => _opacityValue.Text = _opacity.Value + "%";
        opacityRow.Controls.Add(_opacity);
        _opacityValue.Text = _opacity.Value + "%";
        _opacityValue.Width = 50;
        _opacityValue.Height = 35;
        _opacityValue.TextAlign = ContentAlignment.MiddleLeft;
        opacityRow.Controls.Add(_opacityValue);
        panel.Controls.Add(opacityRow);

        var historyRow = new FlowLayoutPanel { Width = 390, Height = 38, FlowDirection = FlowDirection.LeftToRight };
        historyRow.Controls.Add(new Label { Text = "Max Chat History:", Width = 115, Height = 28, TextAlign = ContentAlignment.MiddleLeft });
        _maxHistory.Minimum = 10;
        _maxHistory.Maximum = 500;
        _maxHistory.Increment = 10;
        _maxHistory.Value = Math.Clamp(settings.MaxHistory, 10, 500);
        historyRow.Controls.Add(_maxHistory);
        panel.Controls.Add(historyRow);

        var blocked = new Button { Text = "Manage Blocked Users", Width = 390, Height = 30 };
        blocked.Click += (_, _) =>
        {
            using var dialog = new BlockedUsersForm(settings);
            dialog.ShowDialog(this);
        };
        panel.Controls.Add(blocked);

        var buttons = new FlowLayoutPanel { Width = 390, Height = 42, FlowDirection = FlowDirection.RightToLeft, Padding = new Padding(0, 8, 0, 0) };
        var save = new Button { Text = "Save", Width = 90 };
        var cancel = new Button { Text = "Cancel", Width = 90, DialogResult = DialogResult.Cancel };
        save.Click += (_, _) => SaveAndClose();
        buttons.Controls.Add(save);
        buttons.Controls.Add(cancel);
        panel.Controls.Add(buttons);

        Controls.Add(panel);
        AcceptButton = save;
        CancelButton = cancel;
    }

    private void SaveAndClose()
    {
        _settings.TopMost = _topMost.Checked;
        _settings.CompactMode = _compact.Checked;
        _settings.ShowTime = _showTime.Checked;
        _settings.ShowTimeAsAgo = _timeAgo.Checked;
        _settings.HideStickers = _hideStickers.Checked;
        _settings.WindowOpacity = _opacity.Value;
        _settings.MaxHistory = (int)_maxHistory.Value;
        DialogResult = DialogResult.OK;
        Close();
    }
}

internal sealed class BlockedUsersForm : Form
{
    private readonly ChatOverlaySettings _settings;
    private readonly ListBox _list = new();

    internal BlockedUsersForm(ChatOverlaySettings settings)
    {
        _settings = settings;
        Text = "Blocked Chat Users";
        StartPosition = FormStartPosition.CenterParent;
        FormBorderStyle = FormBorderStyle.FixedDialog;
        MaximizeBox = false;
        MinimizeBox = false;
        ShowInTaskbar = false;
        ClientSize = new Size(430, 310);
        BackColor = Color.FromArgb(35, 35, 35);
        ForeColor = Color.Gainsboro;
        Font = new Font("Segoe UI", 9F);

        _list.Dock = DockStyle.Fill;
        _list.BackColor = Color.FromArgb(45, 45, 45);
        _list.ForeColor = Color.Gainsboro;
        RefreshList();

        var bottom = new FlowLayoutPanel
        {
            Dock = DockStyle.Bottom,
            Height = 44,
            FlowDirection = FlowDirection.RightToLeft,
            Padding = new Padding(6)
        };
        var close = new Button { Text = "Close", Width = 85, DialogResult = DialogResult.OK };
        var unblock = new Button { Text = "Unblock", Width = 85 };
        unblock.Click += (_, _) => UnblockSelected();
        bottom.Controls.Add(close);
        bottom.Controls.Add(unblock);

        Controls.Add(_list);
        Controls.Add(bottom);
        AcceptButton = close;
    }

    private void RefreshList()
    {
        _list.Items.Clear();
        foreach (var user in _settings.BlockedUsers.OrderBy(x => x.Name, StringComparer.OrdinalIgnoreCase))
            _list.Items.Add(new BlockedUserItem(user));
    }

    private void UnblockSelected()
    {
        if (_list.SelectedItem is not BlockedUserItem item) return;
        _settings.BlockedUsers.Remove(item.User);
        RefreshList();
    }

    private sealed class BlockedUserItem(ChatBlockedUser user)
    {
        internal ChatBlockedUser User { get; } = user;
        public override string ToString() => $"{User.Name}  [UID {User.Id}]";
    }
}
