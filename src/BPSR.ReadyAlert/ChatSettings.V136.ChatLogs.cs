using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    private readonly CheckBox _keepLocalChatLogs = new()
    {
        Text = "Keep local chat logs"
    };

    private readonly ComboBox _chatLogRetention = new()
    {
        DropDownStyle = ComboBoxStyle.DropDownList,
        Width = 150
    };

    private static int RetentionIndexFromHours(int hours) => ChatLocalLogRetention.NormalizeHours(hours) switch
    {
        ChatLocalLogRetention.OneDayHours => 0,
        ChatLocalLogRetention.ThreeDaysHours => 1,
        _ => 2
    };

    private int SelectedChatLogRetentionHours() => _chatLogRetention.SelectedIndex switch
    {
        0 => ChatLocalLogRetention.OneDayHours,
        1 => ChatLocalLogRetention.ThreeDaysHours,
        _ => ChatLocalLogRetention.SevenDaysHours
    };

    internal (bool Checked, string Text, int RetentionHours, string RetentionText) GetV136ChatLogUiForSelfTest() =>
        (_keepLocalChatLogs.Checked,
         _keepLocalChatLogs.Text,
         SelectedChatLogRetentionHours(),
         _chatLogRetention.SelectedItem?.ToString() ?? string.Empty);
}
