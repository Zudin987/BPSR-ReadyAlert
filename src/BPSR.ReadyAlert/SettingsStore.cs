using System.Text.Json;

namespace BPSR.ReadyAlert;

internal sealed class AppSettings
{
    public bool QueuePopAlert { get; set; } = true;
    public bool ReadyCheckAlert { get; set; } = true;
    public bool PartyInviteAlert { get; set; } = true;
    public bool PartyRequestAlert { get; set; } = true;
    public bool DesktopNotification { get; set; } = false;
    public bool AutoLaunchResonanceLogs { get; set; } = true;
    public string ResonanceLogsPath { get; set; } = string.Empty;

    // Empty = follow Resonance Logs CN's Npcap choice, then auto-select if needed.
    // Non-empty = explicitly capture only this Npcap device.
    public string NpcapDeviceName { get; set; } = string.Empty;

    public int AlertVolume { get; set; } = 100;

    // Chat capture/overlay is opt-in and can be toggled from the tray menu.
    public bool ChatOverlayEnabled { get; set; } = false;
    public ChatOverlaySettings Chat { get; set; } = new();
    public ChatSpeechTranslationSettings SpeechTranslation { get; set; } = new();
}

internal sealed class SettingsStore
{
    private readonly string _path;
    private readonly string _backupPath;
    private readonly string _tempPath;
    private readonly object _saveLock = new();

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase
    };

    internal SettingsStore(string path)
    {
        _path = path;
        _backupPath = path + ".bak";
        _tempPath = path + ".new";
    }

    internal AppSettings Load()
    {
        TryDeleteStaleTemp();

        if (TryLoadFile(_path, out var settings, out var primaryError))
        {
            ApplyRuntimeSettings(settings);
            return settings;
        }

        if (!string.IsNullOrWhiteSpace(primaryError))
            AppLog.Write("settings: primary load failed " + primaryError);

        if (TryLoadFile(_backupPath, out settings, out var backupError))
        {
            AppLog.Write("settings: recovered from backup " + _backupPath);
            _ = Save(settings);
            return settings;
        }

        if (!string.IsNullOrWhiteSpace(backupError))
            AppLog.Write("settings: backup load failed " + backupError);

        settings = new AppSettings();
        Normalize(settings);
        _ = Save(settings);
        return settings;
    }

    /// <summary>
    /// Persist settings atomically. Runtime state is still normalized/applied even if
    /// disk persistence fails, but callers that present a "Saved" confirmation can now
    /// distinguish a real durable save from a soft filesystem failure.
    /// </summary>
    internal bool Save(AppSettings settings)
    {
        lock (_saveLock)
        {
            var success = false;
            try
            {
                Normalize(settings);
                // Runtime content filtering follows the in-memory settings immediately.
                // File validation below reads old/new JSON through TryLoadFile, which is
                // deliberately side-effect free and therefore cannot revert this state.
                ApplyRuntimeSettings(settings);

                var directory = Path.GetDirectoryName(_path);
                if (!string.IsNullOrWhiteSpace(directory))
                    Directory.CreateDirectory(directory);

                TryDeleteStaleTemp();
                var json = JsonSerializer.Serialize(settings, JsonOptions);
                using (var stream = new FileStream(_tempPath, FileMode.Create, FileAccess.Write, FileShare.None))
                using (var writer = new StreamWriter(stream))
                {
                    writer.Write(json);
                    writer.Flush();
                    stream.Flush(flushToDisk: true);
                }

                // Never replace the known-good file with bytes we cannot read back.
                if (!TryLoadFile(_tempPath, out _, out var validationError))
                    throw new JsonException("Temporary settings validation failed: " + validationError);

                var currentIsValid = TryLoadFile(_path, out _, out _);
                if (File.Exists(_path) && currentIsValid)
                {
                    File.Replace(_tempPath, _path, _backupPath, ignoreMetadataErrors: true);
                }
                else
                {
                    // Keep an existing valid backup intact when the current primary is
                    // already corrupt; the freshly validated temp becomes the primary.
                    File.Move(_tempPath, _path, overwrite: true);
                }

                success = true;
            }
            catch (Exception ex)
            {
                AppLog.Write("settings: save failed " + ex.Message);
            }
            finally
            {
                TryDeleteStaleTemp();
            }

            return success;
        }
    }

    private static bool TryLoadFile(string path, out AppSettings settings, out string error)
    {
        settings = null!;
        error = string.Empty;
        if (!File.Exists(path)) return false;

        try
        {
            using var stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.Read);
            var loaded = JsonSerializer.Deserialize<AppSettings>(stream, JsonOptions);
            if (loaded is null)
            {
                error = "file contained no settings";
                return false;
            }

            Normalize(loaded);
            settings = loaded;
            return true;
        }
        catch (Exception ex)
        {
            error = ex.Message;
            return false;
        }
    }

    private void TryDeleteStaleTemp()
    {
        try
        {
            if (File.Exists(_tempPath)) File.Delete(_tempPath);
        }
        catch (Exception ex)
        {
            AppLog.Write("settings: temp cleanup failed " + ex.Message);
        }
    }

    private static void Normalize(AppSettings settings)
    {
        settings.AlertVolume = Math.Clamp(settings.AlertVolume, 0, 100);
        settings.NpcapDeviceName ??= string.Empty;
        settings.ResonanceLogsPath ??= string.Empty;
        settings.Chat ??= new ChatOverlaySettings();
        settings.SpeechTranslation ??= new ChatSpeechTranslationSettings();
        settings.Chat.Normalize();
        settings.SpeechTranslation.Normalize();
    }

    private static void ApplyRuntimeSettings(AppSettings settings) =>
        ChatContentVisibility.Configure(
            settings.SpeechTranslation.HideEmojiMessages,
            settings.SpeechTranslation.HideLinkedItemMessages);
}
