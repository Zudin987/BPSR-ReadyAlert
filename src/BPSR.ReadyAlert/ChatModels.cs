namespace BPSR.ReadyAlert;

internal enum ChatChannel
{
    Null = 0,
    World = 1,
    Local = 2,
    Team = 3,
    Union = 4,
    Private = 5,
    Group = 6,
    TopNotice = 7,
    Play = 8,
    Newbie = 9,
    System = 99
}

internal enum ChatMessageKind
{
    Text = 0,
    TextNotice = 1,
    MultiLanguageNotice = 2,
    Sticker = 3,
    Picture = 4,
    Voice = 5,
    Hypertext = 6
}

internal readonly record struct ChatMessageEvent(
    long SenderId,
    string SenderName,
    int SenderLevel,
    ChatChannel Channel,
    DateTime Timestamp,
    ChatMessageKind Kind,
    string Text);

internal sealed class ChatTabSettings
{
    public long Id { get; set; } = DateTime.UtcNow.Ticks;
    public string Name { get; set; } = "New Tab";
    public List<int> Channels { get; set; } = [];
    public int MinLevel { get; set; } = 50;
    public string ShowIfMatches { get; set; } = string.Empty;
    public string HideIfMatches { get; set; } = string.Empty;

    internal ChatTabSettings Clone() => new()
    {
        Id = Id,
        Name = Name,
        Channels = new List<int>(Channels),
        MinLevel = MinLevel,
        ShowIfMatches = ShowIfMatches,
        HideIfMatches = HideIfMatches
    };
}

internal sealed class ChatBlockedUser
{
    public long Id { get; set; }
    public string Name { get; set; } = string.Empty;
    public DateTime BlockedAtUtc { get; set; } = DateTime.UtcNow;
}

internal sealed class ChatOverlaySettings
{
    public bool TopMost { get; set; } = false;
    public bool CompactMode { get; set; } = true;
    public bool ShowTime { get; set; } = true;
    public bool ShowTimeAsAgo { get; set; } = true;
    public bool HideStickers { get; set; } = false;

    // Overlay presentation. WindowOpacity is the real Win32 whole-window alpha.
    // Background/toolbar/text opacity are rendered independently inside the
    // owner-drawn overlay so text can stay readable without a heavy solid window.
    public int BackgroundOpacity { get; set; } = 82;
    public int ToolbarOpacity { get; set; } = 92;
    public int TextOpacity { get; set; } = 100;
    public int WindowOpacity { get; set; } = 96;
    public string FontFamily { get; set; } = "Segoe UI";
    public float FontSize { get; set; } = 9F;
    public bool BoldMessageText { get; set; } = false;
    public bool TextShadow { get; set; } = true;
    public bool ShowSeparators { get; set; } = false;
    public bool ShowZebraStripes { get; set; } = true;
    public bool ShowColorBand { get; set; } = true;

    // Game-overlay interaction. Click-through always has a global hotkey so the
    // user cannot permanently lock themselves out of the window with the mouse.
    public bool ClickThrough { get; set; } = false;
    public string ClickThroughHotkey { get; set; } = "Ctrl+Shift+F10";
    public string CollapseHotkey { get; set; } = "Ctrl+Shift+F9";
    public string CollapseSide { get; set; } = "Right";

    // Global notification/highlight rules are separate from per-tab visibility
    // filters. They reuse ReadyAlert's safe, case-insensitive expression engine.
    public string HighlightIfMatches { get; set; } = string.Empty;
    public string HighlightColor { get; set; } = "#6B5A3A";
    public bool HighlightSoundEnabled { get; set; } = false;
    public string HighlightSoundPath { get; set; } = string.Empty;
    public bool PrivateHighlightEnabled { get; set; } = true;
    public string PrivateHighlightColor { get; set; } = "#56355D";
    public bool PrivateSoundEnabled { get; set; } = false;
    public string PrivateSoundPath { get; set; } = string.Empty;
    public int ChatSoundVolume { get; set; } = 100;

    public Dictionary<int, string> ChannelColors { get; set; } = [];

    public int MaxHistory { get; set; } = 200;
    public int WindowX { get; set; } = int.MinValue;
    public int WindowY { get; set; } = int.MinValue;
    public int WindowWidth { get; set; } = 700;
    public int WindowHeight { get; set; } = 430;
    public long LastSelectedTabId { get; set; } = -1;
    public List<ChatTabSettings> Tabs { get; set; } = [];
    public List<ChatBlockedUser> BlockedUsers { get; set; } = [];

    internal void Normalize()
    {
        BackgroundOpacity = Math.Clamp(BackgroundOpacity, 10, 100);
        ToolbarOpacity = Math.Clamp(ToolbarOpacity, 15, 100);
        TextOpacity = Math.Clamp(TextOpacity, 40, 100);
        WindowOpacity = Math.Clamp(WindowOpacity, 25, 100);
        FontFamily = string.IsNullOrWhiteSpace(FontFamily) ? "Segoe UI" : FontFamily.Trim();
        if (FontFamily.Length > 100) FontFamily = FontFamily[..100];
        FontSize = Math.Clamp(float.IsFinite(FontSize) ? FontSize : 9F, 8F, 24F);
        ClickThroughHotkey = NormalizeHotkeyText(ClickThroughHotkey, "Ctrl+Shift+F10");
        CollapseHotkey = NormalizeHotkeyText(CollapseHotkey, "Ctrl+Shift+F9");
        CollapseSide = NormalizeCollapseSide(CollapseSide);
        HighlightIfMatches ??= string.Empty;
        if (HighlightIfMatches.Length > 4096) HighlightIfMatches = HighlightIfMatches[..4096];
        HighlightColor = NormalizeHexColor(HighlightColor, "#6B5A3A");
        PrivateHighlightColor = NormalizeHexColor(PrivateHighlightColor, "#56355D");
        HighlightSoundPath ??= string.Empty;
        PrivateSoundPath ??= string.Empty;
        if (HighlightSoundPath.Length > 1024) HighlightSoundPath = HighlightSoundPath[..1024];
        if (PrivateSoundPath.Length > 1024) PrivateSoundPath = PrivateSoundPath[..1024];
        ChatSoundVolume = Math.Clamp(ChatSoundVolume, 0, 100);

        MaxHistory = Math.Clamp(MaxHistory, 10, 500);
        WindowWidth = Math.Clamp(WindowWidth, 360, 2400);
        WindowHeight = Math.Clamp(WindowHeight, 180, 1600);
        Tabs ??= [];
        BlockedUsers ??= [];
        ChannelColors ??= [];

        var defaultColors = CreateDefaultChannelColors();
        foreach (var pair in defaultColors)
        {
            if (!ChannelColors.TryGetValue(pair.Key, out var color))
                ChannelColors[pair.Key] = pair.Value;
            else
                ChannelColors[pair.Key] = NormalizeHexColor(color, pair.Value);
        }

        var knownChannels = Enum.GetValues<ChatChannel>().Select(x => (int)x).ToHashSet();
        ChannelColors = ChannelColors
            .Where(x => knownChannels.Contains(x.Key))
            .ToDictionary(x => x.Key, x => NormalizeHexColor(x.Value, defaultColors[x.Key]));

        var seenIds = new HashSet<long>();
        for (var i = 0; i < Tabs.Count; i++)
        {
            var tab = Tabs[i];
            tab.Name = string.IsNullOrWhiteSpace(tab.Name) ? "Chat" : tab.Name.Trim();
            if (tab.Name.Length > 40) tab.Name = tab.Name[..40];
            tab.Channels ??= [];
            tab.Channels = tab.Channels.Where(knownChannels.Contains).Distinct().ToList();
            tab.MinLevel = Math.Clamp(tab.MinLevel, 1, 100);
            tab.ShowIfMatches ??= string.Empty;
            tab.HideIfMatches ??= string.Empty;
            if (tab.ShowIfMatches.Length > 4096) tab.ShowIfMatches = tab.ShowIfMatches[..4096];
            if (tab.HideIfMatches.Length > 4096) tab.HideIfMatches = tab.HideIfMatches[..4096];

            if (tab.Id == 0 || !seenIds.Add(tab.Id))
            {
                tab.Id = DateTime.UtcNow.Ticks + i;
                seenIds.Add(tab.Id);
            }
        }

        BlockedUsers = BlockedUsers
            .Where(x => x is not null && x.Id != 0)
            .GroupBy(x => x.Id)
            .Select(g => g.First())
            .ToList();

        if (Tabs.Count == 0)
            AddDefaultTabs();

        if (!Tabs.Any(t => t.Id == LastSelectedTabId))
            LastSelectedTabId = Tabs[0].Id;
    }

    internal static Dictionary<int, string> CreateDefaultChannelColors() => new()
    {
        [(int)ChatChannel.Null] = "#C7C7C7",
        [(int)ChatChannel.World] = "#63C7FF",
        [(int)ChatChannel.Local] = "#8FED8F",
        [(int)ChatChannel.Team] = "#FFB5C2",
        [(int)ChatChannel.Union] = "#FFD600",
        [(int)ChatChannel.Private] = "#FFA1FF",
        [(int)ChatChannel.Group] = "#ADD8E6",
        [(int)ChatChannel.TopNotice] = "#FF8C00",
        [(int)ChatChannel.Play] = "#C6A8FF",
        [(int)ChatChannel.Newbie] = "#9FA8B2",
        [(int)ChatChannel.System] = "#FF6347"
    };

    private static string NormalizeHotkeyText(string? value, string fallback)
    {
        value = value?.Trim() ?? string.Empty;
        return value.Length is > 0 and <= 80 ? value : fallback;
    }

    private static string NormalizeCollapseSide(string? value)
    {
        if (string.Equals(value, "Left", StringComparison.OrdinalIgnoreCase)) return "Left";
        if (string.Equals(value, "Top", StringComparison.OrdinalIgnoreCase)) return "Top";
        if (string.Equals(value, "Bottom", StringComparison.OrdinalIgnoreCase)) return "Bottom";
        return "Right";
    }

    private static string NormalizeHexColor(string? value, string fallback)
    {
        if (string.IsNullOrWhiteSpace(value)) return fallback;
        value = value.Trim();
        if (value.Length != 7 || value[0] != '#') return fallback;
        for (var i = 1; i < value.Length; i++)
        {
            if (!Uri.IsHexDigit(value[i])) return fallback;
        }
        return value.ToUpperInvariant();
    }

    private void AddDefaultTabs()
    {
        Tabs.Add(new ChatTabSettings
        {
            Name = "World",
            Channels = [(int)ChatChannel.World]
        });
        Tabs.Add(new ChatTabSettings
        {
            Name = "Guild / Team",
            Channels = [(int)ChatChannel.Union, (int)ChatChannel.Group, (int)ChatChannel.Team]
        });
        Tabs.Add(new ChatTabSettings
        {
            Name = "All",
            Channels =
            [
                (int)ChatChannel.Null,
                (int)ChatChannel.World,
                (int)ChatChannel.Local,
                (int)ChatChannel.Team,
                (int)ChatChannel.Union,
                (int)ChatChannel.Private,
                (int)ChatChannel.Group,
                (int)ChatChannel.TopNotice,
                (int)ChatChannel.Play,
                (int)ChatChannel.Newbie,
                (int)ChatChannel.System
            ],
            MinLevel = 1
        });
    }
}
