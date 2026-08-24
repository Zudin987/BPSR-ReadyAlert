namespace BPSR.ReadyAlert;

internal static class AppLog
{
    private static readonly object Gate = new();
    private static string? _path;
    private const long MaxBytes = 2 * 1024 * 1024;

    internal static void Initialize(string path)
    {
        lock (Gate)
        {
            _path = path;
            RotateIfNeeded();
        }
    }

    internal static void Write(string message)
    {
        try
        {
            lock (Gate)
            {
                if (string.IsNullOrWhiteSpace(_path)) return;

                // Rotation used to happen only on startup, so a very long-running
                // session could grow indefinitely. Check before each append; ReadyAlert
                // writes infrequently enough that this is negligible overhead.
                RotateIfNeeded();

                File.AppendAllText(
                    _path,
                    $"{DateTime.Now:yyyy-MM-dd HH:mm:ss.fff} {message}{Environment.NewLine}");
            }
        }
        catch { }
    }

    private static void RotateIfNeeded()
    {
        try
        {
            if (string.IsNullOrWhiteSpace(_path) || !File.Exists(_path)) return;
            if (new FileInfo(_path).Length <= MaxBytes) return;

            var old = _path + ".old";
            File.Delete(old);
            File.Move(_path, old);
        }
        catch { }
    }
}
