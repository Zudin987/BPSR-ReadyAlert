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
        ChatUiTheme.ApplyForm(this);
        Text = isNew ? "Add Chat Tab" : $"Edit Chat Tab — {tab.Name}";
        StartPosition = FormStartPosition.CenterParent;
        FormBorderStyle = FormBorderStyle.Sizable;
        MaximizeBox = true;
        MinimizeBox = false;
        ShowInTaskbar = false;
        ClientSize = new Size(720, 640);
        MinimumSize = new Size(620, 500);

        var footer = BuildFooter();
        var scroll = new ChatBufferedPanel
        {
            Dock = DockStyle.Fill,
            AutoScroll = true,
            BackColor = ChatUiTheme.Window,
            Padding = new Padding(18, 16, 18, 20)
        };
        var stack = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            ColumnCount = 1,
            RowCount = 0,
            BackColor = ChatUiTheme.Window,
            Margin = Padding.Empty,
            Padding = Padding.Empty
        };
        stack.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));

        AddStack(stack, BuildHeader(
            isNew ? "Create chat tab" : "Edit chat tab",
            "Choose channels and optional filters."));
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
            Height = 60,
            BackColor = ChatUiTheme.Surface,
            Padding = Padding.Empty
        };

        var root = new TableLayoutPanel
        {
            Dock = DockStyle.Fill,
            ColumnCount = 1,
            RowCount = 2,
            Margin = Padding.Empty,
            Padding = Padding.Empty,
            BackColor = ChatUiTheme.Surface
        };
        root.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        root.RowStyles.Add(new RowStyle(SizeType.Absolute, 1F));
        root.RowStyles.Add(new RowStyle(SizeType.Percent, 100F));
        root.Controls.Add(new Panel { Dock = DockStyle.Fill, BackColor = ChatUiTheme.Border, Margin = Padding.Empty }, 0, 0);

        var actions = new TableLayoutPanel
        {
            Dock = DockStyle.Fill,
            ColumnCount = 5,
            RowCount = 1,
            Margin = Padding.Empty,
            Padding = new Padding(14, 10, 14, 10),
            BackColor = ChatUiTheme.Surface
        };
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 96F));
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 84F));
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 8F));
        actions.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 104F));
        actions.RowStyles.Add(new RowStyle(SizeType.Percent, 100F));

        _applyStatus.Dock = DockStyle.Fill;
        _applyStatus.TextAlign = ContentAlignment.MiddleRight;
        _applyStatus.ForeColor = ChatUiTheme.Success;
        _applyStatus.Font = ChatUiTheme.UiFont(8.5F, FontStyle.Bold);

        _save.Text = "Save tab";
        _save.Dock = DockStyle.Fill;
        _save.Margin = Padding.Empty;
        ChatUiTheme.StylePrimaryButton(_save);
        _save.Click += (_, _) => ApplyTab();

        _cancel.Text = "Cancel";
        _cancel.Dock = DockStyle.Fill;
        _cancel.DialogResult = DialogResult.Cancel;
        _cancel.Margin = Padding.Empty;
        ChatUiTheme.StyleSecondaryButton(_cancel);
        _cancel.Margin = Padding.Empty;

        actions.Controls.Add(_applyStatus, 1, 0);
        actions.Controls.Add(_cancel, 2, 0);
        actions.Controls.Add(_save, 4, 0);
        root.Controls.Add(actions, 0, 1);
        footer.Controls.Add(root);
        return footer;
    }

    private static Control BuildHeader(string title, string subtitle)
    {
        var panel = new Panel { AutoSize = true, Dock = DockStyle.Top, Margin = new Padding(0, 0, 0, 12) };
        var flow = new FlowLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            FlowDirection = FlowDirection.TopDown,
            WrapContents = false,
            Margin = Padding.Empty,
            Padding = Padding.Empty
        };
        flow.Controls.Add(ChatUiTheme.Heading(title, 16F));
        flow.Controls.Add(ChatUiTheme.Subheading(subtitle));
        panel.Controls.Add(flow);
        return panel;
    }

    private ChatCardPanel BuildBasicsCard(ChatTabSettings tab)
    {
        _name.Text = tab.Name;
        _name.MaxLength = 40;
        _name.Width = 320;
        _name.MaximumSize = new Size(320, 0);
        ChatUiTheme.StyleTextBox(_name);

        _minLevel.Minimum = 1;
        _minLevel.Maximum = 100;
        _minLevel.Value = Math.Clamp(tab.MinLevel, 1, 100);
        _minLevel.Width = 100;
        ChatUiTheme.StyleNumeric(_minLevel);

        var table = MakeFieldTable();
        AddFieldRow(table, "Tab name", "Shown on the overlay.", _name, 0);
        AddFieldRow(table, "Minimum level", "Hide lower-level senders when known.", _minLevel, 1);
        return MakeCard("Basics", "Name and level filter.", table);
    }

    private ChatCardPanel BuildChannelsCard(ChatTabSettings tab)
    {
        _channels.CheckOnClick = true;
        _channels.BackColor = ChatUiTheme.Input;
        _channels.ForeColor = ChatUiTheme.Text;
        _channels.BorderStyle = BorderStyle.FixedSingle;
        _channels.Height = 150;
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
            Margin = new Padding(0, 8, 0, 0)
        };
        var selectAll = new Button { Text = "Select all", Width = 88, Height = 34 };
        var clear = new Button { Text = "Clear", Width = 72, Height = 34 };
        ChatUiTheme.StyleSecondaryButton(selectAll);
        ChatUiTheme.StyleSecondaryButton(clear);
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
            Padding = Padding.Empty
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
            Padding = Padding.Empty
        };
        fields.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));

        AddStack(fields, MakeFieldBlock(
            "Show only if",
            "Empty = show everything that passes channel and level.",
            _show));
        AddStack(fields, MakeFieldBlock(
            "Hide if",
            "Matching messages are removed after the Show rule.",
            _hide));

        var syntax = new Panel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            BackColor = ChatUiTheme.SurfaceRaised,
            Padding = new Padding(10, 8, 10, 8),
            Margin = new Padding(0, 2, 0, 8)
        };
        var syntaxText = ChatUiTheme.Hint("Examples: serum | food | raid  •  boss AND hard  •  regex: (raid|dungeon)");
        syntaxText.Dock = DockStyle.Top;
        syntaxText.MaximumSize = new Size(560, 0);
        syntax.Controls.Add(syntaxText);
        AddStack(fields, syntax);

        _validation.AutoSize = true;
        _validation.Font = ChatUiTheme.UiFont(8.5F, FontStyle.Bold);
        _validation.Margin = new Padding(0, 0, 0, 0);
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
            Padding = Padding.Empty
        };
        table.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 170F));
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
            Padding = new Padding(0, 3, 12, 8),
            Margin = Padding.Empty
        };
        labelBox.Controls.Add(ChatUiTheme.FieldLabel(label));
        var hintLabel = ChatUiTheme.Hint(hint);
        hintLabel.MaximumSize = new Size(155, 0);
        hintLabel.Margin = new Padding(0, 2, 0, 0);
        labelBox.Controls.Add(hintLabel);

        var host = new Panel
        {
            Dock = DockStyle.Fill,
            Height = 44,
            Padding = new Padding(0, 3, 0, 8),
            Margin = Padding.Empty
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
            Margin = new Padding(0, 0, 0, 10),
            Padding = Padding.Empty
        };
        block.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        var labelControl = ChatUiTheme.FieldLabel(label);
        var hintControl = ChatUiTheme.Hint(hint);
        hintControl.Margin = new Padding(0, 2, 0, 6);
        control.Dock = DockStyle.Top;
        control.Margin = Padding.Empty;
        block.Controls.Add(labelControl, 0, 0);
        block.Controls.Add(hintControl, 0, 1);
        block.Controls.Add(control, 0, 2);
        return block;
    }

    private static ChatCardPanel MakeCard(string title, string subtitle, Control content)
    {
        var card = new ChatCardPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            Padding = new Padding(14),
            Margin = new Padding(0, 0, 0, 10)
        };
        var stack = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            ColumnCount = 1,
            RowCount = 3,
            Margin = Padding.Empty,
            Padding = Padding.Empty
        };
        stack.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        var titleLabel = ChatUiTheme.Heading(title, 10.5F);
        var subtitleLabel = ChatUiTheme.Subheading(subtitle);
        subtitleLabel.Margin = new Padding(0, 2, 0, 8);
        subtitleLabel.MaximumSize = new Size(520, 0);
        content.Dock = DockStyle.Top;
        stack.Controls.Add(titleLabel, 0, 0);
        stack.Controls.Add(subtitleLabel, 0, 1);
        stack.Controls.Add(content, 0, 2);
        card.Controls.Add(stack);
        return card;
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
        box.Height = 64;
        ChatUiTheme.StyleTextBox(box, multiline: true);
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
            _applyStatus.ForeColor = _everSaved ? ChatUiTheme.Success : ChatUiTheme.Muted;
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
