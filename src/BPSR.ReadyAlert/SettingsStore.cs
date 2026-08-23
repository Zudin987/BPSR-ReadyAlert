using System.Text.Json;

namespace BPSR.ReadyAlert;

internal sealed class AppSettings
{
    public bool QueuePopAlert { get; set; } = true;
    public bool ReadyCheckAlert { get; set; } = true;
    public bool DesktopNotification { get; set; } = false;
    public bool AutoLaunchResonanceLogs { get; set; } = true;
    public string ResonanceLogsPath { get; set; } = string.Empty;
}

internal sealed class SettingsStore
{
    private readonly string _path;
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase
    };

    internal SettingsStore(string path) => _path = path;

    internal AppSettings Load()
    {
        try
        {
            if (File.Exists(_path))
            {
                var loaded = JsonSerializer.Deserialize<AppSettings>(File.ReadAllText(_path), JsonOptions);
                if (loaded is not null) return loaded;
            }
        }
        catch (Exception ex)
        {
            AppLog.Write("settings: load failed " + ex.Message);
        }

        var settings = new AppSettings();
        Save(settings);
        return settings;
    }

    internal void Save(AppSettings settings)
    {
        try
        {
            Directory.CreateDirectory(Path.GetDirectoryName(_path)!);
            var temp = _path + ".new";
            File.WriteAllText(temp, JsonSerializer.Serialize(settings, JsonOptions));
            File.Move(temp, _path, overwrite: true);
        }
        catch (Exception ex)
        {
            AppLog.Write("settings: save failed " + ex.Message);
        }
    }
}
