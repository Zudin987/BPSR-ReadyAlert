using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    /// <summary>
    /// v1.2.2 density pass: keep the WinForms UI native and immediate, but reduce
    /// unnecessary whitespace, oversized fields and repeated explanatory copy.
    /// This method is intentionally idempotent because the Speech page is registered
    /// by the outer constructor after the base Settings pages are built.
    /// </summary>
    private void InstallV122CompactUi()
    {
        if (_pages.TryGetValue("Speech", out var speech))
            speech.Button.Text = "Speech";

        HideV122CardByTitle("Translation used by TTS");
        ReplaceV122Copy(this);

        foreach (var entry in _pages.Values)
        {
            entry.Button.Height = 36;
            entry.Button.Padding = new Padding(12, 0, 7, 0);
            entry.Button.Margin = new Padding(0, 0, 0, 2);

            entry.Page.Padding = new Padding(20, 16, 20, 20);
            if (entry.Page.Tag is TableLayoutPanel stack && stack.Controls.Count > 0)
                stack.Controls[0].Margin = new Padding(0, 0, 0, 12);

            CompactV122Tree(entry.Page);
        }

        _highlight.Height = 70;
        foreach (var box in _soundRuleMatch) box.Height = 58;
        _ttsOwnUsername.Width = Math.Min(320, Math.Max(240, _ttsOwnUsername.Width));
        _ttsOwnUsername.MaximumSize = new Size(320, 0);
        _fontFamily.Width = Math.Min(300, Math.Max(220, _fontFamily.Width));
        _fontFamily.MaximumSize = new Size(300, 0);
        _clickHotkey.MaximumSize = new Size(300, 0);
        _collapseHotkey.MaximumSize = new Size(300, 0);

        var footer = FindV122Footer();
        if (footer is not null) footer.Height = 60;
        var sidebar = FindV122Sidebar();
        if (sidebar is not null) sidebar.Width = 176;
    }

    private static void CompactV122Tree(Control parent)
    {
        foreach (Control child in parent.Controls)
        {
            switch (child)
            {
                case ChatCardPanel card:
                    card.Padding = new Padding(14);
                    card.Margin = new Padding(0, 0, 0, 10);
                    break;

                case TableLayoutPanel table:
                    CompactV122Columns(table);
                    break;

                case FlowLayoutPanel flow:
                    if (flow.Padding.Bottom >= 12)
                        flow.Padding = new Padding(flow.Padding.Left, flow.Padding.Top, flow.Padding.Right, 8);
                    break;

                case TrackBar slider:
                    slider.AutoSize = false;
                    slider.Height = 30;
                    break;

                case TextBox box when box.Multiline:
                    box.Height = Math.Min(box.Height, 70);
                    break;

                case TextBox box when !box.ReadOnly:
                    box.MaximumSize = new Size(340, 0);
                    break;

                case ComboBox combo:
                    combo.MaximumSize = new Size(320, 0);
                    break;

                case NumericUpDown numeric:
                    numeric.Width = Math.Min(numeric.Width, 110);
                    break;

                case Label label:
                    if (label.MaximumSize.Width is >= 380 and <= 460)
                        label.MaximumSize = new Size(560, 0);
                    if (label.Margin.Bottom >= 14)
                        label.Margin = new Padding(label.Margin.Left, label.Margin.Top, label.Margin.Right, 8);
                    if (label.Font.Size >= 17F)
                        label.Font = ChatUiTheme.UiFont(16F, label.Font.Style);
                    break;

                case Panel panel when panel.Height == 54:
                    panel.Height = 44;
                    panel.Padding = new Padding(panel.Padding.Left, Math.Min(panel.Padding.Top, 3), panel.Padding.Right, Math.Min(panel.Padding.Bottom, 8));
                    break;
            }

            if (child.HasChildren)
                CompactV122Tree(child);
        }
    }

    private static void CompactV122Columns(TableLayoutPanel table)
    {
        for (var i = 0; i < table.ColumnStyles.Count; i++)
        {
            var style = table.ColumnStyles[i];
            if (style.SizeType != SizeType.Absolute) continue;
            style.Width = style.Width switch
            {
                >= 225F and <= 235F => 185F,
                >= 188F and <= 192F => 160F,
                >= 130F and <= 134F => 112F,
                >= 110F and <= 114F => 100F,
                >= 56F and <= 60F => 50F,
                _ => style.Width
            };
        }
    }

    private static readonly Dictionary<string, string> V122Copy = new(StringComparer.Ordinal)
    {
        ["Tune readability without making the overlay feel heavy. Changes apply after you save."] = "Readability, density and transparency.",
        ["Choose the information and visual cues shown for each chat line."] = "Choose what each chat row shows.",
        ["Keep the message text comfortable to read over the game."] = "Font used for chat messages.",
        ["The first three controls are independent; Whole window opacity is applied last."] = "Whole-window opacity is applied last.",
        ["Control how the overlay behaves while you play. Hotkeys work globally while ReadyAlert is running."] = "Mouse behavior, hotkeys, docking and history.",
        ["Keep the overlay out of the way without making it impossible to recover."] = "Click-through and sticker visibility.",
        ["When click-through is ON, mouse clicks pass to the game. Use the recovery hotkey below to turn it OFF. If that hotkey cannot register, ReadyAlert automatically disables click-through."] = "Keep a recovery hotkey for click-through. If it cannot register, ReadyAlert turns click-through off.",
        ["Click a box and press the combination you want. Backspace clears it."] = "Press the shortcut you want. Backspace clears it.",
        ["Click a box and press the combination you want. Both recovery shortcuts are required; Backspace clears the current entry before choosing another."] = "Both recovery shortcuts are required. Backspace clears the current entry.",
        ["Choose where the compact edge handle lives and how much recent chat is retained."] = "Collapsed edge and retained chat.",
        ["Highlight important chat visually and configure up to three different keyword sounds. Nothing is sent outside your PC."] = "Keyword highlights and chat sounds.",
        ["One shared volume for all three sound rules and Private / Talk sounds."] = "Keyword and Private / Talk sounds only.",
        ["Keyword rules and Private / Talk sounds only. Independent of Ready / Queue and TTS volume."] = "Keyword and Private / Talk sounds only.",
        ["A single standardized volume keeps sound setup simple."] = "Chat sounds only; Ready / Queue and TTS stay separate.",
        ["One shared level for chat keyword and Private / Talk sounds; other ReadyAlert audio volumes stay independent."] = "Chat sounds only; Ready / Queue and TTS stay separate.",
        ["This changes row color only. Sound triggers are configured separately below."] = "Color matching rows without sound.",
        ["Case-insensitive. Supports PA, serum | food, one pattern per line, AND, or advanced regex."] = "Use | for OR, AND for all terms, or regex.",
        ["Leave empty to use ReadyAlert's built-in alert sound."] = "Empty = built-in alert sound.",
        ["Direct-message audio also uses the shared Chat alert volume above."] = "Uses the Chat alert volume above.",
        ["Less-used customization and troubleshooting tools. These do not start another packet capture."] = "Less-used tools and diagnostics.",
        ["Personalize channel identity and maintain your local block list."] = "Channel colors and blocked users.",
        ["Quickly tell whether BPSR chat packets are reaching the parser and UI queue."] = "Shared capture status and counters.",
        ["Translate World / Guild / Party chat to English and optionally read Guild / Party messages aloud."] = "Translate chat and optionally speak Guild / Party messages.",
        ["The original BPSR message appears immediately. A successful non-English translation is added underneath it later."] = "Original first; English is added when ready.",
        ["World is independent. Guild = BPSR Union. Party / Team covers Team and Group chat."] = "Guild = Union. Party / Team includes Team and Group.",
        ["Speech is intentionally limited to Guild and Party / Team. World chat is never read aloud."] = "Guild and Party / Team only. World is never spoken.",
        ["Exact name match, case-insensitive. Leave empty if you want your own messages read too."] = "Exact name match, case-insensitive. Empty = read mine too.",
        ["Guild / Party speech only. Independent of Ready / Queue volume in the tray and Chat alert volume under Highlights & sounds."] = "Speech only. Ready / Queue and Chat alert volumes stay separate.",
        ["Plays one short Google English TTS sample at the TTS volume above. It does not use either alert-sound volume."] = "Play a short sample at the TTS volume above.",
        ["Uses Google's no-key English (en) Translate TTS voice. Messages are queued one at a time so speech never overlaps itself."] = "Google English TTS. Speech is queued one message at a time.",
        ["Three independent audio volumes"] = "Audio volumes",
        ["Ready / Queue volume is in the tray menu. Chat alert volume controls keyword and Private / Talk sounds. TTS volume above controls spoken Guild / Party chat only. Changing one does not change either of the others."] = "Ready / Queue, Chat alerts and TTS each keep their own volume.",
        ["No API key — Google Translate web service"] = "Google web service",
        ["This uses undocumented Google Translate/gTTS-style web endpoints, not Google Cloud. Only messages needed for enabled translation/TTS channels are sent to Google. Google can rate-limit or change the service without notice; failures never block the overlay or packet capture."] = "No API key. Only enabled translation/TTS chat is sent to Google. TTS translates to English first when possible. Google can rate-limit or change these web endpoints."
    };

    private static void ReplaceV122Copy(Control parent)
    {
        foreach (Control child in parent.Controls)
        {
            if (child is Label label && V122Copy.TryGetValue(label.Text, out var compact))
                label.Text = compact;
            if (child.HasChildren) ReplaceV122Copy(child);
        }
    }

    private void HideV122CardByTitle(string title)
    {
        var label = FindV122Label(this, title);
        if (label is null) return;

        Control container = label;
        while (container.Parent is not null && container.Parent is not TableLayoutPanel)
            container = container.Parent;
        if (container.Parent is TableLayoutPanel)
            container.Visible = false;
    }

    private static Label? FindV122Label(Control parent, string text)
    {
        foreach (Control child in parent.Controls)
        {
            if (child is Label label && string.Equals(label.Text, text, StringComparison.Ordinal)) return label;
            var nested = FindV122Label(child, text);
            if (nested is not null) return nested;
        }
        return null;
    }

    private Panel? FindV122Footer() =>
        Controls.OfType<Panel>().FirstOrDefault(x => x.Dock == DockStyle.Bottom);

    private Panel? FindV122Sidebar()
    {
        var shell = Controls.OfType<Panel>().FirstOrDefault(x => x.Dock == DockStyle.Fill);
        return shell?.Controls.OfType<Panel>().FirstOrDefault(x => x.Dock == DockStyle.Left);
    }

    internal void ShowV122PageForSelfTest(string key) => ShowPage(key);

    internal (int VisiblePages, bool BufferedHost, int SidebarWidth, int FooterHeight, int MaxRuleHeight, int MaxNavHeight, string ActiveKey)
        GetV122CompactMetricsForSelfTest()
    {
        InstallV122CompactUi();
        var visible = _pages.Values.Count(x => x.Page.Visible);
        var sidebar = FindV122Sidebar();
        var footer = FindV122Footer();
        var maxRuleHeight = new[] { _highlight.Height }.Concat(_soundRuleMatch.Select(x => x.Height)).Max();
        var maxNav = _pages.Values.Select(x => x.Button.Height).DefaultIfEmpty(0).Max();
        return (
            visible,
            _contentHost is ChatBufferedPanel,
            sidebar?.Width ?? 0,
            footer?.Height ?? 0,
            maxRuleHeight,
            maxNav,
            _activePageKey);
    }
}
