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
    private NpcapCapturePlan _capturePlan;
    private readonly NotifyIcon _tray;
    private readonly System.Windows.Forms.Timer _timer;
    private readonly ConcurrentQueue<AlertEvent> _events = new();
    private CaptureEngine _engine;
    private readonly AlertAudioPlayer _player;
    private readonly Icon _appIcon;
    private ToolStripMenuItem? _adapterMenu;
    private ToolStripMenuItem? _volumeMenu;

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
        _player = new AlertAudioPlayer(_paths.AlertSoundPath, _settings.AlertVolume);

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
            $"notification={_settings.DesktopNotification} autoLaunch={_settings.AutoLaunchResonanceLogs} " +
            $"volume={_settings.AlertVolume}% adapter={_settings.NpcapDeviceName}");
        AppLog.Write($"capture: selected adapter={_capturePlan.Primary.Description} source={_capturePlan.Primary.Source}");

        _engine = new CaptureEngine(_events, _capturePlan);
        _engine.Start();

        _timer = new System.Windows.Forms.Timer { Interval = 25 };
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

        _adapterMenu = new ToolStripMenuItem();
        RefreshAdapterMenu();
        menu.Items.Add(_adapterMenu);

        _volumeMenu = new ToolStripMenuItem();
        RefreshVolumeMenu();
        menu.Items.Add(_volumeMenu);
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

    private void RefreshAdapterMenu()
    {
        if (_adapterMenu is null) return;

        var description = Shorten(_capturePlan.Primary.Description, 38);
        _adapterMenu.Text = $"Network Adapter: {description}";
        _adapterMenu.DropDownItems.Clear();

        var auto = new ToolStripMenuItem("Follow Resonance Logs CN / Auto")
        {
            Checked = string.IsNullOrWhiteSpace(_settings.NpcapDeviceName)
        };
        auto.Click += (_, _) => SelectAdapter(string.Empty);
        _adapterMenu.DropDownItems.Add(auto);
        _adapterMenu.DropDownItems.Add(new ToolStripSeparator());

        foreach (var device in _capturePlan.AvailableDevices)
        {
            var label = Shorten(device.Description, 64);
            var item = new ToolStripMenuItem(label)
            {
                Checked = !string.IsNullOrWhiteSpace(_settings.NpcapDeviceName) &&
                          string.Equals(_settings.NpcapDeviceName, device.Name, StringComparison.OrdinalIgnoreCase),
                ToolTipText = device.Name
            };
            var deviceName = device.Name;
            item.Click += (_, _) => SelectAdapter(deviceName);
            _adapterMenu.DropDownItems.Add(item);
        }
    }

    private void SelectAdapter(string deviceName)
    {
        if (string.Equals(_settings.NpcapDeviceName, deviceName, StringComparison.OrdinalIgnoreCase))
            return;

        _settings.NpcapDeviceName = deviceName;
        _settingsStore.Save(_settings);
        AppLog.Write("settings: NpcapDeviceName=" + (string.IsNullOrWhiteSpace(deviceName) ? "<auto>" : deviceName));

        try
        {
            _engine.Dispose();
            _capturePlan = NpcapDeviceSelector.SelectPlan(_settings);
            _engine = new CaptureEngine(_events, _capturePlan);
            _engine.Start();
            RefreshAdapterMenu();
            AppLog.Write($"capture: switched adapter={_capturePlan.Primary.Description} source={_capturePlan.Primary.Source}");
        }
        catch (Exception ex)
        {
            AppLog.Write("capture: adapter switch failed " + ex);
            _events.Enqueue(new AlertEvent("error", "BPSR Ready Alert", "Could not switch Npcap adapter: " + ex.Message));
        }
    }

    private void RefreshVolumeMenu()
    {
        if (_volumeMenu is null) return;
        _volumeMenu.Text = $"Alert Volume: {_settings.AlertVolume}%";
        _volumeMenu.DropDownItems.Clear();

        foreach (var volume in Enumerable.Range(0, 11).Select(i => i * 10))
        {
            var value = volume;
            var item = new ToolStripMenuItem(value == 0 ? "Mute" : $"{value}%")
            {
                Checked = value == _settings.AlertVolume
            };
            item.Click += (_, _) => SetVolume(value);
            _volumeMenu.DropDownItems.Add(item);
        }
    }

    private void SetVolume(int volume)
    {
        _settings.AlertVolume = Math.Clamp(volume, 0, 100);
        _settingsStore.Save(_settings);
        _player.Volume = _settings.AlertVolume;
        RefreshVolumeMenu();
        AppLog.Write("settings: AlertVolume=" + _settings.AlertVolume);
    }

    private static string Shorten(string value, int max) =>
        value.Length <= max ? value : value[..(max - 3)] + "...";

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
                var (title, message) = FormatDesktopNotification(evt);
                _tray.ShowBalloonTip(
                    4000,
                    title,
                    message,
                    evt.Kind == "error" ? ToolTipIcon.Error : ToolTipIcon.Info);
            }
        }
    }

    private static (string Title, string Message) FormatDesktopNotification(AlertEvent evt)
    {
        if (evt.Kind == "queue")
            return (string.Empty, "BPSR Party Ready Confirm");

        if (evt.Kind == "ready")
        {
            if (string.Equals(evt.Title, "BPSR Ready Check", StringComparison.Ordinal))
                return (string.Empty, "BPSR FoodSerum Ready Check");

            if (string.Equals(evt.Title, "BPSR Party Ready Vote", StringComparison.Ordinal))
                return (string.Empty, "BPSR Party Ready Confirm");
        }

        return (evt.Title, evt.Message);
    }

    private void PlayAlert(string reason)
    {
        try
        {
            _player.Play(reason);
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
