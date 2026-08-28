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
            BackColor = ChatUiTheme.SettingsSurface,
            Padding = new Padding(7)
        };
        page.Paint += (_, e) =>
        {
            var rect = page.ClientRectangle;
            rect.Width -= 1;
            rect.Height -= 1;
            if (rect.Width <= 0 || rect.Height <= 0) return;
            using var pen = new Pen(ChatUiTheme.SettingsBorder);
            e.Graphics.DrawRectangle(pen, rect);
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
            Padding = new Padding(7, 6, 7, 8)
        };
        stack.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
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

    private static ChatSettingsSectionPanel MakeCard(string title, string subtitle, Control content)
    {
        var section = new ChatSettingsSectionPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            Padding = Padding.Empty,
            Margin = new Padding(0, 0, 0, 8)
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

        var heading = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            ColumnCount = 2,
            RowCount = 1,
            Margin = new Padding(0, 1, 0, 5),
            Padding = Padding.Empty,
            BackColor = ChatUiTheme.SettingsWindow
        };
        heading.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
        heading.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        var titleLabel = new Label
        {
            AutoSize = true,
            Text = title,
            ForeColor = ChatUiTheme.SettingsText,
            Font = ChatUiTheme.UiFont(9F),
            Margin = Padding.Empty,
            Padding = new Padding(0, 0, 7, 0)
        };
        var lineHost = new Panel { Dock = DockStyle.Fill, Height = 18, Margin = Padding.Empty, BackColor = ChatUiTheme.SettingsWindow };
        var line = new Panel
        {
            Height = 1,
            Dock = DockStyle.Top,
            Margin = Padding.Empty,
            BackColor = ChatUiTheme.SettingsBorder
        };
        lineHost.Padding = new Padding(0, 8, 0, 0);
        lineHost.Controls.Add(line);
        heading.Controls.Add(titleLabel, 0, 0);
        heading.Controls.Add(lineHost, 1, 0);
        AddStack(stack, heading);

        if (!string.IsNullOrWhiteSpace(subtitle))
        {
            var subtitleLabel = ChatUiTheme.SettingsHint(subtitle);
            subtitleLabel.Margin = new Padding(18, 0, 0, 6);
            subtitleLabel.MaximumSize = new Size(560, 0);
            AddStack(stack, subtitleLabel);
        }

        content.Dock = DockStyle.Top;
        content.Margin = Padding.Empty;
        AddStack(stack, content);
        section.Controls.Add(stack);
        return section;
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
        table.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 210F));
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
            Padding = new Padding(0, 3, 12, 5),
            Margin = Padding.Empty,
            BackColor = ChatUiTheme.SettingsWindow
        };
        labelBox.Controls.Add(ChatUiTheme.SettingsFieldLabel(label));
        if (!string.IsNullOrWhiteSpace(hint))
        {
            var hintLabel = ChatUiTheme.SettingsHint(hint);
            hintLabel.MaximumSize = new Size(195, 0);
            hintLabel.Margin = new Padding(20, 2, 0, 0);
            labelBox.Controls.Add(hintLabel);
        }

        StyleSettingsInput(control);
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
        StyleSettingsInput(control);
        var block = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            ColumnCount = 1,
            RowCount = 0,
            Margin = new Padding(0, 0, 0, 7),
            Padding = Padding.Empty,
            BackColor = ChatUiTheme.SettingsWindow
        };
        block.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));

        var labelControl = ChatUiTheme.SettingsFieldLabel(label);
        labelControl.Margin = new Padding(0, 1, 0, 0);
        AddStack(block, labelControl);
        if (!string.IsNullOrWhiteSpace(hint))
        {
            var hintControl = ChatUiTheme.SettingsHint(hint);
            hintControl.Margin = new Padding(20, 1, 0, 4);
            AddStack(block, hintControl);
        }
        control.Dock = DockStyle.Top;
        control.Margin = new Padding(20, 0, 0, 0);
        AddStack(block, control);
        return block;
    }

    private static Control MakeSliderRow(string label, string hint, TrackBar slider, Label value, int current, int minimum)
    {
        slider.Minimum = minimum;
        slider.Maximum = 100;
        slider.TickFrequency = 10;
        slider.SmallChange = 5;
        slider.LargeChange = 10;
        slider.Value = Math.Clamp(current, minimum, 100);
        slider.Visible = false;
        slider.TabStop = false;
        slider.Width = 1;
        slider.Height = 1;

        var visual = new ChatCompactSlider
        {
            Minimum = minimum,
            Maximum = 100,
            Value = slider.Value,
            Dock = DockStyle.Fill,
            AccessibleName = label,
            AccessibleDescription = hint,
            BackColor = ChatUiTheme.SettingsWindow,
            ForeColor = ChatUiTheme.SettingsText,
            Enabled = slider.Enabled
        };

        value.Text = slider.Value + "%";
        value.ForeColor = ChatUiTheme.SettingsText;
        value.Font = ChatUiTheme.UiFont(9F);
        value.TextAlign = ContentAlignment.MiddleRight;

        var syncing = false;
        visual.ValueChanged += (_, _) =>
        {
            if (syncing || slider.Value == visual.Value) return;
            syncing = true;
            slider.Value = visual.Value;
            syncing = false;
        };
        slider.ValueChanged += (_, _) =>
        {
            if (!syncing && visual.Value != slider.Value)
            {
                syncing = true;
                visual.Value = slider.Value;
                syncing = false;
            }
            value.Text = slider.Value + "%";
        };
        slider.EnabledChanged += (_, _) => visual.Enabled = slider.Enabled;

        var sliderHost = new Panel
        {
            Dock = DockStyle.Fill,
            Height = 24,
            Margin = Padding.Empty,
            BackColor = ChatUiTheme.SettingsWindow
        };
        sliderHost.Controls.Add(visual);
        sliderHost.Controls.Add(slider);

        var row = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            Height = 30,
            ColumnCount = 3,
            RowCount = 1,
            Margin = new Padding(0, 0, 0, 3),
            Padding = Padding.Empty,
            BackColor = ChatUiTheme.SettingsWindow
        };
        row.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 210F));
        row.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        row.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 44F));
        row.RowStyles.Add(new RowStyle(SizeType.Percent, 100F));

        var labels = new Panel
        {
            Dock = DockStyle.Fill,
            Padding = new Padding(0, 5, 12, 0),
            Margin = Padding.Empty,
            BackColor = ChatUiTheme.SettingsWindow
        };
        var labelControl = ChatUiTheme.SettingsFieldLabel(label);
        labelControl.Dock = DockStyle.Top;
        labels.Controls.Add(labelControl);

        row.Controls.Add(labels, 0, 0);
        row.Controls.Add(sliderHost, 1, 0);
        row.Controls.Add(value, 2, 0);
        return row;
    }

    private static Panel MakeInfoBanner(string title, string text, Color accent)
    {
        var panel = new Panel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            BackColor = ChatUiTheme.SettingsWindow,
            Padding = new Padding(0, 2, 0, 5),
            Margin = new Padding(0, 3, 0, 3)
        };
        var content = new FlowLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            FlowDirection = FlowDirection.TopDown,
            WrapContents = false,
            Padding = new Padding(20, 0, 0, 0),
            BackColor = ChatUiTheme.SettingsWindow
        };
        var titleLabel = ChatUiTheme.SettingsFieldLabel(title);
        titleLabel.ForeColor = accent;
        content.Controls.Add(titleLabel);
        var hintLabel = ChatUiTheme.SettingsHint(text);
        hintLabel.MaximumSize = new Size(540, 0);
        hintLabel.Margin = new Padding(0, 1, 0, 0);
        content.Controls.Add(hintLabel);
        panel.Controls.Add(content);
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
            Margin = new Padding(0, 0, 0, 4),
            Padding = new Padding(0, 1, 0, 1),
            BackColor = ChatUiTheme.SettingsWindow
        };
        row.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        row.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 104F));
        var text = new FlowLayoutPanel
        {
            Dock = DockStyle.Fill,
            AutoSize = true,
            FlowDirection = FlowDirection.TopDown,
            WrapContents = false,
            Padding = Padding.Empty,
            BackColor = ChatUiTheme.SettingsWindow
        };
        text.Controls.Add(ChatUiTheme.SettingsFieldLabel(title));
        if (!string.IsNullOrWhiteSpace(description))
        {
            var descriptionLabel = ChatUiTheme.SettingsHint(description);
            descriptionLabel.MaximumSize = new Size(430, 0);
            descriptionLabel.Margin = new Padding(20, 1, 0, 0);
            text.Controls.Add(descriptionLabel);
        }
        var button = new Button { Text = buttonText, Width = 96, Height = 28, Dock = DockStyle.Top, Margin = new Padding(8, 0, 0, 0) };
        ChatUiTheme.StyleSettingsButton(button);
        button.Click += (_, _) => action();
        row.Controls.Add(text, 0, 0);
        row.Controls.Add(button, 1, 0);
        return row;
    }

    private static Control MakePathRow(TextBox pathBox, Action browse, string label, string hint)
    {
        ChatUiTheme.StyleSettingsTextBox(pathBox);
        var outer = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            AutoSize = true,
            ColumnCount = 1,
            RowCount = 0,
            Margin = new Padding(0, 3, 0, 4),
            Padding = Padding.Empty,
            BackColor = ChatUiTheme.SettingsWindow
        };
        outer.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        var labelText = ChatUiTheme.SettingsFieldLabel(label);
        AddStack(outer, labelText);
        if (!string.IsNullOrWhiteSpace(hint))
        {
            var hintText = ChatUiTheme.SettingsHint(hint);
            hintText.Margin = new Padding(20, 1, 0, 3);
            AddStack(outer, hintText);
        }

        var row = new TableLayoutPanel { Dock = DockStyle.Top, Height = 30, ColumnCount = 2, Margin = new Padding(20, 0, 0, 0) };
        row.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100F));
        row.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 92F));
        pathBox.Dock = DockStyle.Fill;
        var browseButton = new Button { Text = "Browse…", Dock = DockStyle.Fill, Margin = new Padding(6, 0, 0, 0) };
        ChatUiTheme.StyleSettingsButton(browseButton);
        browseButton.Click += (_, _) => browse();
        row.Controls.Add(pathBox, 0, 0);
        row.Controls.Add(browseButton, 1, 0);
        AddStack(outer, row);
        return outer;
    }

    private static void StyleSettingsInput(Control control)
    {
        switch (control)
        {
            case TextBox box:
                ChatUiTheme.StyleSettingsTextBox(box, box.Multiline);
                break;
            case ComboBox combo:
                ChatUiTheme.StyleSettingsComboBox(combo);
                break;
            case NumericUpDown numeric:
                ChatUiTheme.StyleSettingsNumeric(numeric);
                break;
            case CheckBox check:
                ChatUiTheme.StyleSettingsCheckBox(check);
                break;
        }
    }

    private static void AddStack(TableLayoutPanel stack, Control control)
    {
        var row = stack.RowCount++;
        stack.RowStyles.Add(new RowStyle(SizeType.AutoSize));
        control.Dock = DockStyle.Top;
        stack.Controls.Add(control, 0, row);
    }
}
