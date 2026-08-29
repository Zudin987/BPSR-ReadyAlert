namespace BPSR.ReadyAlert;

/// <summary>
/// Owns the four low-latency PCM players used by core ReadyAlert events. They share
/// one user-facing volume setting while keeping a distinct bundled sound per event.
/// Only one core sound is allowed to play at once, matching the historical single-
/// player behavior and preventing rapid Queue/Ready/Party events from overlapping.
/// </summary>
internal sealed class CoreAlertAudioPlayer : IDisposable
{
    private readonly object _sync = new();
    private readonly AlertAudioPlayer _queue;
    private readonly AlertAudioPlayer _ready;
    private readonly AlertAudioPlayer _partyInvite;
    private readonly AlertAudioPlayer _partyRequest;
    private AlertAudioPlayer? _active;
    private int _volume;

    internal CoreAlertAudioPlayer(AppPaths paths, int volume)
    {
        _queue = new AlertAudioPlayer(paths.QueueSoundPath, volume);
        _ready = new AlertAudioPlayer(paths.ReadyCheckSoundPath, volume);
        _partyInvite = new AlertAudioPlayer(paths.PartyInviteSoundPath, volume);
        _partyRequest = new AlertAudioPlayer(paths.PartyRequestSoundPath, volume);
        _volume = Math.Clamp(volume, 0, 100);
    }

    internal int Volume
    {
        get { lock (_sync) return _volume; }
        set
        {
            lock (_sync)
            {
                _volume = Math.Clamp(value, 0, 100);
                _queue.Volume = _volume;
                _ready.Volume = _volume;
                _partyInvite.Volume = _volume;
                _partyRequest.Volume = _volume;
            }
        }
    }

    internal void Play(string kind)
    {
        lock (_sync)
        {
            var player = kind switch
            {
                "queue" => _queue,
                "ready" => _ready,
                "party-invite" => _partyInvite,
                "party-request" => _partyRequest,
                _ => null
            };

            if (player is null)
            {
                AppLog.Write("audio: no core sound mapped for kind=" + kind);
                return;
            }

            if (!ReferenceEquals(_active, player))
                _active?.Stop();
            _active = player;
            player.Play(kind);
        }
    }

    internal static string SoundKeyForSelfTest(string kind) => kind switch
    {
        "queue" => "Queue",
        "ready" => "ReadyCheck",
        "party-invite" => "PartyInvite",
        "party-request" => "PartyRequest",
        _ => string.Empty
    };

    public void Dispose()
    {
        lock (_sync)
        {
            _active?.Stop();
            _active = null;
            _queue.Dispose();
            _ready.Dispose();
            _partyInvite.Dispose();
            _partyRequest.Dispose();
        }
    }
}
