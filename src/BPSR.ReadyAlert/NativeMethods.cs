using System.Reflection;
using System.Runtime.InteropServices;

namespace BPSR.ReadyAlert;

internal static class NativeMethods
{
    internal const ulong WinDivertFlagSniff = 0x0001UL;
    internal const ulong WinDivertFlagRecvOnly = 0x0004UL;
    internal static readonly IntPtr InvalidHandleValue = new(-1);

    private static string? _winDivertDllPath;
    private static bool _resolverConfigured;

    internal static void ConfigureWinDivert(string dllPath)
    {
        if (_resolverConfigured) return;
        _winDivertDllPath = dllPath;
        NativeLibrary.SetDllImportResolver(typeof(NativeMethods).Assembly, ResolveNativeLibrary);
        _resolverConfigured = true;
    }

    private static IntPtr ResolveNativeLibrary(
        string libraryName,
        Assembly assembly,
        DllImportSearchPath? searchPath)
    {
        if (!string.Equals(libraryName, "WinDivert.dll", StringComparison.OrdinalIgnoreCase))
            return IntPtr.Zero;

        if (string.IsNullOrWhiteSpace(_winDivertDllPath))
            return IntPtr.Zero;

        return NativeLibrary.Load(_winDivertDllPath);
    }

    [DllImport("WinDivert.dll", CharSet = CharSet.Ansi, SetLastError = true)]
    internal static extern IntPtr WinDivertOpen(
        [MarshalAs(UnmanagedType.LPStr)] string filter,
        int layer,
        short priority,
        ulong flags);

    [DllImport("WinDivert.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool WinDivertRecv(
        IntPtr handle,
        [Out] byte[] packet,
        uint packetLen,
        out uint recvLen,
        IntPtr address);

    [DllImport("WinDivert.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool WinDivertClose(IntPtr handle);
}
