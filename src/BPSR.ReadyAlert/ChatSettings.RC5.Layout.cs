using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    private static Panel CreatePage(string title, string subtitle)
    {
        var page = new Panel
        {
            Dock = DockStyle.Fill,
            AutoScroll = true,
            BackColor = ChatUiTheme.Window,
            Padding = new Padding(28, 24, 28, 28)
        };
        var stack = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            ColumnCount = 1,
            RowCount = 0,
            Margin = Padding.Empty,
            Padding = Padding.Empty
        };
        stack.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        var header = new FlowLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            FlowDirection = FlowDirection.TopDown,
            WrapContents = false,
            Margin = new Padding(0, 0, 0, 20),
            Padding = Padding.Empty
        };
        header.Controls.Add(ChatUiTheme.Heading(title, 18F));
        header.Controls.Add(ChatUiTheme.Subheading(subtitle));
        AddPageCard(stack, header);
        page.Controls.Add(stack);
        page.Tag = stack;
        return page;
    }

    private static void AddPageCard(TableLayoutPanel stack, Control control)
    {
        var row = stack.RowCount++;
        stack.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        control.Dock = DockStyle.Top;
        stack.Controls.Add(control, 0, row);
    }

    private static ChatCardPanel MakeCard(string title, string subtitle, Control content)
    {
        var card = new ChatCardPanel { Dock = DockStyle.Top, AutoSize = true, AutoSizeMode = AutoSizeMode.GrowAndShrink };
        var stack = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
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
        table.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 230F));
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
            Padding = new Padding(0, 5, 18, 14),
            Margin = Padding.Empty
        };
        labelBox.Controls.Add(ChatUiTheme.FieldLabel(label));
        labelBox.Controls.Add(ChatUiTheme.Hint(hint));
        var host = new Panel { Dock = DockStyle.Fill, Height = 54, Padding = new Padding(0, 4, 0, 14), Margin = Padding.Empty };
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
            Margin = new Padding(0, 0, 0, 14),
            Padding = Padding.Empty
        };
        flow.Controls.Add(ChatUiTheme.FieldLabel(label));
        flow.Controls.Add(ChatUiTheme.Hint(hint));
        control.Width = 660;
        control.Margin = new Padding(0, 8, 0, 0);
        flow.Controls.Add(control);
        return flow;
    }

    private static Control MakeSliderRow(string label, string hint, TrackBar slider, Label value, int current, int minimum)
    {
        slider.Minimum = minimum;
        slider.Maximum = 100;
        slider.TickFrequency = 10;
        slider.Value = Math.Clamp(current, minimum, 100);
        slider.Dock = DockStyle.Fill;
        slider.AutoSize = false;
        slider.Height = 34;
        value.Text = slider.Value + "%";
        value.ForeColor = ChatUiTheme.Text;
        value.Font = ChatUiTheme.UiFont(9F, FontStyle.Bold);
        value.TextAlign = ContentAlignment.MiddleRight;
        slider.ValueChanged += (_, _) => value.Text = slider.Value + "%";

        var row = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            ColumnCount = 3,
            RowCount = 1,
            Margin = new Padding(0, 0, 0, 10),
            Padding = Padding.Empty
        };
        row.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 190F));
        row.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        row.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 58F));
        var labels = new FlowLayoutPanel { Dock = DockStyle.Fill, AutoSize = true, FlowDirection = FlowDirection.TopDown, WrapContents = false, Padding = new Padding(0, 2, 12, 0) };
        labels.Controls.Add(ChatUiTheme.FieldLabel(label));
        labels.Controls.Add(ChatUiTheme.Hint(hint));
        row.Controls.Add(labels, 0, 0);
        row.Controls.Add(slider, 1, 0);
        row.Controls.Add(value, 2, 0);
        return row;
    }

    private static Panel MakeInfoBanner(string title, string text, Color accent)
    {
        var panel = new Panel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            BackColor = ChatUiTheme.SurfaceRaised,
            Padding = new Padding(14, 12, 14, 12),
            Margin = new Padding(0, 10, 0, 0)
        };
        var stripe = new Panel { Dock = DockStyle.Left, Width = 3, BackColor = accent };
        var content = new FlowLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            FlowDirection = FlowDirection.TopDown,
            WrapContents = false,
            Padding = new Padding(10, 0, 0, 0)
        };
        content.Controls.Add(ChatUiTheme.FieldLabel(title));
        content.Controls.Add(ChatUiTheme.Hint(text));
        panel.Controls.Add(content);
        panel.Controls.Add(stripe);
        return panel;
    }

    private static Control MakeActionRow(string title, string description, string buttonText, Action action)
    {
        var row = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            ColumnCount = 2,
            RowCount = 1,
            Margin = new Padding(0, 0, 0, 10),
            Padding = new Padding(0, 4, 0, 4)
        };
        row.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        row.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 132F));
        var text = new FlowLayoutPanel { Dock = DockStyle.Fill, AutoSize = true, FlowDirection = FlowDirection.TopDown, WrapContents = false, Padding = new Padding(0, 0, 16, 0) };
        text.Controls.Add(ChatUiTheme.FieldLabel(title));
        text.Controls.Add(ChatUiTheme.Hint(description));
        var button = new Button { Text = buttonText, Width = 118, Height = 34, Dock = DockStyle.Top };
        ChatUiTheme.StyleSecondaryButton(button);
        button.Click += (_, _) => action();
        row.Controls.Add(text, 0, 0);
        row.Controls.Add(button, 1, 0);
        return row;
    }

    private static Control MakePathRow(TextBox pathBox, Action browse, string label, string hint)
    {
        var outer = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            ColumnCount = 1,
            RowCount = 2,
            Margin = new Padding(0, 8, 0, 8),
            Padding = Padding.Empty
        };
        outer.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        var labelFlow = new FlowLayoutPanel { Dock = DockStyle.Top, AutoSize = true, FlowDirection = FlowDirection.TopDown, WrapContents = false };
        labelFlow.Controls.Add(ChatUiTheme.FieldLabel(label));
        labelFlow.Controls.Add(ChatUiTheme.Hint(hint));

        var row = new TableLayoutPanel { Dock = DockStyle.Top, Height = 38, ColumnCount = 2, Margin = new Padding(0, 8, 0, 0) };
        row.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        row.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 112F));
        pathBox.Dock = DockStyle.Fill;
        var browseButton = new Button { Text = "Browse…", Dock = DockStyle.Fill, Margin = new Padding(8, 0, 0, 0) };
        ChatUiTheme.StyleSecondaryButton(browseButton);
        browseButton.Click += (_, _) => browse();
        row.Controls.Add(pathBox, 0, 0);
        row.Controls.Add(browseButton, 1, 0);
        outer.Controls.Add(labelFlow, 0, 0);
        outer.Controls.Add(row, 0, 1);
        return outer;
    }

    private static void AddStack(TableLayoutPanel stack, Control control)
    {
        var row = stack.RowCount++;
        stack.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        control.Dock = DockStyle.Top;
        stack.Controls.Add(control, 0, row);
    }
}
