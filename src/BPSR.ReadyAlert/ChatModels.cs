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
    public int WindowOpacity { get; set; } = 92;
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
        WindowOpacity = Math.Clamp(WindowOpacity, 25, 100);
        MaxHistory = Math.Clamp(MaxHistory, 10, 500);
        WindowWidth = Math.Clamp(WindowWidth, 420, 2400);
        WindowHeight = Math.Clamp(WindowHeight, 260, 1600);
        Tabs ??= [];
        BlockedUsers ??= [];

        foreach (var tab in Tabs)
        {
            tab.Name = string.IsNullOrWhiteSpace(tab.Name) ? "Chat" : tab.Name.Trim();
            tab.Channels ??= [];
            tab.MinLevel = Math.Clamp(tab.MinLevel, 1, 100);
            tab.ShowIfMatches ??= string.Empty;
            tab.HideIfMatches ??= string.Empty;
        }

        if (Tabs.Count == 0)
            AddDefaultTabs();

        if (!Tabs.Any(t => t.Id == LastSelectedTabId))
            LastSelectedTabId = Tabs[0].Id;
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
                (int)ChatChannel.World,
                (int)ChatChannel.Local,
                (int)ChatChannel.Team,
                (int)ChatChannel.Union,
                (int)ChatChannel.Private,
                (int)ChatChannel.Group,
                (int)ChatChannel.TopNotice,
                (int)ChatChannel.System
            ],
            MinLevel = 1
        });
    }
}
