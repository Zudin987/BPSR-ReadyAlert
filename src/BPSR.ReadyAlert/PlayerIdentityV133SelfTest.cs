using System.Text;

namespace BPSR.ReadyAlert;

internal static class PlayerIdentityV133SelfTest
{
    internal static void Run()
    {
        try
        {
            TestEnterSceneParser();
            TestMalformedPayloads();
            TestDetectedAndManualUsernamePrecedence();
            TestSettingsUiCopy();
        }
        finally
        {
            PlayerIdentityCaptureBridge.ClearForSelfTest();
        }
    }

    private static void TestEnterSceneParser()
    {
        const long uid = 12_345_678;
        const string name = "Mréz玩家";
        var payload = BuildEnterScene(uid, name, nameAttributeId: 1);
        var parsed = PlayerIdentityCaptureBridge.ParseForSelfTest(payload);

        Assert(parsed.Success, "synthetic EnterScene local-player payload parses");
        Assert(parsed.Identity.CharacterUid == uid, "entity UUID decodes to the original character UID");
        Assert(parsed.Identity.Name == name, "UTF-8 BPSR character name is preserved exactly");
    }

    private static void TestMalformedPayloads()
    {
        var valid = BuildEnterScene(42, "Tester", nameAttributeId: 1);
        var truncated = valid[..Math.Max(1, valid.Length - 3)];
        Assert(!PlayerIdentityCaptureBridge.ParseForSelfTest(truncated).Success,
            "truncated EnterScene is rejected without guessing an identity");

        var wrongAttribute = BuildEnterScene(42, "Tester", nameAttributeId: 2);
        Assert(!PlayerIdentityCaptureBridge.ParseForSelfTest(wrongAttribute).Success,
            "a non-name attribute is never treated as the local username");

        Assert(!PlayerIdentityCaptureBridge.ParseForSelfTest(Array.Empty<byte>()).Success,
            "empty EnterScene payload is rejected");
    }

    private static void TestDetectedAndManualUsernamePrecedence()
    {
        PlayerIdentityCaptureBridge.ClearForSelfTest();
        var settings = new ChatSpeechTranslationSettings();
        settings.Normalize();
        Assert(!settings.IsOwnUsername("DetectedHero"),
            "without detection or manual override no sender is treated as the local player");

        PlayerIdentityCaptureBridge.SetForSelfTest("DetectedHero", 42);
        Assert(PlayerIdentityCaptureBridge.EffectiveUsername(string.Empty) == "DetectedHero",
            "blank manual override uses the detected BPSR username");
        Assert(settings.IsOwnUsername("detectedhero"),
            "detected username filters own messages case-insensitively");
        Assert(!settings.IsOwnUsername("OtherHero"),
            "detected username does not suppress another player");

        settings.IgnoreOwnUsername = " ManualHero ";
        settings.Normalize();
        Assert(PlayerIdentityCaptureBridge.EffectiveUsername(settings.IgnoreOwnUsername) == "ManualHero",
            "manual username override takes precedence over auto-detection");
        Assert(settings.IsOwnUsername("manualhero"),
            "manual override filters the configured player");
        Assert(!settings.IsOwnUsername("DetectedHero"),
            "manual override prevents the detected name from also being treated as self");
    }

    private static void TestSettingsUiCopy()
    {
        PlayerIdentityCaptureBridge.ClearForSelfTest();
        var overlay = new ChatOverlaySettings();
        overlay.Normalize();
        var speech = new ChatSpeechTranslationSettings { IgnoreOwnUsername = "ManualHero" };
        speech.Normalize();

        using (var waiting = new ChatGeneralSettingsForm(overlay, speech))
        {
            var state = waiting.GetV133IdentityUiForSelfTest();
            Assert(state.Status.Contains("Waiting for BPSR", StringComparison.Ordinal),
                "settings clearly shows that automatic identity detection is pending");
            Assert(state.ManualOverride == "ManualHero",
                "existing manual override remains visible while detection is pending");
            Assert(state.Placeholder.Contains("detected username", StringComparison.OrdinalIgnoreCase),
                "manual field explains its detected-name fallback");
        }

        PlayerIdentityCaptureBridge.SetForSelfTest("DetectedHero", 98_765_432);
        using var detected = new ChatGeneralSettingsForm(overlay, speech);
        var detectedState = detected.GetV133IdentityUiForSelfTest();
        Assert(detectedState.Status.Contains("DetectedHero", StringComparison.Ordinal) &&
               detectedState.Status.Contains("98765432", StringComparison.Ordinal),
            "settings displays the detected BPSR username and character UID");
        Assert(detectedState.ManualOverride == "ManualHero",
            "displaying auto-detection never overwrites the saved manual override");
    }

    private static byte[] BuildEnterScene(long uid, string name, int nameAttributeId)
    {
        var nameBytes = Encoding.UTF8.GetBytes(name);
        var rawName = Concat(Varint((ulong)nameBytes.Length), nameBytes);
        var attr = Concat(
            VarintField(1, (ulong)nameAttributeId),
            BytesField(2, rawName));
        var attrs = BytesField(2, attr);

        // Same entity encoding used by Resonance Logs CN: UID << 16, character
        // entity type 10 stored at bits 6..13.
        var entityUuid = checked((uid << 16) | (10L << 6));
        var entity = Concat(
            VarintField(1, checked((ulong)entityUuid)),
            BytesField(3, attrs));
        var enterSceneInfo = BytesField(2, entity);
        return BytesField(1, enterSceneInfo);
    }

    private static byte[] VarintField(int field, ulong value) =>
        Concat(Varint((ulong)(field << 3)), Varint(value));

    private static byte[] BytesField(int field, byte[] value) =>
        Concat(Varint((ulong)((field << 3) | 2)), Varint((ulong)value.Length), value);

    private static byte[] Varint(ulong value)
    {
        var bytes = new List<byte>(10);
        do
        {
            var next = (byte)(value & 0x7F);
            value >>= 7;
            if (value != 0) next |= 0x80;
            bytes.Add(next);
        } while (value != 0);
        return bytes.ToArray();
    }

    private static byte[] Concat(params byte[][] parts)
    {
        var length = parts.Sum(x => x.Length);
        var output = new byte[length];
        var offset = 0;
        foreach (var part in parts)
        {
            Buffer.BlockCopy(part, 0, output, offset, part.Length);
            offset += part.Length;
        }
        return output;
    }

    private static void Assert(bool condition, string message)
    {
        if (condition) return;
        throw new InvalidOperationException("v1.3.3 identity self-test failed: " + message);
    }
}
