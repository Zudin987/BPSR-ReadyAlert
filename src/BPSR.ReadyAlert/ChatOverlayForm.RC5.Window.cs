using System.Drawing;
using System.Media;
using System.Windows.Forms;

namespace BPSR.ReadyAlert;

internal sealed partial class ChatOverlayForm
{
    private void HandleMessageNotification(ChatMessageEvent message)
    {
        if (_settings.Chat.BlockedUsers.Any(x => x.Id != 0 && x.Id == message.SenderId)) return;
        if (_settings.Chat.HideStickers && message.Kind == ChatMessageKind.Sticker) return;

        // Private/Talk keeps its dedicated sound priority. Otherwise evaluate the
        // three user sound rules in order and stop at the first match so one chat
        // message never launches multiple notification sounds at once.
        if (message.Channel == ChatChannel.Private && _settings.Chat.PrivateSoundEnabled)
        {
            PlayChatSound(_settings.Chat.PrivateSoundPath, "private");
            return;
        }

        var searchable = ChatTabFilter.SearchableText(message);
        for (var i = 0; i < _settings.Chat.HighlightSoundRules.Count && i < 3; i++)
        {
            var rule = _settings.Chat.HighlightSoundRules[i];
            if (!rule.Enabled || string.IsNullOrWhiteSpace(rule.Match)) continue;
            if (!ChatFilterExpression.IsMatch(searchable, rule.Match)) continue;

            PlayChatSound(rule.SoundPath, $"rule-{i + 1}");
            return;
        }
    }

    private void PlayChatSound(string configuredPath, string reason)
    {
        // There is intentionally no user cooldown for RC9. The timestamp is kept
        // only for diagnostics/future troubleshooting and does not suppress sound.
        _lastSoundUtc = DateTime.UtcNow;
        var preferredPath = !string.IsNullOrWhiteSpace(configuredPath) && File.Exists(configuredPath)
            ? configuredPath
            : _defaultSoundPath;
        ChatSoundVolumePlayer.Play(preferredPath, _defaultSoundPath, _settings.Chat.ChatSoundVolume, reason);
    }

    private void RegisterHotkeys(bool showErrors)
    {
        if (!IsHandleCreated) return;
        UnregisterHotkeys();
        var problems = new List<string>();

        ChatHotkeyGesture clickGesture = default;
        var clickValid = ChatHotkey.TryParse(_settings.Chat.ClickThroughHotkey, out clickGesture, out var clickError);
        if (clickValid)
        {
            _clickThroughRegistered = ChatNativeMethods.RegisterHotKey(Handle, ClickThroughHotkeyId, clickGesture.NativeModifiers, (uint)clickGesture.Key);
            if (!_clickThroughRegistered) problems.Add("Click-through hotkey is already in use by another app.");
        }
        else problems.Add("Click-through hotkey: " + clickError);

        if (ChatHotkey.TryParse(_settings.Chat.CollapseHotkey, out var collapseGesture, out var collapseError))
        {
            if (clickValid && clickGesture.Equals(collapseGesture)) problems.Add("Click-through and collapse hotkeys cannot be the same.");
            else
            {
                _collapseRegistered = ChatNativeMethods.RegisterHotKey(Handle, CollapseHotkeyId, collapseGesture.NativeModifiers, (uint)collapseGesture.Key);
                if (!_collapseRegistered) problems.Add("Collapse hotkey is already in use by another app.");
            }
        }
        else problems.Add("Collapse hotkey: " + collapseError);

        if (!_clickThroughRegistered && _settings.Chat.ClickThrough)
        {
            _settings.Chat.ClickThrough = false;
            _settingsStore.Save(_settings);
            ApplyClickThrough();
            problems.Add("Click-through was turned OFF so the overlay cannot become mouse-locked without a working recovery hotkey.");
        }

        foreach (var problem in problems) AppLog.Write("chat: hotkey " + problem);
        if (showErrors && problems.Count > 0)
        {
            MessageBox.Show(this,
                string.Join(Environment.NewLine, problems) + Environment.NewLine + Environment.NewLine +
                "Change these shortcuts in Chat Overlay Settings > Interaction.",
                "Chat Overlay hotkey",
                MessageBoxButtons.OK,
                MessageBoxIcon.Warning);
        }
    }

    private void UnregisterHotkeys()
    {
        if (!IsHandleCreated) return;
        if (_clickThroughRegistered)
        {
            _ = ChatNativeMethods.UnregisterHotKey(Handle, ClickThroughHotkeyId);
            _clickThroughRegistered = false;
        }
        if (_collapseRegistered)
        {
            _ = ChatNativeMethods.UnregisterHotKey(Handle, CollapseHotkeyId);
            _collapseRegistered = false;
        }
    }

    private void ToggleClickThrough()
    {
        _settings.Chat.ClickThrough = !_settings.Chat.ClickThrough;
        _settingsStore.Save(_settings);
        ApplyClickThrough();
        AppLog.Write("chat: click-through=" + _settings.Chat.ClickThrough);
    }

    private void ApplyClickThrough()
    {
        if (!IsHandleCreated) return;
        if (!ChatNativeMethods.SetClickThrough(Handle, _settings.Chat.ClickThrough))
            AppLog.Write("chat: failed to change click-through window style");
    }

    private void ToggleCollapsed()
    {
        if (_collapsed) ExpandFromEdge();
        else CollapseToEdge();
    }

    private void CollapseToEdge()
    {
        if (_collapsed) return;
        _expandedBounds = Bounds;
        SaveWindowPlacement();
        var screen = Screen.FromRectangle(Bounds).WorkingArea;
        var side = _settings.Chat.CollapseSide;
        _topPanel.Visible = false;
        _messages.Visible = false;
        _emptyState.Visible = false;
        _newMessagesButton.Visible = false;
        _collapsedHandle.Visible = true;
        MinimumSize = Size.Empty;

        if (side == "Left" || side == "Right")
        {
            var height = Math.Min(_expandedBounds.Height, screen.Height);
            var y = Math.Clamp(_expandedBounds.Top, screen.Top, Math.Max(screen.Top, screen.Bottom - height));
            var x = side == "Left" ? screen.Left : screen.Right - CollapsedThickness;
            Bounds = new Rectangle(x, y, CollapsedThickness, height);
        }
        else
        {
            var width = Math.Min(_expandedBounds.Width, screen.Width);
            var x = Math.Clamp(_expandedBounds.Left, screen.Left, Math.Max(screen.Left, screen.Right - width));
            var y = side == "Top" ? screen.Top : screen.Bottom - CollapsedThickness;
            Bounds = new Rectangle(x, y, width, CollapsedThickness);
        }

        _collapsed = true;
        _collapsedHandle.Text = side switch
        {
            "Left" => "▶",
            "Top" => "▼",
            "Bottom" => "▲",
            _ => "◀"
        };
        AppLog.Write("chat: collapsed side=" + side);
    }

    private void ExpandFromEdge()
    {
        if (!_collapsed) return;
        _collapsed = false;
        _collapsedHandle.Visible = false;
        MinimumSize = new Size(420, 220);
        Bounds = _expandedBounds;
        _topPanel.Visible = true;
        RebuildVisibleMessages(keepScroll: true);
        UpdateNewMessagesButton();
        UpdateEmptyState();
        AppLog.Write("chat: expanded");
    }

    private void UpdateCollapseButtonGlyph()
    {
        _collapseButton.Text = _settings.Chat.CollapseSide switch
        {
            "Left" => "◀",
            "Top" => "▲",
            "Bottom" => "▼",
            _ => "▶"
        };
    }

    private string GetTimeText(DateTime timestamp)
    {
        if (!_settings.Chat.ShowTimeAsAgo) return timestamp.ToString("HH:mm");
        var age = DateTime.Now - timestamp;
        if (age.TotalSeconds < 0) age = TimeSpan.Zero;
        if (age.TotalSeconds < 60) return $"{Math.Max(0, (int)age.TotalSeconds)}s";
        if (age.TotalMinutes < 60) return $"{(int)age.TotalMinutes}m";
        if (age.TotalHours < 24) return $"{(int)age.TotalHours}h";
        return timestamp.ToString("MM-dd HH:mm");
    }

    private static string DisplaySenderName(ChatMessageEvent message)
    {
        if (!string.IsNullOrWhiteSpace(message.SenderName)) return message.SenderName;
        return message.SenderId != 0 ? message.SenderId.ToString() : "System";
    }

    private static string GetChannelName(ChatChannel channel) => channel switch
    {
        ChatChannel.Null => "Other",
        ChatChannel.World => "World",
        ChatChannel.Local => "Local",
        ChatChannel.Team => "Team",
        ChatChannel.Union => "Guild",
        ChatChannel.Private => "Private",
        ChatChannel.Group => "Group",
        ChatChannel.TopNotice => "Notice",
        ChatChannel.Play => "Play",
        ChatChannel.Newbie => "Newbie",
        ChatChannel.System => "System",
        _ => "Other"
    };

    private Color GetChannelColor(ChatChannel channel)
    {
        var defaults = ChatOverlaySettings.CreateDefaultChannelColors();
        var key = (int)channel;
        var fallback = ChatColorUtil.Parse(defaults.TryGetValue(key, out var defaultHex) ? defaultHex : "#D3D3D3", Color.LightGray);
        return _settings.Chat.ChannelColors.TryGetValue(key, out var value) ? ChatColorUtil.Parse(value, fallback) : fallback;
    }

    private void RestoreWindowPlacement()
    {
        if (_settings.Chat.WindowX == int.MinValue || _settings.Chat.WindowY == int.MinValue)
        {
            PlaceAtPrimaryScreenBottomRight();
            return;
        }
        var desired = new Rectangle(_settings.Chat.WindowX, _settings.Chat.WindowY,
            Math.Max(420, _settings.Chat.WindowWidth), Math.Max(220, _settings.Chat.WindowHeight));
        var visible = Screen.AllScreens.Any(s => s.WorkingArea.IntersectsWith(desired));
        if (visible) Bounds = desired;
        else PlaceAtPrimaryScreenBottomRight();
    }

    private void PlaceAtPrimaryScreenBottomRight()
    {
        var area = Screen.PrimaryScreen?.WorkingArea ?? new Rectangle(0, 0, 1920, 1080);
        Location = new Point(Math.Max(area.Left, area.Right - Width - 40), Math.Max(area.Top, area.Bottom - Height - 80));
    }

    private void SaveWindowPlacement()
    {
        var bounds = _collapsed ? _expandedBounds : Bounds;
        if (WindowState == FormWindowState.Normal && bounds.Width >= 420 && bounds.Height >= 220)
        {
            _settings.Chat.WindowX = bounds.Left;
            _settings.Chat.WindowY = bounds.Top;
            _settings.Chat.WindowWidth = bounds.Width;
            _settings.Chat.WindowHeight = bounds.Height;
        }
        _settingsStore.Save(_settings);
    }

    protected override void Dispose(bool disposing)
    {
        if (disposing && !_disposedResources)
        {
            _disposedResources = true;
            try { _relativeTimer.Stop(); } catch { }
            try { _resizeTimer.Stop(); } catch { }
            _messageFont?.Dispose();
            _messageBoldFont?.Dispose();
            _senderFont?.Dispose();
            _metaFont?.Dispose();
            _chatSoundPlayer.Dispose();
            _toolTip.Dispose();
        }
        base.Dispose(disposing);
    }
}
