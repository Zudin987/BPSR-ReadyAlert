using System.Reflection;

namespace BPSR.ReadyAlert;

internal static class AppVersion
{
    internal static string Current
    {
        get
        {
            var informational = Assembly.GetExecutingAssembly()
                .GetCustomAttribute<AssemblyInformationalVersionAttribute>()?
                .InformationalVersion;
            if (!string.IsNullOrWhiteSpace(informational))
            {
                var plus = informational.IndexOf('+');
                return plus >= 0 ? informational[..plus] : informational;
            }

            return Assembly.GetExecutingAssembly().GetName().Version?.ToString(3) ?? "1.1.0-rc.4";
        }
    }
}
