using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed class ChatTabEditorForm : Form
{
    private static readonly (string Label, int[] Channels)[] ChannelGroups =
    [
        ("World + Newbie", [(int)ChatChannel.World, (int)ChatChannel.Newbie]),
        ("Guild", [(int)ChatChannel.Union]),
        ("Team + Group", [(int)ChatChannel.Team, (int)ChatChannel.Group]),
        ("Private", [(int)ChatChannel.Private]),
        ("Local", [(int)ChatChannel.Local])
    ];

    private readonly ChatTabSettings _tab;
    private bool _isNew;
    private readonly TextBox _name = new();
    private readonly CheckBox[] _channelChecks = ChannelGroups.Select(x => new CheckBox { Text = x.Label }).ToArray();
    private readonly NumericUpDown _minLevel = new();
    private readonly TextBox _show = new();
    private readonly TextBox _hide = new();
    private readonly Label _validation = new();
    private readonly Label _applyStatus = new();
    private readonly Button _save = new();
    private readonly Button _cancel = new();
    private TableLayoutPanel? _channelGrid;
    private string _savedFingerprint = string.Empty;
    private bool _trackingReady;
    private bool _everSaved;
    private bool _appliedNotPersisted;

    internal ChatTabEditorForm(ChatTabSettings tab, bool isNew)
    {
        _tab = tab;
        _isNew = isNew;
        ChatUiTheme.ApplySettingsForm(this);
        Text = isNew ? "Chat Tab Settings for New Tab" : $"Chat Tab Settings — {tab.Name}";
        StartPosition = FormStartPosition.CenterParent;
        FormBorderStyle = FormBorderStyle.Sizable;
        MaximizeBox = false;
        MinimizeBox = false;
        ShowInTaskbar = false;
        ClientSize = new Size(560, 420);
        MinimumSize = new Size(500, 380);

        var footer = BuildFooter();
        var page = new Panel
        {
            Dock = DockStyle.Fill,
            AutoScroll = true,
            BackColor = ChatUiTheme.SettingsWindow,
            Padding = new Padding(8, 7, 8, 8)
        };
        var stack = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            ColumnCount = 1,
            RowCount = 0,
            BackColor = ChatUiTheme.SettingsWindow,
            Margin = Padding.Empty,
            Padding = Padding.Empty
        };
        stack.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));

        AddStack(stack, BuildBasicsSection(tab));
        AddStack(stack, BuildChannelsSection(tab));
        AddStack(stack, BuildFiltersSection(tab));

        page.Controls.Add(stack);
        Controls.Add(page);
        Controls.Add(footer);
        AcceptButton = _save;
        CancelButton = _cancel;

        _name.TextChanged += (_, _) => RefreshDirtyState();
        _minLevel.ValueChanged += (_, _) => RefreshDirtyState();
        _show.TextChanged += (_, _) => { RefreshValidation(); RefreshDirtyState(); };
        _hide.TextChanged += (_, _) => { RefreshValidation(); RefreshDirtyState(); };
        foreach (var check in _channelChecks)
            check.CheckedChanged += (_, _) => RefreshDirtyState();
        FormClosing += TabEditorFormClosing;

        RefreshValidation();
        _savedFingerprint = CaptureFingerprint();
        _trackingReady = true;
    }

    protected override void OnShown(EventArgs e)
    {
        base.OnShown(e);
        if (Owner is ChatOverlayForm overlay && overlay.TopMost)
        {
            TopMost = true;
            BringToFront();
            Activate();
        }
    }

    private Panel BuildFooter()
    {
        var footer = new Panel { Dock = DockStyle.Bottom, Height = 52, BackColor = ChatUiTheme.SettingsWindow };
        footer.Controls.Add(new Panel { Dock = DockStyle.Top, Height = 1, BackColor = ChatUiTheme.SettingsBorder });
        var actions = new TableLayoutPanel
        {
            Dock = DockStyle.Fill,
            ColumnCount = 5,
            RowCount = 1,
            Margin = Padding.Empty,
            Padding = new Padding(4, 8, 4, 6),
            BackColor = ChatUiTheme.SettingsWindow
        };
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 112F));
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 146F));
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 8F));
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 104F));

        _save.Text = "Save tab";
        _save.Dock = DockStyle.Fill;
        _save.Margin = Padding.Empty;
        ChatUiTheme.StyleSettingsSaveButton(_save);
        _save.Click += (_, _) => ApplyTab();

        _applyStatus.Dock = DockStyle.Fill;
        _applyStatus.TextAlign = ContentAlignment.MiddleRight;
        _applyStatus.ForeColor = ChatUiTheme.Success;
        _applyStatus.Font = ChatUiTheme.UiFont(8.5F, FontStyle.Bold);

        _cancel.Text = "Cancel";
        _cancel.Dock = DockStyle.Fill;
        _cancel.DialogResult = DialogResult.Cancel;
        _cancel.Margin = Padding.Empty;
        ChatUiTheme.StyleSettingsCloseButton(_cancel);

        actions.Controls.Add(_save, 0, 0);
        actions.Controls.Add(_applyStatus, 2, 0);
        actions.Controls.Add(_cancel, 4, 0);
        footer.Controls.Add(actions);
        return footer;
    }

    private Control BuildBasicsSection(ChatTabSettings tab)
    {
        _name.Text = tab.Name;
        _name.MaxLength = 40;
        _name.Width = 280;
        _name.MaximumSize = new Size(280, 0);
        ChatUiTheme.StyleSettingsTextBox(_name);

        _minLevel.Minimum = 1;
        _minLevel.Maximum = 100;
        _minLevel.Value = Math.Clamp(tab.MinLevel, 1, 100);
        _minLevel.Width = 80;
        ChatUiTheme.StyleSettingsNumeric(_minLevel);

        var fields = MakeFieldTable();
        AddFieldRow(fields, "Name", _name, 0);
        AddFieldRow(fields, "Min level", _minLevel, 1);
        return MakeSection("Basics", fields);
    }

    private Control BuildChannelsSection(ChatTabSettings tab)
    {
        _channelGrid = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            ColumnCount = 3,
            RowCount = 2,
            Margin = Padding.Empty,
            Padding = new Padding(0, 1, 0, 1),
            BackColor = ChatUiTheme.SettingsWindow
        };
        for (var i = 0; i < 3; i++)
            _channelGrid.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 33.333F));
        _channelGrid.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        _channelGrid.RowStyles.Add(new RowStyle(SizeType.AutoSize));

        for (var i = 0; i < _channelChecks.Length; i++)
        {
            var check = _channelChecks[i];
            check.AutoSize = true;
            check.AutoCheck = true;
            check.FlatStyle = FlatStyle.Standard;
            check.UseVisualStyleBackColor = false;
            check.ForeColor = ChatUiTheme.SettingsText;
            check.BackColor = Color.Transparent;
            check.Cursor = Cursors.Hand;
            check.Margin = new Padding(0, 3, 14, 3);
            check.Checked = ChannelGroups[i].Channels.Any(tab.Channels.Contains);
            _channelGrid.Controls.Add(check, i % 3, i / 3);
        }

        return MakeSection("Channels", _channelGrid);
    }

    private Control BuildFiltersSection(ChatTabSettings tab)
    {
        ConfigureSingleLineFilter(_show, tab.ShowIfMatches);
        ConfigureSingleLineFilter(_hide, tab.HideIfMatches);

        var fields = MakeFieldTable();
        AddFieldRow(fields, "Show if matches", _show, 0);
        AddFieldRow(fields, "Hide if matches", _hide, 1);

        var host = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            ColumnCount = 1,
            RowCount = 2,
            Margin = Padding.Empty,
            Padding = Padding.Empty,
            BackColor = ChatUiTheme.SettingsWindow
        };
        host.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        host.Controls.Add(fields, 0, 0);
        _validation.AutoSize = true;
        _validation.Font = ChatUiTheme.UiFont(8.5F, FontStyle.Bold);
        _validation.Margin = new Padding(148, 3, 0, 0);
        host.Controls.Add(_validation, 0, 1);
        return MakeSection("Filters", host);
    }

    private static TableLayoutPanel MakeFieldTable()
    {
        var table = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            ColumnCount = 2,
            RowCount = 0,
            Margin = Padding.Empty,
            Padding = Padding.Empty,
            BackColor = ChatUiTheme.SettingsWindow
        };
        table.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 140F));
        table.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        return table;
    }

    private static void AddFieldRow(TableLayoutPanel table, string label, Control control, int row)
    {
        table.RowCount = Math.Max(table.RowCount, row + 1);
        table.RowStyles.Add(new RowStyle(SizeType.Absolute, 34F));
        var text = ChatUiTheme.SettingsFieldLabel(label);
        text.Dock = DockStyle.Fill;
        text.TextAlign = ContentAlignment.MiddleLeft;
        text.Margin = Padding.Empty;
        control.Anchor = AnchorStyles.Left | AnchorStyles.Right;
        control.Margin = new Padding(4, 4, 0, 4);
        table.Controls.Add(text, 0, row);
        table.Controls.Add(control, 1, row);
    }

    private static Control MakeSection(string title, Control content)
    {
        var section = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            ColumnCount = 1,
            RowCount = 2,
            Margin = new Padding(0, 0, 0, 8),
            Padding = Padding.Empty,
            BackColor = ChatUiTheme.SettingsWindow
        };
        section.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));

        var heading = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            Height = 20,
            ColumnCount = 2,
            Margin = Padding.Empty,
            Padding = Padding.Empty,
            BackColor = ChatUiTheme.SettingsWindow
        };
        heading.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
        heading.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        var label = ChatUiTheme.SettingsFieldLabel(title);
        label.Padding = new Padding(0, 0, 7, 0);
        var lineHost = new Panel { Dock = DockStyle.Fill, Margin = Padding.Empty, Padding = new Padding(0, 9, 0, 0) };
        lineHost.Controls.Add(new Panel { Dock = DockStyle.Top, Height = 1, BackColor = ChatUiTheme.SettingsBorder });
        heading.Controls.Add(label, 0, 0);
        heading.Controls.Add(lineHost, 1, 0);

        content.Dock = DockStyle.Top;
        content.Margin = Padding.Empty;
        section.Controls.Add(heading, 0, 0);
        section.Controls.Add(content, 0, 1);
        return section;
    }

    private static void AddStack(TableLayoutPanel stack, Control control)
    {
        var row = stack.RowCount++;
        stack.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        control.Dock = DockStyle.Top;
        stack.Controls.Add(control, 0, row);
    }

    private static void ConfigureSingleLineFilter(TextBox box, string value)
    {
        box.Text = value;
        box.Multiline = false;
        box.ScrollBars = ScrollBars.None;
        box.Height = 24;
        box.MaxLength = 4096;
        ChatUiTheme.StyleSettingsTextBox(box);
    }

    private void RefreshValidation()
    {
        if (!ChatFilterExpression.TryValidate(_show.Text, out var showError))
        {
            _validation.ForeColor = ChatUiTheme.Danger;
            _validation.Text = "● Show: " + showError;
            _save.Enabled = false;
            return;
        }
        if (!ChatFilterExpression.TryValidate(_hide.Text, out var hideError))
        {
            _validation.ForeColor = ChatUiTheme.Danger;
            _validation.Text = "● Hide: " + hideError;
            _save.Enabled = false;
            return;
        }
        _validation.ForeColor = ChatUiTheme.Success;
        _validation.Text = "● Filters valid";
        _save.Enabled = true;
    }

    private void RefreshDirtyState()
    {
        if (!_trackingReady) return;
        var dirty = !string.Equals(CaptureFingerprint(), _savedFingerprint, StringComparison.Ordinal);
        if (dirty)
        {
            _applyStatus.Text = "Unsaved";
            _applyStatus.ForeColor = ChatUiTheme.Warning;
        }
        else if (_appliedNotPersisted)
        {
            _applyStatus.Text = "Applied — not saved";
            _applyStatus.ForeColor = ChatUiTheme.Danger;
        }
        else
        {
            _applyStatus.Text = _everSaved ? "Saved ✓" : string.Empty;
            _applyStatus.ForeColor = _everSaved ? ChatUiTheme.Success : ChatUiTheme.SettingsMuted;
        }
    }

    private string CaptureFingerprint() => string.Join('|',
        _name.Text,
        _minLevel.Value,
        string.Join(',', _channelChecks.Select(x => x.Checked ? '1' : '0')),
        _show.Text,
        _hide.Text);

    private void TabEditorFormClosing(object? sender, FormClosingEventArgs e)
    {
        if (!_trackingReady || e.CloseReason != CloseReason.UserClosing) return;
        var dirty = !string.Equals(CaptureFingerprint(), _savedFingerprint, StringComparison.Ordinal);
        if (!dirty && !_appliedNotPersisted) return;

        var (message, title) = (dirty, _appliedNotPersisted) switch
        {
            (true, true) => ("Some tab edits are unsaved and the last applied tab state could not be saved to disk. Close anyway?", "Tab changes are not safely saved"),
            (false, true) => ("This tab is active for this session but could not be saved to disk. Close anyway?", "Tab is not saved"),
            _ => ("Discard unsaved tab changes?", "Unsaved tab changes")
        };
        if (MessageBox.Show(this, message, title, MessageBoxButtons.YesNo, MessageBoxIcon.Warning, MessageBoxDefaultButton.Button2) != DialogResult.Yes)
            e.Cancel = true;
    }

    private void ApplyTab()
    {
        var name = _name.Text.Trim();
        if (name.Length == 0)
        {
            MessageBox.Show(this, "Enter a tab name.", "Chat tab", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            _name.Focus();
            return;
        }

        var channels = new HashSet<int>();
        for (var i = 0; i < _channelChecks.Length; i++)
        {
            if (!_channelChecks[i].Checked) continue;
            foreach (var channel in ChannelGroups[i].Channels) channels.Add(channel);
        }
        if (channels.Count == 0)
        {
            MessageBox.Show(this, "Select at least one channel group.", "Chat tab", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            _channelChecks[0].Focus();
            return;
        }
        if (!ChatFilterExpression.TryValidate(_show.Text, out var showError))
        {
            MessageBox.Show(this, showError, "Show rule is invalid", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }
        if (!ChatFilterExpression.TryValidate(_hide.Text, out var hideError))
        {
            MessageBox.Show(this, hideError, "Hide rule is invalid", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }

        _tab.Name = name;
        _tab.Channels = channels.OrderBy(x => x).ToList();
        _tab.MinLevel = (int)_minLevel.Value;
        _tab.ShowIfMatches = _show.Text.Trim();
        _tab.HideIfMatches = _hide.Text.Trim();

        var saveSucceeded = true;
        if (Owner is ChatOverlayForm overlay)
        {
            saveSucceeded = overlay.ApplyTabFromOpenDialog(_tab, _isNew);
            TopMost = overlay.TopMost;
            if (TopMost) { BringToFront(); Activate(); }
        }

        _isNew = false;
        Text = $"Chat Tab Settings — {_tab.Name}";
        _savedFingerprint = CaptureFingerprint();
        if (!saveSucceeded)
        {
            _everSaved = false;
            _appliedNotPersisted = true;
            _applyStatus.Text = "Applied — not saved";
            _applyStatus.ForeColor = ChatUiTheme.Danger;
            MessageBox.Show(this, "The tab was applied for this session but could not be saved to disk.", "Tab could not be saved", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return;
        }

        _everSaved = true;
        _appliedNotPersisted = false;
        _applyStatus.Text = "Saved ✓";
        _applyStatus.ForeColor = ChatUiTheme.Success;
    }

    internal (Size DefaultClient, Size MinimumWindow, int ChannelsHeight, int ShowHeight, int HideHeight, int NameWidth, int FooterHeight, string CancelText)
        GetV122CompactMetricsForSelfTest()
    {
        var footer = Controls.OfType<Panel>().FirstOrDefault(x => x.Dock == DockStyle.Bottom);
        return (ClientSize, MinimumSize, _channelGrid?.Height ?? 0, _show.Height, _hide.Height, _name.Width, footer?.Height ?? 0, _cancel.Text);
    }

    internal (string[] Labels, bool SingleLineShow, bool SingleLineHide, bool HasScrollableChannelList) GetV124ChannelEditorForSelfTest() =>
        (_channelChecks.Select(x => x.Text).ToArray(), !_show.Multiline, !_hide.Multiline, Controls.OfType<CheckedListBox>().Any());
}
