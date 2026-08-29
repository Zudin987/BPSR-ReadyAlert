using System.Drawing;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatOverlayForm
{
    // Rendering used to recreate the complete default color dictionary for every
    // owner-drawn row. Keep immutable fallbacks once per process and cache each live
    // configured color until its source string changes. This keeps scrolling and the
    // 15-second relative-time repaint free of avoidable channel-color allocations.
    private static readonly IReadOnlyDictionary<int, Color> V132DefaultChannelColors =
        ChatOverlaySettings.CreateDefaultChannelColors()
            .ToDictionary(
                x => x.Key,
                x => ChatColorUtil.Parse(x.Value, Color.LightGray));

    private readonly Dictionary<int, (string Source, Color Color)> _v132ChannelColorCache = [];

    private Color GetV132ChannelColor(ChatChannel channel)
    {
        var key = (int)channel;
        var fallback = V132DefaultChannelColors.TryGetValue(key, out var defaultColor)
            ? defaultColor
            : Color.LightGray;

        if (!_settings.Chat.ChannelColors.TryGetValue(key, out var value))
            return fallback;

        if (_v132ChannelColorCache.TryGetValue(key, out var cached) &&
            string.Equals(cached.Source, value, StringComparison.Ordinal))
            return cached.Color;

        var parsed = ChatColorUtil.Parse(value, fallback);
        _v132ChannelColorCache[key] = (value, parsed);
        return parsed;
    }

    internal void SelectV132TabForSelfTest(long id) => SelectTab(id);
    internal Color GetV132ChannelColorForSelfTest(ChatChannel channel) => GetV132ChannelColor(channel);
    internal int V132ChannelColorCacheCountForSelfTest => _v132ChannelColorCache.Count;
    internal int V132VisibleMessageCountForSelfTest => _messages.Items.Count;

    internal bool RebuildV132TabBarDisposesOldControlsForSelfTest()
    {
        var old = _tabBar.Controls
            .OfType<System.Windows.Forms.Control>()
            .Select(control => (Control: control, Menu: control.ContextMenuStrip))
            .ToArray();
        RebuildTabBar();
        return old.All(x => x.Control.IsDisposed && (x.Menu is null || x.Menu.IsDisposed));
    }
}
