using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal static class ChatClipboard
{
    /// <summary>
    /// Windows clipboard access can fail transiently when another process owns it.
    /// Use WinForms' built-in retry overload, then fail softly instead of letting a
    /// context-menu click or diagnostics copy action escape as a UI-thread exception.
    /// </summary>
    internal static bool TrySetText(IWin32Window? owner, string? text, string description)
    {
        if (string.IsNullOrEmpty(text)) return false;

        try
        {
            Clipboard.SetDataObject(text, copy: true, retryTimes: 5, retryDelay: 20);
            return true;
        }
        catch (Exception ex)
        {
            AppLog.Write($"clipboard: failed action={description}: {ex.Message}");
            var message = "ReadyAlert could not copy to the Windows clipboard because it is temporarily unavailable.\r\n\r\n" +
                          "Try the copy action again. The app and chat capture are unaffected.";
            if (owner is null)
            {
                MessageBox.Show(message, "Clipboard unavailable", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            }
            else
            {
                MessageBox.Show(owner, message, "Clipboard unavailable", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            }
            return false;
        }
    }
}
