using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    private readonly CheckBox _keepLocalChatLogs = new()
    {
        Text = "Keep local chat logs for 24 hours"
    };

    internal (bool Checked, string Text) GetV136ChatLogUiForSelfTest() =>
        (_keepLocalChatLogs.Checked, _keepLocalChatLogs.Text);
}
