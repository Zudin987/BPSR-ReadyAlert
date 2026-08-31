namespace BPSR.ReadyAlert;

/// <summary>
/// Supported local chat-history retention windows. Keep the persisted value simple
/// and bounded so corrupted/manual settings edits cannot create unbounded storage.
/// </summary>
internal static class ChatLocalLogRetention
{
    internal const int OneDayHours = 24;
    internal const int ThreeDaysHours = 72;
    internal const int SevenDaysHours = 168;
    internal const int DefaultHours = SevenDaysHours;

    internal static int NormalizeHours(int hours) => hours switch
    {
        OneDayHours => OneDayHours,
        ThreeDaysHours => ThreeDaysHours,
        SevenDaysHours => SevenDaysHours,
        _ => DefaultHours
    };

    internal static string DisplayText(int hours) => NormalizeHours(hours) switch
    {
        OneDayHours => "24 hours",
        ThreeDaysHours => "3 days",
        _ => "7 days"
    };
}
