using System.Collections.Concurrent;
using System.Diagnostics;
using System.Drawing;
using System.Media;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed class TrayApplicationContext : ApplicationContext
{
    private readonly AppPaths _paths;
    private readonly AppSettings _settings;
    private readonly SettingsStore _settingsStore;
    private readonly ResonanceLogsLauncher _launcher;
    private readonly NpcapCapturePlan _capturePlan;
    private readonly NotifyIcon _tray;
    private readonly System.Windows.Forms.Timer _timer;
    private readonly ConcurrentQueue<AlertEvent> _events = new();
    private readonly CaptureEngine _engine;
    private readonly SoundPlayer _player;
    private readonly Icon _appIcon;

    internal TrayApplicationContext(
        AppPaths paths,
        AppSettings settings,
        SettingsStore settingsStore,
        ResonanceLogsLauncher launcher,
        NpcapCapturePlan capturePlan)
    {
        _paths = paths;
        _settings = settings;
        _settingsStore = settingsStore;
        _launcher = launcher;
        _capturePlan = capturePlan;
        _player = new SoundPlayer(_paths.AlertSoundPath);
        try { _player.Load(); } catch (Exception ex) { AppLog.Write("audio: preload failed " + ex.Message); }

        _appIcon = LoadApplicationIcon(_paths.AppIconPath);
        var menu = BuildMenu();
        _tray = new NotifyIcon
        {
            Icon = _appIcon,
            Text = "BPSR Ready Alert - Npcap",
            ContextMenuStrip = menu,
            Visible = true
        };

        AppLog.Write(
            $"settings: queue={_settings.QueuePopAlert} ready={_settings.ReadyCheckAlert} " +
            $"notification={_settings.DesktopNotification} autoLaunch={_settings.AutoLaunchResonanceLogs}");
        AppLog.Write($"capture: plan adapters={_capturePlan.Candidates.Count} preferred={_capturePlan.Primary.Description} source={_capturePlan.Primary.Source}");

        _engine = new CaptureEngine(_events, _capturePlan);
        _engine.Start();

        _timer = new System.Windows.Forms.Timer { Interval = 100 };
        _timer.Tick += (_, _) => DrainEvents();
        _timer.Start();

        AppLog.Write("tray: running");
    }

    private static Icon LoadApplicationIcon(string iconPath)
    {
        try
        {
            if (File.Exists(iconPath))
            {
                using var stream = new FileStream(iconPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite);
                using var icon = new Icon(stream);
                AppLog.Write("icon: loaded custom icon " + iconPath);
                return (Icon)icon.Clone();
            }
        }
        catch (Exception ex)
        {
            AppLog.Write("icon: direct load failed " + ex.Message);
        }

        try
        {
            var exe = Environment.ProcessPath;
            if (!string.IsNullOrWhiteSpace(exe) && File.Exists(exe))
            {
                var icon = Icon.ExtractAssociatedIcon(exe);
                if (icon is not null) return icon;
            }
        }
        catch (Exception ex)
        {
            AppLog.Write("icon: exe fallback failed " + ex.Message);
        }

        return (Icon)SystemIcons.Application.Clone();
    }

    private ContextMenuStrip BuildMenu()
    {
        var menu = new ContextMenuStrip();

        var queueItem = new ToolStripMenuItem("Queue Pop Alert")
        {
            Checked = _settings.QueuePopAlert,
            CheckOnClick = true
        };
        queueItem.CheckedChanged += (_, _) =>
        {
            _settings.QueuePopAlert = queueItem.Checked;
            _settingsStore.Save(_settings);
            AppLog.Write("settings: QueuePopAlert=" + _settings.QueuePopAlert);
        };

        var readyItem = new ToolStripMenuItem("Ready Check Alert")
        {
            Checked = _settings.ReadyCheckAlert,
            CheckOnClick = true
        };
        readyItem.CheckedChanged += (_, _) =>
        {
            _settings.ReadyCheckAlert = readyItem.Checked;
            _settingsStore.Save(_settings);
            AppLog.Write("settings: ReadyCheckAlert=" + _settings.ReadyCheckAlert);
        };

        var notificationItem = new ToolStripMenuItem("Desktop Notification")
        {
            Checked = _settings.DesktopNotification,
            CheckOnClick = true
        };
        notificationItem.CheckedChanged += (_, _) =>
        {
            _settings.DesktopNotification = notificationItem.Checked;
            _settingsStore.Save(_settings);
        };

        var autoLaunchItem = new ToolStripMenuItem("Auto-launch Resonance Logs CN")
        {
            Checked = _settings.AutoLaunchResonanceLogs,
            CheckOnClick = true
        };
        autoLaunchItem.CheckedChanged += (_, _) =>
        {
            _settings.AutoLaunchResonanceLogs = autoLaunchItem.Checked;
            _settingsStore.Save(_settings);
        };

        menu.Items.Add(queueItem);
        menu.Items.Add(readyItem);
        menu.Items.Add(notificationItem);
        menu.Items.Add(autoLaunchItem);

        var adaptersMenu = new ToolStripMenuItem($"Npcap: scanning {_capturePlan.Candidates.Count} adapter(s)");
        foreach (var candidate in _capturePlan.Candidates)
        {
            var label = candidate.Description;
            if (label.Length > 58) label = label[..55] + "...";
            adaptersMenu.DropDownItems.Add(new ToolStripMenuItem($"{label} ({candidate.Source})") { Enabled = false });
        }
        menu.Items.Add(adaptersMenu);
        menu.Items.Add(new ToolStripSeparator());

        var test = new ToolStripMenuItem("Test Alert Sound");
        test.Click += (_, _) => PlayAlert("test");
        menu.Items.Add(test);

        var launch = new ToolStripMenuItem("Launch Resonance Logs CN");
        launch.Click += (_, _) => _launcher.EnsureRunningInteractive();
        menu.Items.Add(launch);

        var changePath = new ToolStripMenuItem("Change Resonance Logs CN Location...");
        changePath.Click += (_, _) => _launcher.ChangeExecutableInteractive();
        menu.Items.Add(changePath);

        var startMenu = new ToolStripMenuItem("Create / Refresh Start Menu Shortcut");
        startMenu.Click += (_, _) =>
        {
            var ok = StartMenuShortcut.TryCreateOrRefresh(_paths.AppIconPath);
            MessageBox.Show(
                ok
                    ? "Start Menu shortcut created with the custom icon. Search for 'BPSR Ready Alert', then right-click it and choose 'Pin to Start'."
                    : "Could not create the Start Menu shortcut. See readyalert.log for details.",
                "BPSR Ready Alert",
                MessageBoxButtons.OK,
                ok ? MessageBoxIcon.Information : MessageBoxIcon.Warning);
        };
        menu.Items.Add(startMenu);

        menu.Items.Add(new ToolStripSeparator());

        var openFolder = new ToolStripMenuItem("Open App Data Folder");
        openFolder.Click += (_, _) => OpenPath(_paths.Root);
        menu.Items.Add(openFolder);

        var openLog = new ToolStripMenuItem("Open Log");
        openLog.Click += (_, _) => OpenLog();
        menu.Items.Add(openLog);

        menu.Items.Add(new ToolStripSeparator());
        var exit = new ToolStripMenuItem("Exit");
        exit.Click += (_, _) => ExitThread();
        menu.Items.Add(exit);

        return menu;
    }

    private void DrainEvents()
    {
        while (_events.TryDequeue(out var evt))
        {
            var enabled = evt.Kind switch
            {
                "queue" => _settings.QueuePopAlert,
                "ready" => _settings.ReadyCheckAlert,
                _ => true
            };

            AppLog.Write($"dispatch: kind={evt.Kind} enabled={enabled}");
            if (!enabled)
            {
                AppLog.Write($"dispatch: skipped kind={evt.Kind} because its alert toggle is OFF");
                continue;
            }

            if (evt.Kind is "queue" or "ready")
                PlayAlert(evt.Kind);

            if (_settings.DesktopNotification || evt.Kind == "error")
            {
                _tray.ShowBalloonTip(
                    4000,
                    evt.Title,
                    evt.Message,
                    evt.Kind == "error" ? ToolTipIcon.Error : ToolTipIcon.Info);
            }
        }
    }

    private void PlayAlert(string reason)
    {
        try
        {
            AppLog.Write($"audio: play requested reason={reason} file={_paths.AlertSoundPath}");
            _player.Stop();
            _player.Play();
            AppLog.Write($"audio: play submitted reason={reason}");
        }
        catch (Exception ex)
        {
            AppLog.Write("audio: play failed " + ex.Message);
            SystemSounds.Exclamation.Play();
        }
    }

    private void OpenLog()
    {
        try
        {
            if (!File.Exists(_paths.LogPath)) File.WriteAllText(_paths.LogPath, "No log entries yet." + Environment.NewLine);
            Process.Start(new ProcessStartInfo("notepad.exe", $"\"{_paths.LogPath}\"") { UseShellExecute = true });
        }
        catch (Exception ex)
        {
            AppLog.Write("ui: open log failed " + ex.Message);
        }
    }

    private static void OpenPath(string path)
    {
        try
        {
            Process.Start(new ProcessStartInfo("explorer.exe", $"\"{path}\"") { UseShellExecute = true });
        }
        catch (Exception ex)
        {
            AppLog.Write("ui: open path failed " + ex.Message);
        }
    }

    protected override void ExitThreadCore()
    {
        _timer.Stop();
        _engine.Dispose();
        _player.Dispose();
        _tray.Visible = false;
        _tray.Dispose();
        _appIcon.Dispose();
        AppLog.Write("shutdown: normal");
        base.ExitThreadCore();
    }
}
