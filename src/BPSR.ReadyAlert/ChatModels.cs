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
    string Text,
    long SequenceId = 0);

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
    public bool TopMost { get; set; } = true;
    public bool CompactMode { get; set; } = false;
    public bool ShowTime { get; set; } = true;
    public bool ShowTimeAsAgo { get; set; } = true;
    public bool HideStickers { get; set; } = true;

    // v1.2.4 exposes only whole-window opacity. These legacy internal layer values
    // remain readable from older JSON but normalize to one stable rendering preset.
    public int BackgroundOpacity { get; set; } = 82;
    public int ToolbarOpacity { get; set; } = 92;
    public int TextOpacity { get; set; } = 100;
    public int WindowOpacity { get; set; } = 100;
    public string FontFamily { get; set; } = "Segoe UI";
    public float FontSize { get; set; } = 12F;
    public bool BoldMessageText { get; set; } = false;
    public bool TextShadow { get; set; } = true;
    public bool ShowSeparators { get; set; } = true;
    public bool ShowZebraStripes { get; set; } = true;
    public bool ShowColorBand { get; set; } = true;

    // Click-through keeps one global recovery hotkey. Collapse remains available
    // from the overlay button but no longer has a global hotkey in v1.2.4.
    public bool ClickThrough { get; set; } = false;
    public string ClickThroughHotkey { get; set; } = "Ctrl+Shift+F10";
    public string CollapseHotkey { get; set; } = string.Empty;
    public string CollapseSide { get; set; } = "Left";

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
    // Keep the v1.3.6 boolean name for settings.json compatibility; it remains the
    // on/off preference while LocalChatLogRetentionHours controls the rolling window.
    public bool KeepLocalChatLogs24Hours { get; set; } = true;
    public int LocalChatLogRetentionHours { get; set; } = ChatLocalLogRetention.DefaultHours;
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
        FontSize = Math.Clamp(float.IsFinite(FontSize) ? FontSize : 12F, 8F, 24F);
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

        // Clear migrated legacy values so deleting all modern rules does not make
        // an old RC8 rule reappear on the next settings load.
        HighlightSoundEnabled = false;
        HighlightSoundPath = string.Empty;

        MaxHistory = Math.Clamp(MaxHistory, 10, 500);
        LocalChatLogRetentionHours = ChatLocalLogRetention.NormalizeHours(LocalChatLogRetentionHours);
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
        if (string.Equals(value, "Right", StringComparison.OrdinalIgnoreCase)) return "Right";
        if (string.Equals(value, "Top", StringComparison.OrdinalIgnoreCase)) return "Top";
        if (string.Equals(value, "Bottom", StringComparison.OrdinalIgnoreCase)) return "Bottom";
        return "Left";
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
            Id = 639233255393111833L,
            Name = "All",
            Channels =
            [
                (int)ChatChannel.World,
                (int)ChatChannel.Local,
                (int)ChatChannel.Team,
                (int)ChatChannel.Union,
                (int)ChatChannel.Private,
                (int)ChatChannel.Group,
                (int)ChatChannel.Newbie
            ],
            MinLevel = 50,
            ShowIfMatches = string.Empty,
            HideIfMatches = string.Empty
        });
        Tabs.Add(new ChatTabSettings
        {
            Id = 639233255393111900L,
            Name = "Guild&Team",
            Channels =
            [
                (int)ChatChannel.Team,
                (int)ChatChannel.Union,
                (int)ChatChannel.Private,
                (int)ChatChannel.Group
            ],
            MinLevel = 1,
            ShowIfMatches = string.Empty,
            HideIfMatches = string.Empty
        });
        Tabs.Add(new ChatTabSettings
        {
            Id = 639233255393111918L,
            Name = "Guild",
            Channels = [(int)ChatChannel.Union],
            MinLevel = 1,
            ShowIfMatches = string.Empty,
            HideIfMatches = string.Empty
        });
        Tabs.Add(new ChatTabSettings
        {
            Id = 639235625391474596L,
            Name = "Team",
            Channels = [(int)ChatChannel.Team, (int)ChatChannel.Group],
            MinLevel = 1,
            ShowIfMatches = string.Empty,
            HideIfMatches = string.Empty
        });
    }
}
