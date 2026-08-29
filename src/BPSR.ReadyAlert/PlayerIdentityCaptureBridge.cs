using System.Collections.Concurrent;
using System.Text;

namespace BPSR.ReadyAlert;

internal readonly record struct DetectedPlayerIdentity(
    string Name,
    long CharacterUid,
    DateTime DetectedUtc);

/// <summary>
/// Reads the local character identity from the same already-reassembled/decompressed
/// WorldNtf stream used by ReadyAlert. This intentionally owns no Npcap handle and
/// performs no network work of its own.
/// </summary>
internal static class PlayerIdentityCaptureBridge
{
    private const ulong WorldNtfService = 1_664_308_034UL;
    private const uint EnterSceneMethod = 0x03;
    private const int NameAttributeId = 0x01;
    private const int MaxUsernameLength = 128;

    private static readonly object Gate = new();
    private static DetectedPlayerIdentity? _current;
    private static ChatSpeechTranslationSettings? _speechTemplate;
    private static ConcurrentQueue<ChatTranslationResult>? _translationResults;
    private static int _loggedParseFailure;

    internal static event Action<DetectedPlayerIdentity?>? IdentityChanged;

    internal static DetectedPlayerIdentity? Current
    {
        get
        {
            lock (Gate) return _current;
        }
    }

    internal static bool TryHandle(ulong service, uint method, byte[] payload)
    {
        if (service != WorldNtfService || method != EnterSceneMethod)
            return false;

        if (!TryParseEnterScene(payload, out var identity))
        {
            if (Interlocked.Exchange(ref _loggedParseFailure, 1) == 0)
                AppLog.Write($"identity: first EnterScene parse failure protoLen={payload.Length}");
            return true;
        }

        if (Publish(identity))
            AppLog.Write($"identity: local player detected uid={identity.CharacterUid} nameLength={identity.Name.Length}");
        return true;
    }

    /// <summary>
    /// Centralizes speech-engine configuration so the persisted/manual username stays
    /// separate from the transient auto-detected name. Existing translation result
    /// routing is retained when detection refreshes the TTS filter from the capture
    /// thread.
    /// </summary>
    internal static void ConfigureSpeechEngine(
        ChatSpeechTranslationSettings settings,
        ConcurrentQueue<ChatTranslationResult>? translationResults = null)
    {
        ArgumentNullException.ThrowIfNull(settings);
        var template = CloneSpeechSettings(settings, settings.IgnoreOwnUsername);
        template.Normalize();

        DetectedPlayerIdentity? detected;
        ConcurrentQueue<ChatTranslationResult>? results;
        lock (Gate)
        {
            _speechTemplate = template;
            if (translationResults is not null)
                _translationResults = translationResults;
            detected = _current;
            results = _translationResults;
        }

        var runtime = CloneSpeechSettings(
            template,
            EffectiveUsername(template.IgnoreOwnUsername, detected));
        ChatSpeechTranslationEngine.Configure(runtime, results);
    }

    internal static bool IsOwnUsername(string? senderName, string? manualOverride)
    {
        var expected = EffectiveUsername(manualOverride, Current);
        return expected.Length > 0 &&
               string.Equals(expected, senderName?.Trim(), StringComparison.OrdinalIgnoreCase);
    }

    internal static string EffectiveUsername(string? manualOverride) =>
        EffectiveUsername(manualOverride, Current);

    internal static void ClearCurrent()
    {
        Action<DetectedPlayerIdentity?>? handlers;
        lock (Gate)
        {
            if (_current is null) return;
            _current = null;
            handlers = IdentityChanged;
        }

        NotifyHandlers(handlers, null);
        RefreshSpeechEngine();
    }

    private static bool Publish(DetectedPlayerIdentity identity)
    {
        identity = identity with
        {
            Name = NormalizeUsername(identity.Name),
            DetectedUtc = DateTime.UtcNow
        };
        if (identity.Name.Length == 0 || identity.CharacterUid <= 0)
            return false;

        Action<DetectedPlayerIdentity?>? handlers;
        lock (Gate)
        {
            if (_current is { } current &&
                current.CharacterUid == identity.CharacterUid &&
                string.Equals(current.Name, identity.Name, StringComparison.Ordinal))
            {
                return false;
            }

            _current = identity;
            handlers = IdentityChanged;
        }

        NotifyHandlers(handlers, identity);
        RefreshSpeechEngine();
        return true;
    }

    private static void RefreshSpeechEngine()
    {
        ChatSpeechTranslationSettings? template;
        ConcurrentQueue<ChatTranslationResult>? results;
        DetectedPlayerIdentity? detected;
        lock (Gate)
        {
            template = _speechTemplate is null
                ? null
                : CloneSpeechSettings(_speechTemplate, _speechTemplate.IgnoreOwnUsername);
            results = _translationResults;
            detected = _current;
        }

        if (template is null) return;
        var runtime = CloneSpeechSettings(
            template,
            EffectiveUsername(template.IgnoreOwnUsername, detected));
        ChatSpeechTranslationEngine.Configure(runtime, results);
    }

    private static ChatSpeechTranslationSettings CloneSpeechSettings(
        ChatSpeechTranslationSettings source,
        string ignoreOwnUsername) => new()
    {
        TranslationEnabled = source.TranslationEnabled,
        TranslationWorld = source.TranslationWorld,
        TranslationGuild = source.TranslationGuild,
        TranslationPartyTeam = source.TranslationPartyTeam,
        ShowTranslationInOverlay = source.ShowTranslationInOverlay,
        TtsEnabled = source.TtsEnabled,
        TtsGuild = source.TtsGuild,
        TtsPartyTeam = source.TtsPartyTeam,
        ReadSenderName = source.ReadSenderName,
        IgnoreOwnUsername = ignoreOwnUsername,
        TtsVolume = source.TtsVolume,
        HideEmojiMessages = source.HideEmojiMessages,
        HideLinkedItemMessages = source.HideLinkedItemMessages
    };

    private static string EffectiveUsername(
        string? manualOverride,
        DetectedPlayerIdentity? detected)
    {
        var manual = NormalizeUsername(manualOverride);
        return manual.Length > 0 ? manual : detected?.Name ?? string.Empty;
    }

    private static void NotifyHandlers(
        Action<DetectedPlayerIdentity?>? handlers,
        DetectedPlayerIdentity? identity)
    {
        if (handlers is null) return;
        foreach (var subscriber in handlers.GetInvocationList())
        {
            try { ((Action<DetectedPlayerIdentity?>)subscriber)(identity); }
            catch (Exception ex) { AppLog.Write("identity: UI listener failed " + ex.Message); }
        }
    }

    private static string NormalizeUsername(string? value)
    {
        if (string.IsNullOrWhiteSpace(value)) return string.Empty;
        var normalized = value
            .Replace('\r', ' ')
            .Replace('\n', ' ')
            .Replace('\0', ' ')
            .Trim();
        if (normalized.Length > MaxUsernameLength)
            normalized = normalized[..MaxUsernameLength];
        return normalized;
    }

    private static bool TryParseEnterScene(byte[] data, out DetectedPlayerIdentity identity)
    {
        identity = default;
        if (data.Length == 0) return false;

        // EnterScene.enter_scene_info = field 1.
        if (!TryGetLengthField(data, 0, data.Length, 1, out var infoOffset, out var infoLength))
            return false;
        // EnterSceneInfo.player_ent = field 2.
        if (!TryGetLengthField(data, infoOffset, infoLength, 2, out var playerOffset, out var playerLength))
            return false;
        // Entity.uuid = field 1.
        if (!TryGetVarintField(data, playerOffset, playerLength, 1, out var rawUuid) || rawUuid > long.MaxValue)
            return false;
        // Entity.attrs = field 3.
        if (!TryGetLengthField(data, playerOffset, playerLength, 3, out var attrsOffset, out var attrsLength))
            return false;
        if (!TryFindAttributeRawData(
                data,
                attrsOffset,
                attrsLength,
                NameAttributeId,
                out var rawNameOffset,
                out var rawNameLength))
            return false;
        if (!TryDecodePrefixedUtf8(data, rawNameOffset, rawNameLength, out var name))
            return false;

        name = NormalizeUsername(name);
        var entityUuid = checked((long)rawUuid);
        var characterUid = entityUuid >> 16;
        if (name.Length == 0 || characterUid <= 0)
            return false;

        identity = new DetectedPlayerIdentity(name, characterUid, DateTime.UtcNow);
        return true;
    }

    private static bool TryFindAttributeRawData(
        byte[] data,
        int offset,
        int length,
        int wantedAttributeId,
        out int rawOffset,
        out int rawLength)
    {
        rawOffset = 0;
        rawLength = 0;
        var p = offset;
        var limit = offset + length;
        if (offset < 0 || length < 0 || limit < offset || limit > data.Length) return false;

        while (p < limit)
        {
            if (!ReadVarint(data, ref p, limit, out var key)) return false;
            var field = (int)(key >> 3);
            var wire = (int)(key & 7);

            if (field == 2 && wire == 2) // AttrCollection.attrs (repeated Attr)
            {
                if (!ReadVarint(data, ref p, limit, out var len) || len > (ulong)(limit - p))
                    return false;
                var attrOffset = p;
                var attrLength = checked((int)len);
                p += attrLength;

                if (!TryGetVarintField(data, attrOffset, attrLength, 1, out var attrId))
                    continue;
                if (attrId != (ulong)wantedAttributeId)
                    continue;
                return TryGetLengthField(data, attrOffset, attrLength, 2, out rawOffset, out rawLength);
            }

            if (!SkipField(data, ref p, limit, wire)) return false;
        }

        return false;
    }

    private static bool TryDecodePrefixedUtf8(byte[] data, int offset, int length, out string value)
    {
        value = string.Empty;
        if (offset < 0 || length < 0 || offset + length > data.Length) return false;

        var p = offset;
        var limit = offset + length;
        if (!ReadVarint(data, ref p, limit, out var byteLength) || byteLength > (ulong)(limit - p))
            return false;
        if (byteLength == 0 || byteLength > 512) return false;

        try
        {
            value = new UTF8Encoding(false, true).GetString(data, p, checked((int)byteLength));
            return true;
        }
        catch (DecoderFallbackException)
        {
            return false;
        }
    }

    private static bool TryGetLengthField(
        byte[] data,
        int offset,
        int length,
        int wantedField,
        out int valueOffset,
        out int valueLength)
    {
        valueOffset = 0;
        valueLength = 0;
        var p = offset;
        var limit = offset + length;
        if (offset < 0 || length < 0 || limit < offset || limit > data.Length) return false;

        while (p < limit)
        {
            if (!ReadVarint(data, ref p, limit, out var key)) return false;
            var field = (int)(key >> 3);
            var wire = (int)(key & 7);

            if (wire == 2)
            {
                if (!ReadVarint(data, ref p, limit, out var len) || len > (ulong)(limit - p)) return false;
                if (field == wantedField)
                {
                    valueOffset = p;
                    valueLength = checked((int)len);
                    return true;
                }
                p += checked((int)len);
            }
            else if (!SkipField(data, ref p, limit, wire))
            {
                return false;
            }
        }

        return false;
    }

    private static bool TryGetVarintField(
        byte[] data,
        int offset,
        int length,
        int wantedField,
        out ulong value)
    {
        value = 0;
        var p = offset;
        var limit = offset + length;
        if (offset < 0 || length < 0 || limit < offset || limit > data.Length) return false;

        while (p < limit)
        {
            if (!ReadVarint(data, ref p, limit, out var key)) return false;
            var field = (int)(key >> 3);
            var wire = (int)(key & 7);

            if (wire == 0)
            {
                if (!ReadVarint(data, ref p, limit, out var fieldValue)) return false;
                if (field == wantedField)
                {
                    value = fieldValue;
                    return true;
                }
            }
            else if (!SkipField(data, ref p, limit, wire))
            {
                return false;
            }
        }

        return false;
    }

    private static bool ReadVarint(byte[] data, ref int p, int limit, out ulong value)
    {
        value = 0;
        var shift = 0;
        while (p < limit && shift < 64)
        {
            var b = data[p++];
            value |= ((ulong)(b & 0x7F)) << shift;
            if ((b & 0x80) == 0) return true;
            shift += 7;
        }
        return false;
    }

    private static bool SkipField(byte[] data, ref int p, int limit, int wire)
    {
        switch (wire)
        {
            case 0:
                return ReadVarint(data, ref p, limit, out _);
            case 1:
                if (p + 8 > limit) return false;
                p += 8;
                return true;
            case 2:
                if (!ReadVarint(data, ref p, limit, out var len) || len > (ulong)(limit - p)) return false;
                p += checked((int)len);
                return true;
            case 5:
                if (p + 4 > limit) return false;
                p += 4;
                return true;
            default:
                return false;
        }
    }

    internal static (bool Success, DetectedPlayerIdentity Identity) ParseForSelfTest(byte[] payload)
    {
        var success = TryParseEnterScene(payload, out var identity);
        return (success, identity);
    }

    internal static void SetForSelfTest(string name, long characterUid) =>
        Publish(new DetectedPlayerIdentity(name, characterUid, DateTime.UtcNow));

    internal static void ClearForSelfTest() => ClearCurrent();
}
