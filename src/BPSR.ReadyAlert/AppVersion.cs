using System.Reflection;

namespace BPSR.ReadyAlert;

internal static class AppVersion
{
    internal static string Current =>
        Assembly.GetExecutingAssembly().GetName().Version?.ToString(3) ?? "1.0.0";
}
