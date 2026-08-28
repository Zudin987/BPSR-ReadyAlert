namespace BPSR.ReadyAlert;

internal sealed partial class ChatGeneralSettingsForm
{
    internal (int PageCount, int VisiblePages, int ActivePages, string ActiveKey, string FrontKey)
        GetV124PageSwitchStateForSelfTest()
    {
        var visible = _pages.Values.Count(x => x.Page.Visible);
        var active = _pages.Values.Count(x => x.Page.ActivePage);
        var front = _contentHost.Controls.Count > 0
            ? _pages.FirstOrDefault(x => ReferenceEquals(x.Value.Page, _contentHost.Controls[0])).Key ?? string.Empty
            : string.Empty;
        return (_pages.Count, visible, active, _activePageKey, front);
    }

    internal void SubscribeV124VisibleChangedForSelfTest(Action callback)
    {
        foreach (var page in _pages.Values.Select(x => x.Page))
            page.VisibleChanged += (_, _) => callback();
    }
}
