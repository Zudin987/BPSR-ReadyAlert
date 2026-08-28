using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    /// <summary>
    /// Keep Settings checkboxes on the native WinForms interaction path. The flat
    /// checkbox styling introduced for v1.2.3 looked compact but was unreliable on
    /// the user's live Windows setup. Standard AutoCheck controls are still compact
    /// while guaranteeing mouse/keyboard toggles update Checked immediately.
    /// </summary>
    private void ApplyV124InteractiveCheckboxes()
    {
        ApplyV124InteractiveCheckboxes(this);
    }

    private static void ApplyV124InteractiveCheckboxes(Control parent)
    {
        foreach (Control child in parent.Controls)
        {
            if (child is CheckBox check)
                MakeV124CheckboxInteractive(check);
            if (child.HasChildren)
                ApplyV124InteractiveCheckboxes(child);
        }
    }

    private static void MakeV124CheckboxInteractive(CheckBox check)
    {
        check.AutoCheck = true;
        check.FlatStyle = FlatStyle.Standard;
        check.UseVisualStyleBackColor = false;
        check.Cursor = Cursors.Hand;
        check.TabStop = true;
    }

    internal bool ToggleV124CheckboxForSelfTest(string text)
    {
        var check = FindV124Checkbox(this, text);
        if (check is null) return false;
        var before = check.Checked;
        check.Checked = !before;
        return check.Checked != before && check.AutoCheck && check.Enabled;
    }

    private static CheckBox? FindV124Checkbox(Control parent, string text)
    {
        foreach (Control child in parent.Controls)
        {
            if (child is CheckBox check && string.Equals(check.Text, text, StringComparison.Ordinal))
                return check;
            var nested = FindV124Checkbox(child, text);
            if (nested is not null) return nested;
        }
        return null;
    }
}
