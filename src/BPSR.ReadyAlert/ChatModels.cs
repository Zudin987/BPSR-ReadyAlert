using System.Text.Json.Serialization;

namespace BPSR.ReadyAlert;

internal enum ChatChannel
{
    Null = 0,
    World = 1,
    Local = 2,
    Group = 3,
    Team = 4,
    Private = 5,
    Union = 6,
    System = 7,
    TopNotice = 8,
    Newbie = 9,
    Play = 10
}

internal enum ChatMessageKind
{
    Text,
    Sticker,
    Voice,
    Image,
    Unknown
}

internal sealed record ChatMessageEvent(
    long SequenceId,
    DateTime Timestamp,
    ChatChannel Channel,
    ChatMessageKind Kind,
    long SenderId,
    string SenderName,
    int SenderLevel,
    string Text,
    int PayloadLength,
    int Flags);

internal sealed class ChatBlockedUser
{
    public long Id { get; set; }
    public string Name { get; set; } = string.Empty;
    public DateTime BlockedAtUtc { get; set; } = DateTime.UtcNow;
}

internal sealed class ChatTabSettings
{
    public long Id { get; set; } = DateTime.UtcNow.Ticks;
    public string Name { get; set; } = "Chat";
    public List<int> Channels { get; set; } = [];
    public int MinLevel { get; set; } = 1;
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

internal sealed class ChatSoundRule
{
    public bool Enabled { get; set; }
    public string Match { get; set; } = string.Empty;
    public string SoundPath { get; set; } = string.Empty;

    internal ChatSoundRule Clone() => new()
    {
        Enabled = Enabled,
        Match = Match,
        SoundPath = SoundPath
    };
}

internal sealed class ChatOverlaySettings
{
    public bool TopMost { get; set; } = false;
    public bool CompactMode { get; set; } = true;
    public bool ShowTime { get; set; } = true;
    public bool ShowTimeAsAgo { get; set; } = true;
    public bool HideStickers { get; set; } = false;

    // v1.2.4 exposes one user-facing transparency control: WindowOpacity. The
    // internal layer values remain serialized only for backward-compatible reads
    // and are normalized to one stable rendering preset.
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

    // Click-through keeps one global recovery hotkey. Collapse remains available
    // from the overlay button but no longer has a global hotkey in v1.2.4.
    public bool ClickThrough { get; set; } = false;
    public string ClickThroughHotkey { get; set; } = "Ctrl+Shift+F10";
    public string CollapseHotkey { get; set; } = string.Empty;
    public string CollapseSide { get; set; } = "Right";

    public string HighlightIfMatches { get; set; } = string.Empty;
    public string HighlightColor { get; set; } = "#6B5A3A";
    public List<ChatSoundRule> HighlightSoundRules { get; set; } = [];

    // Legacy RC8 fields are kept only so existing settings migrate automatically.
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
        BackgroundOpacity = 82;
        ToolbarOpacity = 92;
        TextOpacity = 100;
        WindowOpacity = Math.Clamp(WindowOpacity, 25, 100);
        FontFamily = string.IsNullOrWhiteSpace(FontFamily) ? "Segoe UI" : FontFamily.Trim();
        if (FontFamily.Length > 100) FontFamily = FontFamily[..100];
        FontSize = Math.Clamp(float.IsFinite(FontSize) ? FontSize : 9F, 8F, 24F);
        ClickThroughHotkey = NormalizeHotkeyText(ClickThroughHotkey, "Ctrl+Shift+F10");
        CollapseHotkey = string.Empty;
        CollapseSide = NormalizeCollapseSide(CollapseSide);

        HighlightIfMatches ??= string.Empty;
        if (HighlightIfMatches.Length > 4096) HighlightIfMatches = HighlightIfMatches[..4096];
        HighlightColor = NormalizeHexColor(HighlightColor, "#6B5A3A");
        PrivateHighlightColor = NormalizeHexColor(PrivateHighlightColor, "#56355D");
        PrivateSoundPath ??= string.Empty;
        if (PrivateSoundPath.Length > 1024) PrivateSoundPath = PrivateSoundPath[..1024];
        ChatSoundVolume = Math.Clamp(ChatSoundVolume, 0, 100);

        HighlightSoundRules ??= [];
        HighlightSoundPath ??= string.Empty;
        if (HighlightSoundRules.Count == 0 && HighlightSoundEnabled && !string.IsNullOrWhiteSpace(HighlightIfMatches))
        {
            HighlightSoundRules.Add(new ChatSoundRule
            {
                Enabled = true,
                Match = HighlightIfMatches,
                SoundPath = HighlightSoundPath
            });
        }

        HighlightSoundRules = HighlightSoundRules
            .Where(x => x is not null)
            .Take(2)
            .Select(x => x!)
            .ToList();
        foreach (var rule in HighlightSoundRules)
        {
            rule.Match = (rule.Match ?? string.Empty).Trim();
            rule.SoundPath = rule.SoundPath ?? string.Empty;
            if (rule.Match.Length > 4096) rule.Match = rule.Match[..4096];
            if (rule.SoundPath.Length > 1024) rule.SoundPath = rule.SoundPath[..1024];
            if (string.IsNullOrWhiteSpace(rule.Match)) rule.Enabled = false;
        }

        HighlightSoundEnabled = false;
        HighlightSoundPath = string.Empty;

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
        [(int)ChatChannel.Private] = "#D7A4FF",
        [(int)ChatChannel.Group] = "#FFB5C2",
        [(int)ChatChannel.System] = "#AFC8FF",
        [(int)ChatChannel.TopNotice] = "#FF7A7A",
        [(int)ChatChannel.Newbie] = "#86DEFF",
        [(int)ChatChannel.Play] = "#C9B8FF"
    };

    private void AddDefaultTabs()
    {
        var world = new ChatTabSettings
        {
            Id = 1001,
            Name = "World",
            Channels = [(int)ChatChannel.World, (int)ChatChannel.Newbie],
            MinLevel = 1
        };
        var guild = new ChatTabSettings
        {
            Id = 1002,
            Name = "Guild/Team",
            Channels = [(int)ChatChannel.Union, (int)ChatChannel.Team, (int)ChatChannel.Group],
            MinLevel = 1
        };
        var all = new ChatTabSettings
        {
            Id = 1003,
            Name = "All",
            Channels =
            [
                (int)ChatChannel.World, (int)ChatChannel.Newbie,
                (int)ChatChannel.Local, (int)ChatChannel.Union,
                (int)ChatChannel.Team, (int)ChatChannel.Group,
                (int)ChatChannel.Private
            ],
            MinLevel = 1
        };
        Tabs = [world, guild, all];
        LastSelectedTabId = world.Id;
    }

    private static string NormalizeHexColor(string? value, string fallback)
    {
        if (string.IsNullOrWhiteSpace(value)) return fallback;
        var text = value.Trim();
        if (text.Length == 7 && text[0] == '#' &&
            text.AsSpan(1).ToString().All(Uri.IsHexDigit)) return text.ToUpperInvariant();
        return fallback;
    }

    private static string NormalizeHotkeyText(string? value, string fallback)
    {
        var text = string.IsNullOrWhiteSpace(value) ? fallback : value.Trim();
        if (text.Length > 80) text = text[..80];
        return text;
    }

    private static string NormalizeCollapseSide(string? value) =>
        value is "Left" or "Right" or "Top" or "Bottom" ? value : "Right";
}
