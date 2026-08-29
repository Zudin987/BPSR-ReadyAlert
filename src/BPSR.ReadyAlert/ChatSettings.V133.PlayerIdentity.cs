using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    private readonly Label _v133DetectedUsername = new();
    private bool _v133IdentitySubscribed;

    private void InstallV133PlayerIdentityUi()
    {
        RefreshV133PlayerIdentityDisplay();
        Shown += (_, _) => StartV133PlayerIdentityTracking();
        FormClosed += (_, _) => StopV133PlayerIdentityTracking();
    }

    private void StartV133PlayerIdentityTracking()
    {
        if (_v133IdentitySubscribed) return;
        _v133IdentitySubscribed = true;
        PlayerIdentityCaptureBridge.IdentityChanged += V133IdentityChanged;
        RefreshV133PlayerIdentityDisplay();
    }

    private void StopV133PlayerIdentityTracking()
    {
        if (!_v133IdentitySubscribed) return;
        _v133IdentitySubscribed = false;
        PlayerIdentityCaptureBridge.IdentityChanged -= V133IdentityChanged;
    }

    private void V133IdentityChanged(DetectedPlayerIdentity? identity)
    {
        if (IsDisposed || Disposing) return;
        if (InvokeRequired)
        {
            try { BeginInvoke(new Action(RefreshV133PlayerIdentityDisplay)); }
            catch (InvalidOperationException) { }
            catch (ObjectDisposedException) { }
            return;
        }
        RefreshV133PlayerIdentityDisplay();
    }

    private void RefreshV133PlayerIdentityDisplay()
    {
        if (IsDisposed) return;
        var identity = PlayerIdentityCaptureBridge.Current;
        if (identity is { } detected)
        {
            _v133DetectedUsername.Text = $"{detected.Name}   ·   UID {detected.CharacterUid}";
            _v133DetectedUsername.ForeColor = ChatUiTheme.Success;
            _v133DetectedUsername.AccessibleDescription =
                $"Detected BPSR username {detected.Name}, UID {detected.CharacterUid}.";
        }
        else
        {
            _v133DetectedUsername.Text = "Waiting for BPSR to identify your character…";
            _v133DetectedUsername.ForeColor = ChatUiTheme.TextMuted;
            _v133DetectedUsername.AccessibleDescription =
                "ReadyAlert has not received the current BPSR EnterScene identity yet.";
        }
    }

    internal (string Status, string ManualOverride, string Placeholder) GetV133IdentityUiForSelfTest() =>
        (_v133DetectedUsername.Text, _ttsOwnUsername.Text, _ttsOwnUsername.PlaceholderText);
}
