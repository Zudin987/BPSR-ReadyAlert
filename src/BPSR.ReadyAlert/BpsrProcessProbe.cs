using System.Diagnostics;

namespace BPSR.ReadyAlert;

internal static class BpsrProcessProbe
{
    private static readonly string[] ProcessNames =
    [
        "BPSR", "BPSR_STEAM", "BPSR_EPIC",
        "StarSEA", "StarASIA", "StarSEA_STEAM", "StarASIA_STEAM", "Star"
    ];

    private static readonly object Gate = new();
    private static DateTime _lastRefreshUtc = DateTime.MinValue;
    private static bool _lastResult;

    internal static bool IsRunning()
    {
        lock (Gate)
        {
            var now = DateTime.UtcNow;
            // Recovery decisions operate on 20-45 second windows. A five-second
            // process snapshot is responsive enough for session cleanup/watchdogs
            // while avoiding another full Windows process-table walk every 2 seconds.
            if ((now - _lastRefreshUtc).TotalSeconds < 5)
                return _lastResult;

            _lastRefreshUtc = now;
            _lastResult = ProbeNow();
            return _lastResult;
        }
    }

    private static bool ProbeNow()
    {
        try
        {
            var found = false;
            foreach (var process in Process.GetProcesses())
            {
                try
                {
                    if (ProcessNames.Contains(process.ProcessName, StringComparer.OrdinalIgnoreCase))
                        found = true;
                }
                catch { }
                finally { process.Dispose(); }
            }
            return found;
        }
        catch (Exception ex)
        {
            AppLog.Write("capture-watchdog: process probe failed " + ex.Message);
            return false;
        }
    }
}
