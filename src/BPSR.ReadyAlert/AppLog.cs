using System.Collections.Concurrent;
using System.Text;

namespace BPSR.ReadyAlert;

/// <summary>
/// Small bounded diagnostic logger. Callers only enqueue in memory; all filesystem
/// access is isolated to one background thread so capture/UI/TTS paths never wait on
/// a slow, locked or failing diagnostic log.
/// </summary>
internal static class AppLog
{
    private const long MaxBytes = 2 * 1024 * 1024;
    private const int MaxQueuedEntries = 2_048;
    private const int MaxBatchEntries = 256;

    private static readonly object LifecycleGate = new();
    private static readonly ConcurrentQueue<LogEntry> Queue = new();
    private static readonly AutoResetEvent Signal = new(false);
    private static Thread? _writerThread;
    private static string? _path;
    private static volatile bool _stopping;
    private static int _queueCount;
    private static long _dropped;
    private static long _reportedDropped;

    internal static void Initialize(string path)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(path);

        lock (LifecycleGate)
        {
            if (_writerThread is not null) return;
            _path = path;
            _stopping = false;
            _writerThread = new Thread(WriterMain)
            {
                IsBackground = true,
                Name = "BPSR-ReadyAlert-AppLog"
            };
            _writerThread.Start();
        }
    }

    internal static void Write(string message)
    {
        if (_stopping || string.IsNullOrWhiteSpace(message) || string.IsNullOrWhiteSpace(Volatile.Read(ref _path)))
            return;

        var reserved = Interlocked.Increment(ref _queueCount);
        if (reserved > MaxQueuedEntries)
        {
            Interlocked.Decrement(ref _queueCount);
            Interlocked.Increment(ref _dropped);
            return;
        }

        Queue.Enqueue(new LogEntry(DateTime.Now, message));
        try { Signal.Set(); }
        catch (ObjectDisposedException) { }
    }

    internal static void Shutdown()
    {
        Thread? thread;
        lock (LifecycleGate)
        {
            thread = _writerThread;
            if (thread is null) return;
            _stopping = true;
        }

        try { Signal.Set(); }
        catch (ObjectDisposedException) { }

        // A broken/blocked disk must not make ReadyAlert hang on exit. The writer is
        // a background thread, so if Windows/filesystem I/O does not return promptly,
        // process shutdown continues safely rather than blocking capture teardown.
        _ = thread.Join(TimeSpan.FromSeconds(2));
    }

    private static void WriterMain()
    {
        RotateIfNeeded();
        var batch = new StringBuilder(16 * 1024);

        while (!_stopping)
        {
            try { Signal.WaitOne(TimeSpan.FromSeconds(2)); }
            catch (ObjectDisposedException) { break; }
            DrainAllAvailable(batch);
        }

        DrainAllAvailable(batch);
        batch.Clear();
        WriteDropNoticeIfNeeded(batch, force: true);
        FlushBatch(batch);
    }

    private static void DrainAllAvailable(StringBuilder batch)
    {
        while (Volatile.Read(ref _queueCount) > 0)
        {
            if (!DrainBatch(batch)) break;
        }
    }

    private static bool DrainBatch(StringBuilder batch)
    {
        batch.Clear();
        var count = 0;
        while (count < MaxBatchEntries && Queue.TryDequeue(out var entry))
        {
            Interlocked.Decrement(ref _queueCount);
            batch.Append(entry.Timestamp.ToString("yyyy-MM-dd HH:mm:ss.fff"));
            batch.Append(' ');
            batch.AppendLine(entry.Message);
            count++;
        }

        WriteDropNoticeIfNeeded(batch, force: false);
        if (batch.Length > 0) FlushBatch(batch);
        return count > 0;
    }

    private static void WriteDropNoticeIfNeeded(StringBuilder batch, bool force)
    {
        var dropped = Interlocked.Read(ref _dropped);
        var reported = Interlocked.Read(ref _reportedDropped);
        if (dropped <= reported) return;
        if (!force && Volatile.Read(ref _queueCount) > MaxBatchEntries) return;

        if (Interlocked.CompareExchange(ref _reportedDropped, dropped, reported) != reported)
            return;

        batch.Append(DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss.fff"));
        batch.Append(" applog: bounded queue dropped ");
        batch.Append(dropped - reported);
        batch.AppendLine(" diagnostic line(s); application continued normally");
    }

    private static void FlushBatch(StringBuilder batch)
    {
        try
        {
            var path = Volatile.Read(ref _path);
            if (string.IsNullOrWhiteSpace(path) || batch.Length == 0) return;

            var directory = Path.GetDirectoryName(path);
            if (!string.IsNullOrWhiteSpace(directory)) Directory.CreateDirectory(directory);
            RotateIfNeeded();
            File.AppendAllText(path, batch.ToString(), Encoding.UTF8);
        }
        catch
        {
            // Diagnostics are best-effort only. Never feed a logging failure back into
            // capture, UI, translation/TTS, or the chat-log writer.
        }
    }

    private static void RotateIfNeeded()
    {
        try
        {
            var path = Volatile.Read(ref _path);
            if (string.IsNullOrWhiteSpace(path) || !File.Exists(path)) return;
            if (new FileInfo(path).Length <= MaxBytes) return;

            var old = path + ".old";
            File.Delete(old);
            File.Move(path, old);
        }
        catch { }
    }

    private readonly record struct LogEntry(DateTime Timestamp, string Message);
}
