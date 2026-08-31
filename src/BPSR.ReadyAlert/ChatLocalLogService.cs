using System.Collections.Concurrent;
using System.Diagnostics;
using System.Globalization;
using System.Text;

namespace BPSR.ReadyAlert;

internal readonly record struct ChatLocalLogStatus(
    bool Enabled,
    int QueueCount,
    long Enqueued,
    long Written,
    long Dropped,
    long WriteFailures,
    long CleanupFailures);

/// <summary>
/// Local-only rolling chat history. The capture path only performs a bounded,
/// non-blocking enqueue; all filesystem work and AppLog diagnostics happen on the
/// dedicated background writer thread.
/// </summary>
internal static class ChatLocalLogService
{
    private static readonly object Gate = new();
    private static ChatLocalLogWriter? _writer;
    private static volatile bool _enabled;

    internal static bool Enabled
    {
        get => _enabled;
        set
        {
            _enabled = value;
            Volatile.Read(ref _writer)?.SetEnabled(value);
        }
    }

    internal static string LogDirectory => Volatile.Read(ref _writer)?.DirectoryPath ?? string.Empty;

    internal static void Initialize(string directory)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(directory);
        lock (Gate)
        {
            if (_writer is not null) return;
            var writer = new ChatLocalLogWriter(directory, startWorker: true);
            writer.SetEnabled(_enabled);
            Volatile.Write(ref _writer, writer);
        }
    }

    internal static void TryEnqueue(ChatMessageEvent message)
    {
        if (!_enabled) return;
        Volatile.Read(ref _writer)?.TryEnqueue(message);
    }

    /// <summary>
    /// Queue an infrequent diagnostic originating on a capture/hot path. The actual
    /// AppLog.Write call is performed by the chat-log worker, never by the caller.
    /// </summary>
    internal static void QueueAppDiagnostic(string message) =>
        Volatile.Read(ref _writer)?.TryEnqueueDiagnostic(message);

    internal static ChatLocalLogStatus GetStatus() =>
        Volatile.Read(ref _writer)?.GetStatus() ?? new ChatLocalLogStatus(_enabled, 0, 0, 0, 0, 0, 0);

    internal static void OpenFolder()
    {
        var path = LogDirectory;
        if (string.IsNullOrWhiteSpace(path)) return;

        ThreadPool.UnsafeQueueUserWorkItem(static state =>
        {
            var folder = (string)state!;
            try
            {
                Directory.CreateDirectory(folder);
                Process.Start(new ProcessStartInfo("explorer.exe", $"\"{folder}\"") { UseShellExecute = true });
            }
            catch (Exception ex)
            {
                AppLog.Write("chatlog: open folder failed " + ex.Message);
            }
        }, path, preferLocal: false);
    }

    internal static void Shutdown()
    {
        ChatLocalLogWriter? writer;
        lock (Gate)
        {
            writer = _writer;
            _writer = null;
        }

        writer?.Dispose();
    }
}

internal readonly record struct ChatLocalLogEntry(
    DateTimeOffset LocalTimestamp,
    ChatChannel Channel,
    string SenderName,
    string Text)
{
    internal DateTimeOffset UtcTimestamp => LocalTimestamp.ToUniversalTime();
}

internal sealed class ChatLocalLogWriter : IDisposable
{
    internal const int MaxQueuedMessages = 4_096;
    private const int MaxQueuedDiagnostics = 64;
    private const int MaxBatchMessages = 256;
    private const int MaxLogLineChars = 256 * 1024;
    private static readonly TimeSpan Retention = TimeSpan.FromHours(24);
    private static readonly TimeSpan CleanupInterval = TimeSpan.FromMinutes(5);
    private static readonly UTF8Encoding Utf8NoBom = new(false);
    private static readonly string TimestampFormat = "yyyy-MM-dd HH:mm:ss.fff zzz";

    private readonly ConcurrentQueue<ChatLocalLogEntry> _queue = new();
    private readonly ConcurrentQueue<string> _diagnostics = new();
    private readonly SemaphoreSlim _signal = new(0);
    private readonly Thread? _thread;
    private readonly ManualResetEventSlim _startupCleanupDone = new(false);
    private volatile bool _enabled;
    private volatile bool _stopping;
    private int _queueCount;
    private int _diagnosticCount;
    private long _enqueued;
    private long _written;
    private long _dropped;
    private long _writeFailures;
    private long _cleanupFailures;
    private long _lastFailureLogUtcTicks;
    private DateTimeOffset _nextCleanupUtc;
    private int _disposed;

    internal string DirectoryPath { get; }

    internal ChatLocalLogWriter(string directory, bool startWorker)
    {
        DirectoryPath = directory;
        _nextCleanupUtc = DateTimeOffset.MinValue;
        if (!startWorker) return;

        _thread = new Thread(WorkerMain)
        {
            IsBackground = true,
            Name = "BPSR-ReadyAlert-ChatLogWriter"
        };
        _thread.Start();
    }

    internal void SetEnabled(bool enabled)
    {
        _enabled = enabled;
        if (enabled)
        {
            try { _signal.Release(); }
            catch (ObjectDisposedException) { }
        }
    }

    internal bool TryEnqueue(ChatMessageEvent message)
    {
        if (!_enabled || _stopping) return false;

        var localTimestamp = ToLocalTimestamp(message.Timestamp);
        var entry = new ChatLocalLogEntry(
            localTimestamp,
            message.Channel,
            message.SenderName ?? string.Empty,
            message.Text ?? string.Empty);
        return TryEnqueueEntry(entry);
    }

    private bool TryEnqueueEntry(ChatLocalLogEntry entry)
    {
        var reserved = Interlocked.Increment(ref _queueCount);
        if (reserved > MaxQueuedMessages)
        {
            Interlocked.Decrement(ref _queueCount);
            Interlocked.Increment(ref _dropped);
            return false;
        }

        _queue.Enqueue(entry);
        Interlocked.Increment(ref _enqueued);
        try { _signal.Release(); }
        catch (ObjectDisposedException) { }
        return true;
    }

    internal bool TryEnqueueDiagnostic(string message)
    {
        if (_stopping || string.IsNullOrWhiteSpace(message)) return false;
        var reserved = Interlocked.Increment(ref _diagnosticCount);
        if (reserved > MaxQueuedDiagnostics)
        {
            Interlocked.Decrement(ref _diagnosticCount);
            return false;
        }

        _diagnostics.Enqueue(message);
        try { _signal.Release(); }
        catch (ObjectDisposedException) { }
        return true;
    }

    internal ChatLocalLogStatus GetStatus() => new(
        _enabled,
        Math.Max(0, Volatile.Read(ref _queueCount)),
        Interlocked.Read(ref _enqueued),
        Interlocked.Read(ref _written),
        Interlocked.Read(ref _dropped),
        Interlocked.Read(ref _writeFailures),
        Interlocked.Read(ref _cleanupFailures));

    private void WorkerMain()
    {
        try
        {
            RunCleanup(DateTimeOffset.UtcNow);
        }
        catch (Exception ex)
        {
            Interlocked.Increment(ref _cleanupFailures);
            ReportFailure("startup cleanup", ex);
        }
        finally
        {
            _startupCleanupDone.Set();
        }

        _nextCleanupUtc = DateTimeOffset.UtcNow + CleanupInterval;
        var batch = new List<ChatLocalLogEntry>(MaxBatchMessages);

        while (!_stopping)
        {
            try { _signal.Wait(TimeSpan.FromSeconds(2)); }
            catch (ObjectDisposedException) { break; }

            DrainDiagnostics();
            DrainBatch(batch, DateTimeOffset.UtcNow);

            var now = DateTimeOffset.UtcNow;
            if (now >= _nextCleanupUtc)
            {
                RunCleanupSafe(now);
                _nextCleanupUtc = now + CleanupInterval;
            }
        }

        DrainDiagnostics();
        do
        {
            DrainBatch(batch, DateTimeOffset.UtcNow);
        } while (Volatile.Read(ref _queueCount) > 0);
        RunCleanupSafe(DateTimeOffset.UtcNow);
    }

    private void DrainBatch(List<ChatLocalLogEntry> batch, DateTimeOffset nowUtc)
    {
        batch.Clear();
        while (batch.Count < MaxBatchMessages && _queue.TryDequeue(out var entry))
        {
            Interlocked.Decrement(ref _queueCount);
            if (entry.UtcTimestamp >= nowUtc - Retention)
                batch.Add(entry);
        }

        if (batch.Count == 0) return;
        WriteBatchSafe(batch);
    }

    private void WriteBatchSafe(IReadOnlyList<ChatLocalLogEntry> entries)
    {
        try
        {
            Directory.CreateDirectory(DirectoryPath);
            var groups = new Dictionary<string, StringBuilder>(StringComparer.OrdinalIgnoreCase);
            foreach (var entry in entries)
            {
                var path = GetHourlyPath(entry.UtcTimestamp);
                if (!groups.TryGetValue(path, out var builder))
                {
                    builder = new StringBuilder(Math.Min(64 * 1024, entries.Count * 96));
                    groups.Add(path, builder);
                }
                builder.AppendLine(FormatLine(entry));
            }

            foreach (var pair in groups)
            {
                using var stream = new FileStream(
                    pair.Key,
                    FileMode.Append,
                    FileAccess.Write,
                    FileShare.ReadWrite | FileShare.Delete,
                    bufferSize: 16 * 1024,
                    FileOptions.SequentialScan);
                using var writer = new StreamWriter(stream, Utf8NoBom, bufferSize: 16 * 1024, leaveOpen: false);
                writer.Write(pair.Value.ToString());
            }

            Interlocked.Add(ref _written, entries.Count);
        }
        catch (Exception ex)
        {
            Interlocked.Increment(ref _writeFailures);
            ReportFailure("write", ex);
        }
    }

    private void DrainDiagnostics()
    {
        while (_diagnostics.TryDequeue(out var message))
        {
            Interlocked.Decrement(ref _diagnosticCount);
            AppLog.Write(message);
        }
    }

    private void RunCleanupSafe(DateTimeOffset nowUtc)
    {
        try { RunCleanup(nowUtc); }
        catch (Exception ex)
        {
            Interlocked.Increment(ref _cleanupFailures);
            ReportFailure("cleanup", ex);
        }
    }

    private void RunCleanup(DateTimeOffset nowUtc)
    {
        Directory.CreateDirectory(DirectoryPath);
        var cutoffUtc = nowUtc.ToUniversalTime() - Retention;

        foreach (var staleTemp in Directory.EnumerateFiles(DirectoryPath, "*.cleanup.tmp", SearchOption.TopDirectoryOnly))
        {
            try { File.Delete(staleTemp); }
            catch (Exception ex)
            {
                Interlocked.Increment(ref _cleanupFailures);
                ReportFailure("stale temp cleanup", ex);
            }
        }

        foreach (var path in Directory.EnumerateFiles(DirectoryPath, "*.txt", SearchOption.TopDirectoryOnly))
            CleanupFile(path, cutoffUtc);
    }

    private void CleanupFile(string path, DateTimeOffset cutoffUtc)
    {
        var tempPath = path + ".cleanup.tmp";
        var kept = 0;
        var removed = 0;
        var invalid = 0;
        var changed = false;

        try
        {
            using (var input = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete, 16 * 1024, FileOptions.SequentialScan))
            using (var reader = new StreamReader(input, Encoding.UTF8, detectEncodingFromByteOrderMarks: true, bufferSize: 16 * 1024, leaveOpen: false))
            using (var output = new FileStream(tempPath, FileMode.Create, FileAccess.Write, FileShare.None, 16 * 1024, FileOptions.SequentialScan))
            using (var writer = new StreamWriter(output, Utf8NoBom, bufferSize: 16 * 1024, leaveOpen: false))
            {
                string? line;
                while ((line = reader.ReadLine()) is not null)
                {
                    if (line.Length > MaxLogLineChars || !TryParseTimestamp(line, out var timestamp))
                    {
                        invalid++;
                        changed = true;
                        continue;
                    }

                    if (timestamp.ToUniversalTime() < cutoffUtc)
                    {
                        removed++;
                        changed = true;
                        continue;
                    }

                    writer.WriteLine(line);
                    kept++;
                }
            }

            if (!changed)
            {
                File.Delete(tempPath);
                return;
            }

            if (kept == 0)
            {
                File.Delete(path);
                File.Delete(tempPath);
            }
            else
            {
                File.Move(tempPath, path, overwrite: true);
            }

            if (invalid > 0)
                AppLog.Write($"chatlog: cleanup dropped {invalid} invalid/corrupt line(s) from {Path.GetFileName(path)}");
            if (removed > 0)
                AppLog.Write($"chatlog: cleanup removed {removed} expired line(s) from {Path.GetFileName(path)}");
        }
        catch (Exception ex)
        {
            Interlocked.Increment(ref _cleanupFailures);
            try { if (File.Exists(tempPath)) File.Delete(tempPath); } catch { }
            ReportFailure("cleanup file " + Path.GetFileName(path), ex);
        }
    }

    private void ReportFailure(string operation, Exception ex)
    {
        // A locked/full/broken disk may fail repeatedly. Rate-limit diagnostics so
        // the independent ReadyAlert diagnostic log cannot be flooded by one fault.
        var nowTicks = DateTime.UtcNow.Ticks;
        var previous = Interlocked.Read(ref _lastFailureLogUtcTicks);
        if (previous != 0 && nowTicks - previous < TimeSpan.FromSeconds(30).Ticks) return;
        if (Interlocked.CompareExchange(ref _lastFailureLogUtcTicks, nowTicks, previous) != previous) return;
        AppLog.Write($"chatlog: {operation} failed; chat capture continues. {ex.Message}");
    }

    private string GetHourlyPath(DateTimeOffset utcTimestamp) =>
        Path.Combine(DirectoryPath, $"chat-{utcTimestamp.UtcDateTime:yyyyMMdd-HH}Z.txt");

    private static DateTimeOffset ToLocalTimestamp(DateTime timestamp)
    {
        try
        {
            if (timestamp.Kind == DateTimeKind.Utc)
                return new DateTimeOffset(timestamp).ToLocalTime();
            if (timestamp.Kind == DateTimeKind.Local)
                return new DateTimeOffset(timestamp);

            var offset = TimeZoneInfo.Local.GetUtcOffset(timestamp);
            return new DateTimeOffset(timestamp, offset);
        }
        catch
        {
            return DateTimeOffset.Now;
        }
    }

    private static string FormatLine(ChatLocalLogEntry entry)
    {
        var sender = string.IsNullOrWhiteSpace(entry.SenderName) ? "<unknown>" : Escape(entry.SenderName);
        return $"[{entry.LocalTimestamp.ToString(TimestampFormat, CultureInfo.InvariantCulture)}] [{ChannelName(entry.Channel)}] {sender}: {Escape(entry.Text)}";
    }

    private static string ChannelName(ChatChannel channel) => channel switch
    {
        ChatChannel.Union => "Guild",
        ChatChannel.Team => "Team",
        ChatChannel.Group => "Group",
        ChatChannel.World => "World",
        ChatChannel.Local => "Local",
        ChatChannel.Private => "Private",
        ChatChannel.TopNotice => "TopNotice",
        ChatChannel.Play => "Play",
        ChatChannel.Newbie => "Newbie",
        ChatChannel.System => "System",
        _ => "Unknown"
    };

    private static string Escape(string? value)
    {
        if (string.IsNullOrEmpty(value)) return string.Empty;
        StringBuilder? builder = null;
        for (var i = 0; i < value.Length; i++)
        {
            var c = value[i];
            string? replacement = c switch
            {
                '\\' => "\\\\",
                '\r' => "\\r",
                '\n' => "\\n",
                '\t' => "\\t",
                _ when char.IsControl(c) => $"\\u{(int)c:X4}",
                _ => null
            };

            if (replacement is null)
            {
                builder?.Append(c);
                continue;
            }

            builder ??= new StringBuilder(value.Length + 16).Append(value, 0, i);
            builder.Append(replacement);
        }
        return builder?.ToString() ?? value;
    }

    private static bool TryParseTimestamp(string line, out DateTimeOffset timestamp)
    {
        timestamp = default;
        if (line.Length < 2 || line[0] != '[') return false;
        var close = line.IndexOf(']');
        if (close <= 1) return false;
        var raw = line.AsSpan(1, close - 1);
        return DateTimeOffset.TryParseExact(
            raw,
            TimestampFormat,
            CultureInfo.InvariantCulture,
            DateTimeStyles.None,
            out timestamp);
    }

    internal static string FormatLineForSelfTest(ChatMessageEvent message) =>
        FormatLine(new ChatLocalLogEntry(ToLocalTimestamp(message.Timestamp), message.Channel, message.SenderName, message.Text));

    internal static bool TryParseTimestampForSelfTest(string line, out DateTimeOffset timestamp) =>
        TryParseTimestamp(line, out timestamp);

    internal void WriteMessagesForSelfTest(IReadOnlyList<ChatMessageEvent> messages, DateTimeOffset nowUtc)
    {
        var cutoff = nowUtc.ToUniversalTime() - Retention;
        var entries = messages
            .Select(x => new ChatLocalLogEntry(ToLocalTimestamp(x.Timestamp), x.Channel, x.SenderName, x.Text))
            .Where(x => x.UtcTimestamp >= cutoff)
            .ToArray();
        if (entries.Length > 0) WriteBatchSafe(entries);
    }

    internal void CleanupForSelfTest(DateTimeOffset nowUtc) => RunCleanupSafe(nowUtc);

    internal bool WaitForStartupCleanupForSelfTest(TimeSpan timeout) => _startupCleanupDone.Wait(timeout);

    internal int QueueCountForSelfTest => Math.Max(0, Volatile.Read(ref _queueCount));
    internal long DroppedForSelfTest => Interlocked.Read(ref _dropped);

    public void Dispose()
    {
        if (Interlocked.Exchange(ref _disposed, 1) != 0) return;
        _stopping = true;
        try { _signal.Release(); } catch (ObjectDisposedException) { }

        if (_thread is not null && _thread.IsAlive && !_thread.Join(TimeSpan.FromSeconds(2)))
            AppLog.Write("chatlog: writer did not stop within 2 seconds; shutdown continues");

        _startupCleanupDone.Dispose();
        _signal.Dispose();
    }
}
