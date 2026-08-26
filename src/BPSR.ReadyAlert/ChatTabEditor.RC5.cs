using System.Drawing;
using System.Drawing.Text;
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
    private readonly TextBox _name = new();
    private readonly CheckedListBox _channels = new();
    private readonly NumericUpDown _minLevel = new();
    private readonly TextBox _show = new();
    private readonly TextBox _hide = new();
    private readonly Label _validation = new();
    private readonly Button _save = new();

    internal ChatTabEditorForm(ChatTabSettings tab, bool isNew)
    {
        _tab = tab;
        ChatUiTheme.ApplyForm(this);
        Text = isNew ? "Add Chat Tab" : $"Edit Chat Tab — {tab.Name}";
        StartPosition = FormStartPosition.CenterParent;
        FormBorderStyle = FormBorderStyle.Sizable;
        MaximizeBox = true;
        MinimizeBox = false;
        ShowInTaskbar = false;
        ClientSize = new Size(820, 760);
        MinimumSize = new Size(720, 620);

        var footer = BuildFooter();
        var scroll = new Panel
        {
            Dock = DockStyle.Fill,
            AutoScroll = true,
            BackColor = ChatUiTheme.Window,
            Padding = new Padding(24, 22, 24, 26)
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
            isNew ? "Create a chat tab" : "Edit chat tab",
            "Choose what this tab shows. You can change it again later by right-clicking the tab."));
        AddStack(stack, BuildBasicsCard(tab));
        AddStack(stack, BuildChannelsCard(tab));
        AddStack(stack, BuildFiltersCard(tab));

        scroll.Controls.Add(stack);
        Controls.Add(scroll);
        Controls.Add(footer);

        AcceptButton = _save;
        _show.TextChanged += (_, _) => RefreshValidation();
        _hide.TextChanged += (_, _) => RefreshValidation();
        RefreshValidation();
    }

    private Panel BuildFooter()
    {
        var footer = new Panel
        {
            Dock = DockStyle.Bottom,
            Height = 64,
            Padding = new Padding(18, 14, 18, 14),
            BackColor = ChatUiTheme.Surface
        };
        footer.Controls.Add(ChatUiTheme.Divider());

        var buttons = new FlowLayoutPanel
        {
            Dock = DockStyle.Fill,
            FlowDirection = FlowDirection.RightToLeft,
            WrapContents = false,
            BackColor = ChatUiTheme.Surface,
            Padding = Padding.Empty,
            Margin = Padding.Empty
        };

        _save.Text = "Save tab";
        _save.Width = 112;
        _save.Height = 36;
        ChatUiTheme.StylePrimaryButton(_save);
        _save.Click += (_, _) => SaveAndClose();

        var cancel = new Button
        {
            Text = "Cancel",
            Width = 96,
            Height = 36,
            DialogResult = DialogResult.Cancel,
            Margin = new Padding(0, 0, 8, 0)
        };
        ChatUiTheme.StyleSecondaryButton(cancel);
        buttons.Controls.Add(_save);
        buttons.Controls.Add(cancel);
        footer.Controls.Add(buttons);
        CancelButton = cancel;
        return footer;
    }

    private static Control BuildHeader(string title, string subtitle)
    {
        var panel = new Panel { AutoSize = true, Dock = DockStyle.Top, Margin = new Padding(0, 0, 0, 18) };
        var flow = new FlowLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            FlowDirection = FlowDirection.TopDown,
            WrapContents = false,
            Margin = Padding.Empty,
            Padding = Padding.Empty
        };
        flow.Controls.Add(ChatUiTheme.Heading(title, 17F));
        flow.Controls.Add(ChatUiTheme.Subheading(subtitle));
        panel.Controls.Add(flow);
        return panel;
    }

    private ChatCardPanel BuildBasicsCard(ChatTabSettings tab)
    {
        _name.Text = tab.Name;
        _name.MaxLength = 40;
        ChatUiTheme.StyleTextBox(_name);
        _minLevel.Minimum = 1;
        _minLevel.Maximum = 100;
        _minLevel.Value = Math.Clamp(tab.MinLevel, 1, 100);
        _minLevel.Width = 110;
        ChatUiTheme.StyleNumeric(_minLevel);

        var table = MakeFieldTable();
        AddFieldRow(table, "Tab name", "Short name shown on the overlay.", _name, 0);
        AddFieldRow(table, "Minimum player level", "Messages from lower-level players are hidden when their level is known.", _minLevel, 1);
        return MakeCard("Basics", "Name the tab and choose its player-level floor.", table);
    }

    private ChatCardPanel BuildChannelsCard(ChatTabSettings tab)
    {
        _channels.CheckOnClick = true;
        _channels.BackColor = ChatUiTheme.Input;
        _channels.ForeColor = ChatUiTheme.Text;
        _channels.BorderStyle = BorderStyle.FixedSingle;
        _channels.Height = 210;
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
            Margin = new Padding(0, 10, 0, 0)
        };
        var selectAll = new Button { Text = "Select all", Width = 100, Height = 32 };
        var clear = new Button { Text = "Clear", Width = 82, Height = 32 };
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
        return MakeCard("Channels", "Check every BPSR chat channel that belongs in this tab.", content);
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
            "Show only if message matches",
            "Leave empty to show every message that passes the channel and level rules.",
            _show));
        AddStack(fields, MakeFieldBlock(
            "Hide if message matches",
            "Anything matching this rule is removed even if it passes the Show rule.",
            _hide));

        var syntax = new Panel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            BackColor = ChatUiTheme.SurfaceRaised,
            Padding = new Padding(12),
            Margin = new Padding(0, 4, 0, 10)
        };
        var syntaxText = ChatUiTheme.Hint(
            "Quick examples:  serum | food | raid   •   one pattern per line = OR   •   boss AND hard\r\n" +
            "Matching ignores letter case. Advanced regex is also supported, for example (raid|dungeon). Invalid regex is blocked safely.");
        syntaxText.Dock = DockStyle.Top;
        syntax.Controls.Add(syntaxText);
        AddStack(fields, syntax);

        _validation.AutoSize = true;
        _validation.Font = ChatUiTheme.UiFont(9F, FontStyle.Bold);
        _validation.Margin = new Padding(0, 2, 0, 0);
        AddStack(fields, _validation);

        return MakeCard("Filters", "Optional rules for finding useful chat without making users write complicated regex.", fields);
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
        table.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 220F));
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
            Padding = new Padding(0, 5, 16, 14),
            Margin = Padding.Empty
        };
        labelBox.Controls.Add(ChatUiTheme.FieldLabel(label));
        labelBox.Controls.Add(ChatUiTheme.Hint(hint));

        var host = new Panel { Dock = DockStyle.Fill, Height = 52, Padding = new Padding(0, 4, 0, 14), Margin = Padding.Empty };
        control.Anchor = AnchorStyles.Left | AnchorStyles.Right | AnchorStyles.Top;
        if (control is TextBox) control.Width = 430;
        host.Controls.Add(control);

        table.Controls.Add(labelBox, 0, row);
        table.Controls.Add(host, 1, row);
    }

    private static Control MakeFieldBlock(string label, string hint, Control control)
    {
        var flow = new FlowLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            FlowDirection = FlowDirection.TopDown,
            WrapContents = false,
            Margin = new Padding(0, 0, 0, 16),
            Padding = Padding.Empty
        };
        flow.Controls.Add(ChatUiTheme.FieldLabel(label));
        flow.Controls.Add(ChatUiTheme.Hint(hint));
        control.Margin = new Padding(0, 8, 0, 0);
        control.Width = 690;
        flow.Controls.Add(control);
        return flow;
    }

    private static ChatCardPanel MakeCard(string title, string subtitle, Control content)
    {
        var card = new ChatCardPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink
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
        var titleLabel = ChatUiTheme.Heading(title, 11F);
        var subtitleLabel = ChatUiTheme.Subheading(subtitle);
        subtitleLabel.Margin = new Padding(0, 4, 0, 14);
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
        box.Height = 96;
        ChatUiTheme.StyleTextBox(box, multiline: true);
    }

    private void RefreshValidation()
    {
        if (!ChatFilterExpression.TryValidate(_show.Text, out var showError))
        {
            _validation.ForeColor = ChatUiTheme.Danger;
            _validation.Text = "● Show rule needs attention: " + showError;
            _save.Enabled = false;
            return;
        }
        if (!ChatFilterExpression.TryValidate(_hide.Text, out var hideError))
        {
            _validation.ForeColor = ChatUiTheme.Danger;
            _validation.Text = "● Hide rule needs attention: " + hideError;
            _save.Enabled = false;
            return;
        }

        _validation.ForeColor = ChatUiTheme.Success;
        _validation.Text = "● Filters are valid — matching is case-insensitive.";
        _save.Enabled = true;
    }

    private void SaveAndClose()
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
        DialogResult = DialogResult.OK;
        Close();
    }
}
