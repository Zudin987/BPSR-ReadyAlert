using System.Diagnostics;
using System.Text;

namespace BPSR.ReadyAlert;

internal static class AppLogV136SelfTest
{
    internal static void Run()
    {
        var root = Path.Combine(Path.GetTempPath(), "BPSR-ReadyAlert-applog-v136-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);
        var path = Path.Combine(root, "readyalert.log");

        try
        {
            AppLog.Initialize(path);

            const int count = 1_000;
            var watch = Stopwatch.StartNew();
            for (var i = 0; i < count; i++)
                AppLog.Write("selftest async diagnostic " + i);
            watch.Stop();

            // Callers must only enqueue; a broad ceiling catches accidental reversion
            // to one synchronous filesystem append per diagnostic.
            Assert(watch.Elapsed < TimeSpan.FromSeconds(1),
                "1,000 diagnostic calls remain nonblocking/batched");

            AppLog.Shutdown();
            Assert(File.Exists(path), "diagnostic background writer creates readyalert.log");

            var text = File.ReadAllText(path, Encoding.UTF8);
            Assert(text.Contains("selftest async diagnostic 0", StringComparison.Ordinal),
                "first queued diagnostic is flushed");
            Assert(text.Contains("selftest async diagnostic 999", StringComparison.Ordinal),
                "final queued diagnostic is flushed on shutdown");

            // Regression guard for the final-batch shutdown path: no accidental replay
            // of the last batch after drain completion.
            Assert(CountOccurrences(text, "selftest async diagnostic 999") == 1,
                "shutdown does not duplicate the final diagnostic batch");
        }
        finally
        {
            AppLog.Shutdown();
            try { Directory.Delete(root, recursive: true); } catch { }
        }
    }

    private static int CountOccurrences(string text, string value)
    {
        var count = 0;
        var offset = 0;
        while ((offset = text.IndexOf(value, offset, StringComparison.Ordinal)) >= 0)
        {
            count++;
            offset += value.Length;
        }
        return count;
    }

    private static void Assert(bool condition, string message)
    {
        if (condition) return;
        throw new InvalidOperationException("v1.3.6 AppLog self-test failed: " + message);
    }
}
