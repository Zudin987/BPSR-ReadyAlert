using System.Collections.Concurrent;
using System.Diagnostics;
using System.Drawing;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed class TrayApplicationContext : ApplicationContext
{
    private const int MaxChatUiMessagesPerTick = 200;

    private readonly AppPaths _paths;
    private readonly AppSettings _settings;
    private readonly SettingsStore _settingsStore;
    private readonly ResonanceLogsLauncher _launcher;
    private NpcapCapturePlan _capturePlan;
    private readonly NotifyIcon _tray;
    private readonly System.Windows.Forms.Timer _timer;
    private readonly ConcurrentQueue<AlertEvent> _events = new();
    private readonly ConcurrentQueue<ChatMessageEvent> _chatEvents = new();
    private CaptureEngine _engine;
    private ChatOverlayForm? _chatWindow;
    private readonly CoreAlertAudioPlayer _player;
    private readonly Icon _appIcon;
    private ToolStripMenuItem? _adapterMenu;
    private ToolStripMenuItem? _volumeMenu;
    private ToolStripMenuItem? _chatMenuItem;
    private bool _updatingChatToggle;

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
        _player = new CoreAlertAudioPlayer(_paths, _settings.AlertVolume);

        // NotifyIcon is a small-icon surface. Explicitly request the exact 16x16
        // frame from App.ico instead of relying on Windows/System.Drawing to pick
        // a larger frame and scale it down.
        _appIcon = LoadApplicationIcon(_paths.AppIconPath, 16, 16);
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
            $"partyInvite={_settings.PartyInviteAlert} partyRequest={_settings.PartyRequestAlert} " +
            $"notification={_settings.DesktopNotification} chat={_settings.ChatOverlayEnabled} " +
            $"autoLaunch={_settings.AutoLaunchResonanceLogs} volume={_settings.AlertVolume}% " +
            $"adapter={_settings.NpcapDeviceName}");
        AppLog.Write($"capture: selected adapter={_capturePlan.Primary.Description} source={_capturePlan.Primary.Source}");

        // Party invite/request alerts are core ReadyAlert consumers. They reuse the
        // existing capture path but have independent toggles and dedicated sounds.
        PartyAlertCaptureBridge.Configure(_events);
        ChatCaptureBridge.Configure(_chatEvents);
        ChatCaptureBridge.Enabled = _settings.ChatOverlayEnabled;

        _engine = new CaptureEngine(_events, _capturePlan);
        _engine.Start();

        if (_settings.ChatOverlayEnabled)
            EnsureChatWindow().ShowOverlay();

        _timer = new System.Windows.Forms.Timer { Interval = 25 };
        _timer.Tick += (_, _) => DrainEvents();
        _timer.Start();

        AppLog.Write("tray: running");
    }

    private static Icon LoadApplicationIcon(string iconPath, int width, int height)
    {
        try
        {
            if (File.Exists(iconPath))
            {
                using var stream = new FileStream(iconPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite);
                using var icon = new Icon(stream, width, height);
                AppLog.Write($"icon: loaded custom icon {width}x{height} {iconPath}");
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
                using var associated = Icon.ExtractAssociatedIcon(exe);
                if (associated is not null)
                    return new Icon(associated, width, height);
            }
        }
        catch (Exception ex)
        {
            AppLog.Write("icon: exe fallback failed " + ex.Message);
        }

        return new Icon(SystemIcons.Application, width, height);
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

        var partyInviteItem = new ToolStripMenuItem("Party Invite Alert")
        {
            Checked = _settings.PartyInviteAlert,
            CheckOnClick = true
        };
        partyInviteItem.CheckedChanged += (_, _) =>
        {
            _settings.PartyInviteAlert = partyInviteItem.Checked;
            _settingsStore.Save(_settings);
            AppLog.Write("settings: PartyInviteAlert=" + _settings.PartyInviteAlert);
        };

        var partyRequestItem = new ToolStripMenuItem("Party Request Alert")
        {
            Checked = _settings.PartyRequestAlert,
            CheckOnClick = true
        };
        partyRequestItem.CheckedChanged += (_, _) =>
        {
            _settings.PartyRequestAlert = partyRequestItem.Checked;
            _settingsStore.Save(_settings);
            AppLog.Write("settings: PartyRequestAlert=" + _settings.PartyRequestAlert);
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

        _chatMenuItem = new ToolStripMenuItem("Chat Overlay")
        {
            Checked = _settings.ChatOverlayEnabled,
            CheckOnClick = true
        };
        _chatMenuItem.CheckedChanged += (_, _) =>
        {
            if (_updatingChatToggle) return;
            SetChatOverlayEnabled(_chatMenuItem.Checked);
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
        menu.Items.Add(partyInviteItem);
        menu.Items.Add(partyRequestItem);
        menu.Items.Add(notificationItem);
        menu.Items.Add(_chatMenuItem);
        menu.Items.Add(autoLaunchItem);

        _adapterMenu = new ToolStripMenuItem();
        RefreshAdapterMenu();
        menu.Items.Add(_adapterMenu);

        _volumeMenu = new ToolStripMenuItem();
        RefreshVolumeMenu();
        menu.Items.Add(_volumeMenu);
        menu.Items.Add(new ToolStripSeparator());

        var test = new ToolStripMenuItem("Test Alert Sounds");
        var testQueue = new ToolStripMenuItem("Queue Pop");
        testQueue.Click += (_, _) => PlayAlert("queue");
        var testReady = new ToolStripMenuItem("Ready Check");
        testReady.Click += (_, _) => PlayAlert("ready");
        var testInvite = new ToolStripMenuItem("Party Invite");
        testInvite.Click += (_, _) => PlayAlert("party-invite");
        var testRequest = new ToolStripMenuItem("Party Request");
        testRequest.Click += (_, _) => PlayAlert("party-request");
        test.DropDownItems.Add(testQueue);
        test.DropDownItems.Add(testReady);
        test.DropDownItems.Add(testInvite);
        test.DropDownItems.Add(testRequest);
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

        menu.Opening += (_, _) => RefreshChatMenuState();
        RefreshChatMenuState();
        return menu;
    }

    private ChatOverlayForm EnsureChatWindow()
    {
        if (_chatWindow is { IsDisposed: false }) return _chatWindow;

        _chatWindow = new ChatOverlayForm(
            _settings,
            _settingsStore,
            _paths.AppIconPath,
            _paths.AlertSoundPath);
        return _chatWindow;
    }

    private void SetChatOverlayEnabled(bool enabled)
    {
        if (_updatingChatToggle) return;
        _updatingChatToggle = true;
        try
        {
            if (_settings.ChatOverlayEnabled != enabled)
            {
                _settings.ChatOverlayEnabled = enabled;
                _settingsStore.Save(_settings);
                AppLog.Write("settings: ChatOverlayEnabled=" + enabled);
            }

            if (_chatMenuItem is not null && _chatMenuItem.Checked != enabled)
                _chatMenuItem.Checked = enabled;

            ChatCaptureBridge.Enabled = enabled;

            if (enabled)
            {
                EnsureChatWindow().ShowOverlay();
                AppLog.Write("chat: shared notify processing enabled");
            }
            else
            {
                while (_chatEvents.TryDequeue(out _)) { }
                if (_chatWindow is { IsDisposed: false })
                    _chatWindow.Shutdown();
                _chatWindow = null;
                AppLog.Write("chat: shared notify processing disabled");
            }

            RefreshChatMenuState();
        }
        finally
        {
            _updatingChatToggle = false;
        }
    }

    private void RefreshChatMenuState()
    {
        if (_chatMenuItem is not null && _chatMenuItem.Checked != _settings.ChatOverlayEnabled)
        {
            _updatingChatToggle = true;
            try { _chatMenuItem.Checked = _settings.ChatOverlayEnabled; }
            finally { _updatingChatToggle = false; }
        }
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

        var oldSetting = _settings.NpcapDeviceName;
        var oldPlan = _capturePlan;
        var previousEngine = _engine;
        var oldCaptureStopped = false;
        _settings.NpcapDeviceName = deviceName;

        try
        {
            var candidatePlan = NpcapDeviceSelector.SelectPlan(_settings);

            // If Auto/Resonance Logs resolves to the adapter already being captured,
            // only the preference metadata changed; keep the live capture untouched.
            if (string.Equals(candidatePlan.Primary.DeviceName, oldPlan.Primary.DeviceName, StringComparison.OrdinalIgnoreCase))
            {
                if (!_settingsStore.Save(_settings))
                {
                    _settings.NpcapDeviceName = oldSetting;
                    _capturePlan = oldPlan;
                    RefreshAdapterMenu();
                    _events.Enqueue(new AlertEvent("error", "BPSR Ready Alert", "Could not save the new adapter preference. The previous preference was kept."));
                    return;
                }

                _capturePlan = candidatePlan;
                RefreshAdapterMenu();
                AppLog.Write($"capture: adapter preference changed without restart; active={candidatePlan.Primary.Description} source={candidatePlan.Primary.Source}");
                return;
            }

            // Prove the requested adapter can actually activate before tearing down
            // the currently working capture. This is a short validation handle only,
            // not a second packet-processing pipeline.
            using (var probe = new NpcapCapture(candidatePlan.Primary.DeviceName))
                AppLog.Write($"capture: adapter preflight ok device={candidatePlan.Primary.Description} datalink={probe.DataLink}");

            previousEngine.Dispose();
            oldCaptureStopped = true;

            var replacement = new CaptureEngine(_events, candidatePlan);
            try
            {
                replacement.Start();
                _engine = replacement;
            }
            catch
            {
                replacement.Dispose();
                throw;
            }

            _capturePlan = candidatePlan;
            var saved = _settingsStore.Save(_settings);
            RefreshAdapterMenu();
            AppLog.Write($"capture: switched adapter={_capturePlan.Primary.Description} source={_capturePlan.Primary.Source}");
            if (!saved)
            {
                _events.Enqueue(new AlertEvent(
                    "error",
                    "BPSR Ready Alert",
                    "The adapter switched for this session, but the preference could not be saved. It may revert after restart."));
            }
        }
        catch (Exception ex)
        {
            _settings.NpcapDeviceName = oldSetting;
            _capturePlan = oldPlan;
            _settingsStore.Save(_settings);

            // The normal failure path occurs during preflight, while the old engine
            // is still alive. If an unexpected failure happened after disposal,
            // stop any partial replacement and restart the known-good plan.
            if (oldCaptureStopped)
            {
                try
                {
                    if (!ReferenceEquals(_engine, previousEngine))
                        _engine.Dispose();
                }
                catch (Exception disposeEx)
                {
                    AppLog.Write("capture: failed disposing partial replacement " + disposeEx.Message);
                }

                try
                {
                    _engine = new CaptureEngine(_events, oldPlan);
                    _engine.Start();
                    AppLog.Write($"capture: rollback restored adapter={oldPlan.Primary.Description}");
                }
                catch (Exception rollbackEx)
                {
                    AppLog.Write("capture: adapter rollback failed " + rollbackEx);
                    _events.Enqueue(new AlertEvent(
                        "error",
                        "BPSR Ready Alert",
                        "Adapter switch failed and ReadyAlert could not restart the previous capture. Restart ReadyAlert. " + rollbackEx.Message));
                    RefreshAdapterMenu();
                    return;
                }
            }

            RefreshAdapterMenu();
            AppLog.Write("capture: adapter switch rejected; restored previous selection. " + ex);
            _events.Enqueue(new AlertEvent(
                "error",
                "BPSR Ready Alert",
                "Could not switch Npcap adapter. ReadyAlert kept the previous adapter. " + ex.Message));
        }
    }

    private void RefreshVolumeMenu()
    {
        if (_volumeMenu is null) return;
        _volumeMenu.Text = ReadyQueueVolumeMenuText(_settings.AlertVolume);
        _volumeMenu.ToolTipText = "Queue Pop, Ready Check, Party Invite and Party Request sounds. Chat alert and TTS volumes are separate.";
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

    internal static string ReadyQueueVolumeMenuText(int volume) =>
        $"Ready / Queue / Party Volume: {Math.Clamp(volume, 0, 100)}%";

    internal static int ChatUiDrainLimitForSelfTest => MaxChatUiMessagesPerTick;

    internal static string[] CoreAlertMenuOrderForSelfTest =>
        ["Queue Pop Alert", "Ready Check Alert", "Party Invite Alert", "Party Request Alert"];

    internal static bool LegacyShowHideChatItemsForSelfTest => false;

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
            var enabled = IsAlertEnabled(_settings, evt.Kind);

            AppLog.Write($"dispatch: kind={evt.Kind} enabled={enabled}");
            if (!enabled)
            {
                AppLog.Write($"dispatch: skipped kind={evt.Kind} because its alert toggle is OFF");
                continue;
            }

            if (IsCoreSoundEvent(evt.Kind))
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

        if (!_settings.ChatOverlayEnabled)
        {
            while (_chatEvents.TryDequeue(out _)) { }
            return;
        }

        var chatWindow = EnsureChatWindow();
        var drained = 0;
        while (drained < MaxChatUiMessagesPerTick && _chatEvents.TryDequeue(out var chat))
        {
            chatWindow.AddMessage(chat);
            drained++;
        }

        // AddMessage coalesces expensive ListBox rebuilds. Flush at our per-tick
        // boundary so a sustained queue still paints progress instead of looking hung.
        chatWindow.FlushDeferredMessageBatch();
    }

    private static bool IsAlertEnabled(AppSettings settings, string kind) => kind switch
    {
        "queue" => settings.QueuePopAlert,
        "ready" => settings.ReadyCheckAlert,
        "party-invite" => settings.PartyInviteAlert,
        "party-request" => settings.PartyRequestAlert,
        _ => true
    };

    internal static bool IsAlertEnabledForSelfTest(AppSettings settings, string kind) =>
        IsAlertEnabled(settings, kind);

    private static bool IsCoreSoundEvent(string kind) =>
        kind is "queue" or "ready" or "party-invite" or "party-request";

    internal static bool IsCoreSoundEventForSelfTest(string kind) => IsCoreSoundEvent(kind);

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
            // Never replace a failed core alert sound with a Windows SystemSound:
            // it has an unrelated mixer volume and can violate the user's setting.
            AppLog.Write("audio: play failed " + ex.Message);
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
        ChatCaptureBridge.Enabled = false;
        _engine.Dispose();
        _chatWindow?.Shutdown();
        _player.Dispose();
        _tray.Visible = false;
        _tray.Dispose();
        _appIcon.Dispose();
        AppLog.Write("shutdown: normal");
        base.ExitThreadCore();
    }
}
