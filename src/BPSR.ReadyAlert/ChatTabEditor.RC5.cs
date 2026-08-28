using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed class ChatTabEditorForm : Form
{
    private static readonly (string Label, ChatChannel Channel)[] ChannelChoices =
    [
        ("Null / Other", ChatChannel.Null),
        ("World", ChatChannel.World),
        ("Local / Scene", ChatChannel.Local),
        ("Group", ChatChannel.Group),
        ("Team", ChatChannel.Team),
        ("Private", ChatChannel.Private),
        ("Union / Guild", ChatChannel.Union),
        ("System", ChatChannel.System),
        ("Top Notice", ChatChannel.TopNotice),
        ("Newbie", ChatChannel.Newbie),
        ("Play", ChatChannel.Play)
    ];

    private readonly ChatTabSettings _tab;
    private bool _isNew;
    private readonly TextBox _name = new();
    private readonly CheckedListBox _channels = new();
    private readonly NumericUpDown _minLevel = new();
    private readonly TextBox _show = new();
    private readonly TextBox _hide = new();
    private readonly Label _validation = new();
    private readonly Label _applyStatus = new();
    private readonly Button _save = new();
    private readonly Button _cancel = new();
    private string _savedFingerprint = string.Empty;
    private bool _trackingReady;
    private bool _everSaved;
    private bool _appliedNotPersisted;

    internal ChatTabEditorForm(ChatTabSettings tab, bool isNew)
    {
        _tab = tab;
        _isNew = isNew;
        ChatUiTheme.ApplySettingsForm(this);
        Text = isNew ? "Add Chat Tab" : $"Edit Chat Tab — {tab.Name}";
        StartPosition = FormStartPosition.CenterParent;
        FormBorderStyle = FormBorderStyle.Sizable;
        MaximizeBox = true;
        MinimizeBox = false;
        ShowInTaskbar = false;
        ClientSize = new Size(640, 570);
        MinimumSize = new Size(540, 430);

        var footer = BuildFooter();
        var scroll = new ChatBufferedPanel
        {
            Dock = DockStyle.Fill,
            AutoScroll = true,
            BackColor = ChatUiTheme.SettingsSurface,
            Padding = new Padding(7)
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
            Padding = new Padding(8, 7, 8, 10)
        };
        stack.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));

        AddStack(stack, BuildHeader(
            isNew ? "Create chat tab" : "Edit chat tab",
            "Channels and optional filters."));
        AddStack(stack, BuildBasicsCard(tab));
        AddStack(stack, BuildChannelsCard(tab));
        AddStack(stack, BuildFiltersCard(tab));

        scroll.Controls.Add(stack);
        Controls.Add(scroll);
        Controls.Add(footer);

        AcceptButton = _save;
        CancelButton = _cancel;

        _show.TextChanged += (_, _) =>
        {
            RefreshValidation();
            RefreshDirtyState();
        };
        _hide.TextChanged += (_, _) =>
        {
            RefreshValidation();
            RefreshDirtyState();
        };
        _name.TextChanged += (_, _) => RefreshDirtyState();
        _minLevel.ValueChanged += (_, _) => RefreshDirtyState();
        _channels.ItemCheck += (_, _) =>
        {
            if (!IsHandleCreated || IsDisposed) return;
            BeginInvoke(new Action(RefreshDirtyState));
        };
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
        var footer = new Panel
        {
            Dock = DockStyle.Bottom,
            Height = 52,
            BackColor = ChatUiTheme.SettingsWindow,
            Padding = Padding.Empty
        };
        var line = new Panel { Dock = DockStyle.Top, Height = 1, BackColor = ChatUiTheme.SettingsBorder };
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
        actions.RowStyles.Add(new RowStyle(SizeType.Percent, 100F));

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
        footer.Controls.Add(line);
        return footer;
    }

    private static Control BuildHeader(string title, string subtitle)
    {
        var row = new FlowLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            FlowDirection = FlowDirection.TopDown,
            WrapContents = false,
            Margin = new Padding(0, 0, 0, 7),
            Padding = Padding.Empty,
            BackColor = ChatUiTheme.SettingsWindow
        };
        row.Controls.Add(new Label
        {
            AutoSize = true,
            Text = title,
            ForeColor = ChatUiTheme.SettingsText,
            Font = ChatUiTheme.UiFont(10F, FontStyle.Bold),
            Margin = Padding.Empty
        });
        var hint = ChatUiTheme.SettingsHint(subtitle);
        hint.Margin = new Padding(18, 1, 0, 0);
        row.Controls.Add(hint);
        return row;
    }

    private ChatCardPanel BuildBasicsCard(ChatTabSettings tab)
    {
        _name.Text = tab.Name;
        _name.MaxLength = 40;
        _name.Width = 260;
        _name.MaximumSize = new Size(260, 0);
        ChatUiTheme.StyleSettingsTextBox(_name);

        _minLevel.Minimum = 1;
        _minLevel.Maximum = 100;
        _minLevel.Value = Math.Clamp(tab.MinLevel, 1, 100);
        _minLevel.Width = 82;
        ChatUiTheme.StyleSettingsNumeric(_minLevel);

        var table = MakeFieldTable();
        AddFieldRow(table, "Tab name", "Shown on the overlay.", _name, 0);
        AddFieldRow(table, "Minimum level", "Hide lower-level senders when known.", _minLevel, 1);
        return MakeCard("Basics", "Name and level filter.", table);
    }

    private ChatCardPanel BuildChannelsCard(ChatTabSettings tab)
    {
        _channels.CheckOnClick = true;
        _channels.BackColor = ChatUiTheme.SettingsInput;
        _channels.ForeColor = ChatUiTheme.SettingsText;
        _channels.BorderStyle = BorderStyle.FixedSingle;
        _channels.Height = 128;
        _channels.Dock = DockStyle.Top;
        _channels.IntegralHeight = false;

        for (var i = 0; i < ChannelChoices.Length; i++)
        {
            _channels.Items.Add(ChannelChoices[i].Label);
            if (tab.Channels.Contains((int)ChannelChoices[i].Channel))
                _channels.SetItemChecked(i, true);
        }

        var quick = new FlowLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            WrapContents = false,
            FlowDirection = FlowDirection.LeftToRight,
            Margin = new Padding(0, 5, 0, 0),
            BackColor = ChatUiTheme.SettingsWindow
        };
        var selectAll = new Button { Text = "Select all", Width = 82, Height = 28 };
        var clear = new Button { Text = "Clear", Width = 64, Height = 28 };
        ChatUiTheme.StyleSettingsButton(selectAll);
        ChatUiTheme.StyleSettingsButton(clear);
        selectAll.Click += (_, _) =>
        {
            for (var i = 0; i < _channels.Items.Count; i++) _channels.SetItemChecked(i, true);
        };
        clear.Click += (_, _) =>
        {
            for (var i = 0; i < _channels.Items.Count; i++) _channels.SetItemChecked(i, false);
        };
        quick.Controls.Add(selectAll);
        quick.Controls.Add(clear);

        var content = new TableLayoutPanel
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
        content.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        content.Controls.Add(_channels, 0, 0);
        content.Controls.Add(quick, 0, 1);
        return MakeCard("Channels", "Choose chat channels.", content);
    }

    private ChatCardPanel BuildFiltersCard(ChatTabSettings tab)
    {
        ConfigureFilterBox(_show, tab.ShowIfMatches);
        ConfigureFilterBox(_hide, tab.HideIfMatches);

        var fields = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            ColumnCount = 1,
            RowCount = 0,
            Margin = Padding.Empty,
            Padding = Padding.Empty,
            BackColor = ChatUiTheme.SettingsWindow
        };
        fields.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));

        AddStack(fields, MakeFieldBlock(
            "Show only if",
            "Empty = show all messages that pass channel and level.",
            _show));
        AddStack(fields, MakeFieldBlock(
            "Hide if",
            "Applied after the Show rule.",
            _hide));

        var syntaxText = ChatUiTheme.SettingsHint("Examples: serum | food | raid  •  boss AND hard  •  regex: (raid|dungeon)");
        syntaxText.Margin = new Padding(18, 0, 0, 4);
        AddStack(fields, syntaxText);

        _validation.AutoSize = true;
        _validation.Font = ChatUiTheme.UiFont(8.5F, FontStyle.Bold);
        _validation.Margin = new Padding(18, 0, 0, 0);
        AddStack(fields, _validation);

        return MakeCard("Filters", "Optional message filters.", fields);
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
        table.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 160F));
        table.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        return table;
    }

    private static void AddFieldRow(TableLayoutPanel table, string label, string hint, Control control, int row)
    {
        table.RowCount = Math.Max(table.RowCount, row + 1);
        table.RowStyles.Add(new RowStyle(SizeType.AutoSize));

        var labelBox = new FlowLayoutPanel
        {
            Dock = DockStyle.Fill,
            AutoSize = true,
            FlowDirection = FlowDirection.TopDown,
            WrapContents = false,
            Padding = new Padding(0, 3, 10, 4),
            Margin = Padding.Empty,
            BackColor = ChatUiTheme.SettingsWindow
        };
        labelBox.Controls.Add(ChatUiTheme.SettingsFieldLabel(label));
        var hintLabel = ChatUiTheme.SettingsHint(hint);
        hintLabel.MaximumSize = new Size(145, 0);
        hintLabel.Margin = new Padding(18, 1, 0, 0);
        labelBox.Controls.Add(hintLabel);

        var host = new Panel
        {
            Dock = DockStyle.Fill,
            Height = 34,
            Padding = new Padding(0, 2, 0, 4),
            Margin = Padding.Empty,
            BackColor = ChatUiTheme.SettingsWindow
        };
        control.Dock = DockStyle.None;
        control.Anchor = AnchorStyles.Top | AnchorStyles.Left;
        control.Margin = Padding.Empty;
        host.Controls.Add(control);

        table.Controls.Add(labelBox, 0, row);
        table.Controls.Add(host, 1, row);
    }

    private static Control MakeFieldBlock(string label, string hint, Control control)
    {
        var block = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            ColumnCount = 1,
            RowCount = 3,
            Margin = new Padding(0, 0, 0, 6),
            Padding = Padding.Empty,
            BackColor = ChatUiTheme.SettingsWindow
        };
        block.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        var labelControl = ChatUiTheme.SettingsFieldLabel(label);
        var hintControl = ChatUiTheme.SettingsHint(hint);
        hintControl.Margin = new Padding(18, 1, 0, 4);
        control.Dock = DockStyle.Top;
        control.Margin = new Padding(18, 0, 0, 0);
        block.Controls.Add(labelControl, 0, 0);
        block.Controls.Add(hintControl, 0, 1);
        block.Controls.Add(control, 0, 2);
        return block;
    }

    private static ChatCardPanel MakeCard(string title, string subtitle, Control content)
    {
        var section = new ChatCardPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            Padding = Padding.Empty,
            Margin = new Padding(0, 0, 0, 7)
        };
        var stack = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            ColumnCount = 1,
            RowCount = 0,
            Margin = Padding.Empty,
            Padding = Padding.Empty,
            BackColor = ChatUiTheme.SettingsWindow
        };
        stack.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));

        var heading = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            ColumnCount = 2,
            RowCount = 1,
            Margin = new Padding(0, 0, 0, 4),
            Padding = Padding.Empty,
            BackColor = ChatUiTheme.SettingsWindow
        };
        heading.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
        heading.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        var titleLabel = ChatUiTheme.SettingsFieldLabel(title);
        titleLabel.Padding = new Padding(0, 0, 7, 0);
        var lineHost = new Panel { Dock = DockStyle.Fill, Height = 18, BackColor = ChatUiTheme.SettingsWindow };
        lineHost.Padding = new Padding(0, 8, 0, 0);
        lineHost.Controls.Add(new Panel { Dock = DockStyle.Top, Height = 1, BackColor = ChatUiTheme.SettingsBorder });
        heading.Controls.Add(titleLabel, 0, 0);
        heading.Controls.Add(lineHost, 1, 0);
        AddStack(stack, heading);

        var subtitleLabel = ChatUiTheme.SettingsHint(subtitle);
        subtitleLabel.Margin = new Padding(18, 0, 0, 5);
        subtitleLabel.MaximumSize = new Size(500, 0);
        AddStack(stack, subtitleLabel);

        content.Dock = DockStyle.Top;
        AddStack(stack, content);
        section.Controls.Add(stack);
        return section;
    }

    private static void AddStack(TableLayoutPanel stack, Control control)
    {
        var row = stack.RowCount++;
        stack.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        control.Dock = DockStyle.Top;
        stack.Controls.Add(control, 0, row);
    }

    private static void ConfigureFilterBox(TextBox box, string value)
    {
        box.Text = value;
        box.Height = 52;
        ChatUiTheme.StyleSettingsTextBox(box, multiline: true);
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

    private string CaptureFingerprint()
    {
        var channels = string.Join(',', Enumerable.Range(0, _channels.Items.Count)
            .Where(_channels.GetItemChecked));
        return string.Join('|',
            _name.Text,
            _minLevel.Value,
            channels,
            _show.Text,
            _hide.Text);
    }

    private void TabEditorFormClosing(object? sender, FormClosingEventArgs e)
    {
        if (!_trackingReady || e.CloseReason != CloseReason.UserClosing) return;

        var dirty = !string.Equals(CaptureFingerprint(), _savedFingerprint, StringComparison.Ordinal);
        if (!dirty && !_appliedNotPersisted) return;

        var (message, title) = (dirty, _appliedNotPersisted) switch
        {
            (true, true) => (
                "Some tab edits have not been applied, and the last applied tab state could not be saved to disk.\r\n\r\n" +
                "Closing will discard the unapplied edits. The tab already active in this session may also be lost after ReadyAlert restarts. Close anyway?",
                "Tab changes are not safely saved"),
            (false, true) => (
                "This tab is active for the current ReadyAlert session, but Windows could not save it to disk.\r\n\r\n" +
                "Closing keeps it active until ReadyAlert exits, but it may be lost after restart. Close anyway?",
                "Tab is not saved"),
            _ => (
                "Discard changes that have not been saved?",
                "Unsaved tab changes")
        };

        var answer = MessageBox.Show(
            this,
            message,
            title,
            MessageBoxButtons.YesNo,
            MessageBoxIcon.Warning,
            MessageBoxDefaultButton.Button2);
        if (answer != DialogResult.Yes) e.Cancel = true;
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

        var channels = new List<int>();
        for (var i = 0; i < ChannelChoices.Length; i++)
            if (_channels.GetItemChecked(i)) channels.Add((int)ChannelChoices[i].Channel);

        if (channels.Count == 0)
        {
            MessageBox.Show(this, "Select at least one chat channel.", "Chat tab", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            _channels.Focus();
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
        _tab.Channels = channels;
        _tab.MinLevel = (int)_minLevel.Value;
        _tab.ShowIfMatches = _show.Text.Trim();
        _tab.HideIfMatches = _hide.Text.Trim();

        var saveSucceeded = true;
        if (Owner is ChatOverlayForm overlay)
        {
            saveSucceeded = overlay.ApplyTabFromOpenDialog(_tab, _isNew);
            TopMost = overlay.TopMost;
            if (TopMost)
            {
                BringToFront();
                Activate();
            }
        }

        _isNew = false;
        Text = $"Edit Chat Tab — {_tab.Name}";
        _savedFingerprint = CaptureFingerprint();

        if (!saveSucceeded)
        {
            _everSaved = false;
            _appliedNotPersisted = true;
            _applyStatus.Text = "Applied — not saved";
            _applyStatus.ForeColor = ChatUiTheme.Danger;
            MessageBox.Show(
                this,
                "The tab was applied for this ReadyAlert session, but Windows could not save it to disk.\r\n\r\n" +
                "It may be lost after restart. Check folder permissions or disk availability, then press 'Save tab' again.",
                "Tab could not be saved",
                MessageBoxButtons.OK,
                MessageBoxIcon.Warning);
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
        return (
            ClientSize,
            MinimumSize,
            _channels.Height,
            _show.Height,
            _hide.Height,
            _name.Width,
            footer?.Height ?? 0,
            _cancel.Text);
    }
}
