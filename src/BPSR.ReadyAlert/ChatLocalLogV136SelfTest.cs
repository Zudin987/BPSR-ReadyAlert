using System.Collections.Concurrent;
using System.Diagnostics;
using System.Reflection;
using System.Text;

namespace BPSR.ReadyAlert;

/// <summary>
/// Published-EXE coverage for v1.3.6 rolling local chat history. All tests use a
/// temporary directory and never require Npcap, BPSR, network access or a real user log.
/// </summary>
internal static class ChatLocalLogV136SelfTest
{
    internal static void Run()
    {
        var root = Path.Combine(Path.GetTempPath(), "BPSR-ReadyAlert-v136-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);
        try
        {
            TestReadableFormattingAndUnicode(root);
            TestRollingRetentionBoundary(root);
            TestStartupAndCorruptCleanup(root);
            TestLockedFilesystemFailureIsolation(root);
            TestBoundedQueue(root);
            TestSettingsCompatibility(root);
            TestSettingsUi();
            TestSharedCaptureRouting();
            TestNoSecondCaptureOwnership();
            TestHighVolumePerformance(root);
        }
        finally
        {
            ChatLocalLogService.Enabled = false;
            ChatCaptureBridge.Enabled = false;
            try { Directory.Delete(root, recursive: true); } catch { }
        }
    }

    private static void TestReadableFormattingAndUnicode(string root)
    {
        var now = DateTimeOffset.UtcNow;
        var messages = new[]
        {
            Message(now, ChatChannel.Union, "Aiman玩家😀", "hello makan 食物 안녕"),
            Message(now.AddSeconds(1), ChatChannel.Team, "AnotherUser", "line one\r\nline two\tend")
        };

        using var writer = new ChatLocalLogWriter(Path.Combine(root, "format"), startWorker: false);
        writer.WriteMessagesForSelfTest(messages, now.AddMinutes(1));
        var files = Directory.GetFiles(writer.DirectoryPath, "*.txt");
        Assert(files.Length == 1, "normal chat writes one hourly TXT file");

        var text = File.ReadAllText(files[0], Encoding.UTF8);
        Assert(text.Contains("[Guild] Aiman玩家😀: hello makan 食物 안녕", StringComparison.Ordinal),
            "UTF-8 username/message and Guild channel are preserved");
        Assert(text.Contains("[Team] AnotherUser: line one\\r\\nline two\\tend", StringComparison.Ordinal),
            "multiline/tab content is escaped onto one physical log line");
        Assert(!text.Contains("line one\r\nline two", StringComparison.Ordinal),
            "message newlines cannot corrupt the apparent TXT record structure");

        var first = File.ReadLines(files[0], Encoding.UTF8).First();
        Assert(ChatLocalLogWriter.TryParseTimestampForSelfTest(first, out var parsed) &&
               Math.Abs((parsed.ToUniversalTime() - now.ToUniversalTime()).TotalSeconds) < 1,
            "log line contains an exact local timestamp with a parseable UTC offset");
    }

    private static void TestRollingRetentionBoundary(string root)
    {
        var directory = Path.Combine(root, "retention");
        Directory.CreateDirectory(directory);
        using var writer = new ChatLocalLogWriter(directory, startWorker: false);
        var now = new DateTimeOffset(2026, 8, 31, 12, 30, 0, TimeSpan.Zero);
        var keep2359 = Message(now.AddHours(-23).AddMinutes(-59), ChatChannel.World, "Keep2359", "keep");
        var keepExact24 = Message(now.AddHours(-24), ChatChannel.Team, "Keep24", "keep exact cutoff");
        var removeOld = Message(now.AddHours(-24).AddMinutes(-1), ChatChannel.Union, "Remove2401", "remove");

        // Deliberately mix old/new records in one boundary file so cleanup cannot
        // simply trust an hourly filename.
        var boundary = Path.Combine(directory, "boundary.txt");
        File.WriteAllLines(boundary,
        [
            ChatLocalLogWriter.FormatLineForSelfTest(removeOld),
            ChatLocalLogWriter.FormatLineForSelfTest(keepExact24),
            ChatLocalLogWriter.FormatLineForSelfTest(keep2359)
        ], Encoding.UTF8);

        writer.CleanupForSelfTest(now);
        var result = File.ReadAllText(boundary, Encoding.UTF8);
        Assert(!result.Contains("Remove2401", StringComparison.Ordinal), "record older than 24 hours is removed");
        Assert(result.Contains("Keep24", StringComparison.Ordinal), "record exactly 24 hours old remains");
        Assert(result.Contains("Keep2359", StringComparison.Ordinal), "record 23h59m old remains");

        foreach (var name in new[] { "old-a.txt", "old-b.txt", "old-c.txt" })
            File.WriteAllText(Path.Combine(directory, name), ChatLocalLogWriter.FormatLineForSelfTest(removeOld) + Environment.NewLine, Encoding.UTF8);
        writer.CleanupForSelfTest(now);
        Assert(new[] { "old-a.txt", "old-b.txt", "old-c.txt" }.All(x => !File.Exists(Path.Combine(directory, x))),
            "multiple fully expired log files are removed");
    }

    private static void TestStartupAndCorruptCleanup(string root)
    {
        var startupDir = Path.Combine(root, "startup");
        Directory.CreateDirectory(startupDir);
        var old = Message(DateTimeOffset.UtcNow.AddHours(-30), ChatChannel.World, "Old", "expired");
        var oldPath = Path.Combine(startupDir, "old-on-startup.txt");
        File.WriteAllText(oldPath, ChatLocalLogWriter.FormatLineForSelfTest(old) + Environment.NewLine, Encoding.UTF8);

        using (var startupWriter = new ChatLocalLogWriter(startupDir, startWorker: true))
        {
            Assert(startupWriter.WaitForStartupCleanupForSelfTest(TimeSpan.FromSeconds(5)),
                "startup cleanup completes on the background writer");
            Assert(!File.Exists(oldPath), "startup cleanup removes chat older than 24 hours");
        }

        var corruptDir = Path.Combine(root, "corrupt");
        Directory.CreateDirectory(corruptDir);
        using var writer = new ChatLocalLogWriter(corruptDir, startWorker: false);
        var fresh = Message(DateTimeOffset.UtcNow.AddMinutes(-1), ChatChannel.Team, "Fresh", "valid");
        var corruptPath = Path.Combine(corruptDir, "mixed-corrupt.txt");
        File.WriteAllLines(corruptPath,
        [
            "not a valid ReadyAlert chat record",
            "[broken timestamp] [Guild] Corrupt: stale",
            ChatLocalLogWriter.FormatLineForSelfTest(fresh)
        ], Encoding.UTF8);

        writer.CleanupForSelfTest(DateTimeOffset.UtcNow);
        var cleaned = File.ReadAllText(corruptPath, Encoding.UTF8);
        Assert(cleaned.Contains("Fresh", StringComparison.Ordinal) &&
               !cleaned.Contains("not a valid", StringComparison.Ordinal) &&
               !cleaned.Contains("broken timestamp", StringComparison.Ordinal),
            "corrupt/invalid records are privacy-safely dropped without crashing cleanup");
    }

    private static void TestLockedFilesystemFailureIsolation(string root)
    {
        var directory = Path.Combine(root, "locked");
        Directory.CreateDirectory(directory);
        using var writer = new ChatLocalLogWriter(directory, startWorker: false);
        var now = DateTimeOffset.UtcNow;
        var expired = Message(now.AddHours(-30), ChatChannel.World, "Expired", "locked old line");
        var lockedCleanupPath = Path.Combine(directory, "locked-cleanup.txt");
        File.WriteAllText(lockedCleanupPath, ChatLocalLogWriter.FormatLineForSelfTest(expired) + Environment.NewLine, Encoding.UTF8);

        using (var lockStream = new FileStream(lockedCleanupPath, FileMode.Open, FileAccess.ReadWrite, FileShare.None))
        {
            writer.CleanupForSelfTest(now);
            Assert(File.Exists(lockedCleanupPath), "externally locked cleanup file is deferred instead of crashing");
        }
        writer.CleanupForSelfTest(now);
        Assert(!File.Exists(lockedCleanupPath), "previously locked expired file is removed on the next cleanup attempt");

        var currentPath = Path.Combine(directory, $"chat-{now.UtcDateTime:yyyyMMdd-HH}Z.txt");
        File.WriteAllText(currentPath, string.Empty, Encoding.UTF8);
        var fresh = Message(now, ChatChannel.Team, "WriterLock", "capture must survive");
        var beforeFailures = writer.GetStatus().WriteFailures;
        using (var lockStream = new FileStream(currentPath, FileMode.Open, FileAccess.ReadWrite, FileShare.None))
            writer.WriteMessagesForSelfTest([fresh], now);
        Assert(writer.GetStatus().WriteFailures > beforeFailures,
            "unwritable/locked destination is recorded as a log failure without throwing");

        writer.WriteMessagesForSelfTest([fresh], now);
        Assert(File.ReadAllText(currentPath, Encoding.UTF8).Contains("capture must survive", StringComparison.Ordinal),
            "logging resumes after a temporary filesystem failure");
    }

    private static void TestBoundedQueue(string root)
    {
        using var writer = new ChatLocalLogWriter(Path.Combine(root, "bounded"), startWorker: false);
        writer.SetEnabled(true);
        var now = DateTimeOffset.UtcNow;
        var message = Message(now, ChatChannel.World, "Flood", "synthetic");
        for (var i = 0; i < ChatLocalLogWriter.MaxQueuedMessages + 2_000; i++)
            writer.TryEnqueue(message);

        Assert(writer.QueueCountForSelfTest == ChatLocalLogWriter.MaxQueuedMessages,
            "logging queue is strictly bounded");
        Assert(writer.DroppedForSelfTest == 2_000,
            "overflow is dropped instead of causing unlimited RAM growth");
    }

    private static void TestSettingsCompatibility(string root)
    {
        var directory = Path.Combine(root, "settings");
        Directory.CreateDirectory(directory);
        var path = Path.Combine(directory, "settings.json");
        File.WriteAllText(path, "{\"chatOverlayEnabled\":false,\"chat\":{\"topMost\":false,\"maxHistory\":123}}", Encoding.UTF8);

        var store = new SettingsStore(path);
        var loaded = store.Load();
        Assert(!loaded.ChatOverlayEnabled && !loaded.Chat.TopMost && loaded.Chat.MaxHistory == 123,
            "legacy settings keep unrelated saved values during v1.3.6 load");
        Assert(loaded.Chat.KeepLocalChatLogs24Hours,
            "old settings.json missing the new field safely normalizes to 24-hour local history enabled");

        loaded.Chat.KeepLocalChatLogs24Hours = false;
        Assert(store.Save(loaded), "new chat-log preference saves through existing atomic settings store");
        var reloaded = new SettingsStore(path).Load();
        Assert(!reloaded.Chat.KeepLocalChatLogs24Hours,
            "explicit local chat-log preference survives settings round-trip");
    }

    private static void TestSettingsUi()
    {
        var settings = DefaultSettingsProfile.CreateChatOverlay();
        settings.KeepLocalChatLogs24Hours = true;
        using var form = new ChatGeneralSettingsForm(settings);
        var state = form.GetV136ChatLogUiForSelfTest();
        Assert(state.Checked && state.Text.Contains("24 hours", StringComparison.OrdinalIgnoreCase),
            "Settings exposes the enabled 24-hour local chat history preference");
    }

    private static void TestSharedCaptureRouting()
    {
        var queue = new ConcurrentQueue<ChatMessageEvent>();
        ChatCaptureBridge.Configure(queue);
        var notify = BuildTextNotify(ChatChannel.World, 1234, "OriginalUser", "ORIGINAL only");

        ChatCaptureBridge.Enabled = false;
        ChatLocalLogService.Enabled = true;
        var before = ChatCaptureBridge.GetStatus().ParsedMessages;
        Assert(ChatCaptureBridge.TryHandle(ChatProtocol.ServiceId, ChatProtocol.NotifyNewestChitChatMsgs, notify),
            "shared bridge owns ChitChat Notify while only local logging is enabled");
        Assert(ChatCaptureBridge.GetStatus().ParsedMessages == before + 1,
            "logging-only mode reuses the existing single protobuf parse");
        Assert(queue.IsEmpty, "logging-only mode does not wake/show the overlay UI queue");
        Assert(!ChatNotificationEngine.Enabled && !ChatSpeechTranslationEngine.Enabled,
            "logging-only mode does not enable notification, translation or TTS routing");

        // Verify the pre-existing overlay route still receives the same parsed event.
        ChatLocalLogService.Enabled = false;
        ChatCaptureBridge.Enabled = true;
        ChatNotificationEngine.Enabled = false;
        ChatSpeechTranslationEngine.Enabled = false;
        Assert(ChatCaptureBridge.TryHandle(ChatProtocol.ServiceId, ChatProtocol.NotifyNewestChitChatMsgs, notify),
            "shared bridge still owns normal overlay chat");
        Assert(queue.TryDequeue(out var overlayMessage) && overlayMessage.Text == "ORIGINAL only",
            "existing overlay queue still receives original chat after logger integration");

        ChatCaptureBridge.Enabled = false;
    }

    private static void TestNoSecondCaptureOwnership()
    {
        var fieldTypes = typeof(ChatLocalLogWriter)
            .GetFields(BindingFlags.Instance | BindingFlags.NonPublic | BindingFlags.Public)
            .Select(x => x.FieldType)
            .ToArray();
        Assert(!fieldTypes.Contains(typeof(NpcapCapture)) && !fieldTypes.Contains(typeof(CaptureEngine)),
            "local logger owns no NpcapCapture or CaptureEngine instance");
    }

    private static void TestHighVolumePerformance(string root)
    {
        const int count = 5_000;
        var directory = Path.Combine(root, "performance");
        using var writer = new ChatLocalLogWriter(directory, startWorker: false);
        var now = DateTimeOffset.UtcNow;
        var messages = Enumerable.Range(0, count)
            .Select(i => Message(now, i % 2 == 0 ? ChatChannel.World : ChatChannel.Team, "LoadUser" + i, "synthetic chat payload " + i))
            .ToArray();

        var allocatedBefore = GC.GetAllocatedBytesForCurrentThread();
        var writeWatch = Stopwatch.StartNew();
        writer.WriteMessagesForSelfTest(messages, now.AddMinutes(1));
        writeWatch.Stop();
        var allocated = GC.GetAllocatedBytesForCurrentThread() - allocatedBefore;

        var cleanupWatch = Stopwatch.StartNew();
        writer.CleanupForSelfTest(now.AddMinutes(1));
        cleanupWatch.Stop();

        Assert(writer.GetStatus().Written == count, "high-volume synthetic batch writes every accepted record");
        Assert(writeWatch.Elapsed < TimeSpan.FromSeconds(5), "5,000-message batch write stays within a broad CI CPU/I/O guard");
        Assert(cleanupWatch.Elapsed < TimeSpan.FromSeconds(5), "5,000-message retention scan stays within a broad CI CPU/I/O guard");
        Assert(allocated < 96L * 1024 * 1024, "5,000-message batch avoids unreasonable managed allocation growth");

        try
        {
            File.AppendAllLines(
                Path.Combine(AppContext.BaseDirectory, "ui-performance-v132.txt"),
                [
                    $"chatlog_v136_messages={count}",
                    $"chatlog_v136_write_ms={writeWatch.Elapsed.TotalMilliseconds:F2}",
                    $"chatlog_v136_cleanup_ms={cleanupWatch.Elapsed.TotalMilliseconds:F2}",
                    $"chatlog_v136_alloc_mib={allocated / 1024d / 1024d:F2}",
                    $"chatlog_v136_queue_capacity={ChatLocalLogWriter.MaxQueuedMessages}"
                ]);
        }
        catch { }
    }

    private static ChatMessageEvent Message(DateTimeOffset utc, ChatChannel channel, string sender, string text) =>
        new(1, sender, 80, channel, utc.UtcDateTime.ToLocalTime(), ChatMessageKind.Text, text);

    private static byte[] BuildTextNotify(ChatChannel channel, long senderId, string senderName, string text)
    {
        var sender = ProtoMessage(
            VarintField(1, unchecked((ulong)senderId)),
            StringField(2, senderName),
            VarintField(5, 80));
        var info = ProtoMessage(
            VarintField(1, (ulong)ChatMessageKind.Text),
            StringField(3, text));
        var chatMsg = ProtoMessage(
            BytesField(2, sender),
            VarintField(3, (ulong)DateTimeOffset.UtcNow.ToUnixTimeSeconds()),
            BytesField(4, info));
        var request = ProtoMessage(
            VarintField(1, (ulong)channel),
            BytesField(2, chatMsg));
        return ProtoMessage(BytesField(1, request));
    }

    private static byte[] ProtoMessage(params byte[][] fields)
    {
        var length = fields.Sum(x => x.Length);
        var output = new byte[length];
        var offset = 0;
        foreach (var field in fields)
        {
            Buffer.BlockCopy(field, 0, output, offset, field.Length);
            offset += field.Length;
        }
        return output;
    }

    private static byte[] StringField(int field, string value) => BytesField(field, Encoding.UTF8.GetBytes(value));

    private static byte[] BytesField(int field, byte[] value) =>
        ProtoMessage(Varint(((ulong)field << 3) | 2UL), Varint((ulong)value.Length), value);

    private static byte[] VarintField(int field, ulong value) =>
        ProtoMessage(Varint((ulong)field << 3), Varint(value));

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

    private static void Assert(bool condition, string message)
    {
        if (condition) return;
        throw new InvalidOperationException("v1.3.6 local chat-log self-test failed: " + message);
    }
}
