using System.Runtime.InteropServices;

namespace BPSR.ReadyAlert;

internal sealed record NpcapDevice(string Name, string Description);

internal sealed class NpcapCapture : IDisposable
{
    internal const int DltNull = 0;
    internal const int DltEthernet = 1;
    internal const int DltRaw = 12;
    internal const int DltLoop = 108;
    internal const int DltIpv4 = 228;
    internal const int DltIpv6 = 229;

    private const int ErrorBufferSize = 256;
    private IntPtr _handle;

    internal int DataLink { get; }
    internal string DeviceName { get; }

    internal NpcapCapture(string deviceName)
    {
        EnsureNpcapDllSearchPath();
        DeviceName = deviceName;

        var errbuf = Marshal.AllocHGlobal(ErrorBufferSize);
        try
        {
            Zero(errbuf, ErrorBufferSize);
            _handle = Native.pcap_create(deviceName, errbuf);
            if (_handle == IntPtr.Zero)
                throw new InvalidOperationException("pcap_create failed: " + ReadErrorBuffer(errbuf));

            ConfigureHandle(_handle);
            DataLink = Native.pcap_datalink(_handle);
        }
        catch
        {
            Dispose();
            throw;
        }
        finally
        {
            Marshal.FreeHGlobal(errbuf);
        }
    }

    internal static string GetVersion()
    {
        EnsureNpcapDllSearchPath();
        var ptr = Native.pcap_lib_version();
        return ptr == IntPtr.Zero ? "Npcap/libpcap" : Marshal.PtrToStringAnsi(ptr) ?? "Npcap/libpcap";
    }

    internal static IReadOnlyList<NpcapDevice> ListDevices()
    {
        EnsureNpcapDllSearchPath();
        var errbuf = Marshal.AllocHGlobal(ErrorBufferSize);
        IntPtr all = IntPtr.Zero;
        try
        {
            Zero(errbuf, ErrorBufferSize);
            if (Native.pcap_findalldevs(out all, errbuf) == -1)
                throw new InvalidOperationException("pcap_findalldevs failed: " + ReadErrorBuffer(errbuf));

            var result = new List<NpcapDevice>();
            var current = all;
            while (current != IntPtr.Zero)
            {
                var item = Marshal.PtrToStructure<PcapIf>(current);
                var name = Marshal.PtrToStringAnsi(item.Name) ?? string.Empty;
                var description = item.Description == IntPtr.Zero
                    ? name
                    : Marshal.PtrToStringAnsi(item.Description) ?? name;
                if (!string.IsNullOrWhiteSpace(name))
                    result.Add(new NpcapDevice(name, description));
                current = item.Next;
            }
            return result;
        }
        finally
        {
            if (all != IntPtr.Zero) Native.pcap_freealldevs(all);
            Marshal.FreeHGlobal(errbuf);
        }
    }

    internal bool TryRead(out byte[]? packet)
    {
        packet = null;
        if (_handle == IntPtr.Zero) return false;

        var result = Native.pcap_next_ex(_handle, out var headerPtr, out var dataPtr);
        switch (result)
        {
            case 1:
            {
                var header = Marshal.PtrToStructure<PcapPkthdr>(headerPtr);
                if (header.CapLen == 0 || dataPtr == IntPtr.Zero) return false;
                packet = new byte[header.CapLen];
                Marshal.Copy(dataPtr, packet, 0, checked((int)header.CapLen));
                return true;
            }
            case 0:
            case -2:
                return false;
            case -1:
                throw new InvalidOperationException("pcap_next_ex failed: " + GetHandleError(_handle));
            default:
                throw new InvalidOperationException("Unexpected pcap_next_ex result: " + result);
        }
    }

    private static void ConfigureHandle(IntPtr handle)
    {
        CheckPreActivate(Native.pcap_set_snaplen(handle, 65_536), "pcap_set_snaplen", handle);
        CheckPreActivate(Native.pcap_set_promisc(handle, 1), "pcap_set_promisc", handle);
        // Keep a short fallback timeout because v0.4.1 may scan several adapters.
        // On normal Npcap builds immediate mode makes reads return promptly anyway.
        CheckPreActivate(Native.pcap_set_timeout(handle, 50), "pcap_set_timeout", handle);
        CheckPreActivate(Native.pcap_set_buffer_size(handle, 16 * 1024 * 1024), "pcap_set_buffer_size", handle);

        try
        {
            var immediate = Native.pcap_set_immediate_mode(handle, 1);
            if (immediate != 0)
                AppLog.Write("npcap: pcap_set_immediate_mode returned " + immediate + "; continuing");
        }
        catch (EntryPointNotFoundException)
        {
            AppLog.Write("npcap: immediate mode unavailable; continuing with timeout mode");
        }

        var activate = Native.pcap_activate(handle);
        if (activate < 0)
            throw new InvalidOperationException($"pcap_activate failed ({activate}): {GetHandleError(handle)}");
        if (activate > 0)
            AppLog.Write($"npcap: pcap_activate warning={activate}: {GetHandleError(handle)}");

        var filter = new BpfProgram();
        const string expression = "tcp and not portrange 0-1000";
        if (Native.pcap_compile(handle, ref filter, expression, 1, uint.MaxValue) != 0)
            throw new InvalidOperationException("pcap_compile failed: " + GetHandleError(handle));
        try
        {
            if (Native.pcap_setfilter(handle, ref filter) != 0)
                throw new InvalidOperationException("pcap_setfilter failed: " + GetHandleError(handle));
        }
        finally
        {
            Native.pcap_freecode(ref filter);
        }
    }

    private static void CheckPreActivate(int result, string name, IntPtr handle)
    {
        if (result != 0)
            throw new InvalidOperationException($"{name} failed ({result}): {GetHandleError(handle)}");
    }

    private static string GetHandleError(IntPtr handle)
    {
        var ptr = Native.pcap_geterr(handle);
        return ptr == IntPtr.Zero ? "unknown error" : Marshal.PtrToStringAnsi(ptr) ?? "unknown error";
    }

    private static string ReadErrorBuffer(IntPtr buffer) =>
        Marshal.PtrToStringAnsi(buffer) ?? "unknown error";

    private static void Zero(IntPtr pointer, int length)
    {
        for (var i = 0; i < length; i++) Marshal.WriteByte(pointer, i, 0);
    }

    private static bool _dllPathConfigured;
    private static readonly object DllPathLock = new();

    private static void EnsureNpcapDllSearchPath()
    {
        lock (DllPathLock)
        {
            if (_dllPathConfigured) return;

            var candidates = new[]
            {
                Path.Combine(Environment.SystemDirectory, "Npcap"),
                Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.Windows), "SysWOW64", "Npcap")
            };

            foreach (var dir in candidates)
            {
                if (!Directory.Exists(dir)) continue;
                if (!File.Exists(Path.Combine(dir, "wpcap.dll"))) continue;
                Native.SetDllDirectory(dir);
                AppLog.Write("npcap: dll search path=" + dir);
                _dllPathConfigured = true;
                return;
            }

            // Some Npcap installs expose wpcap.dll through the normal DLL search path.
            _dllPathConfigured = true;
        }
    }

    public void Dispose()
    {
        var handle = _handle;
        _handle = IntPtr.Zero;
        if (handle != IntPtr.Zero)
        {
            try { Native.pcap_close(handle); } catch { }
        }
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct PcapIf
    {
        internal IntPtr Next;
        internal IntPtr Name;
        internal IntPtr Description;
        internal IntPtr Addresses;
        internal uint Flags;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct TimeVal
    {
        internal int Seconds;
        internal int Microseconds;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct PcapPkthdr
    {
        internal TimeVal Timestamp;
        internal uint CapLen;
        internal uint Length;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct BpfProgram
    {
        internal uint Length;
        internal IntPtr Instructions;
    }

    private static class Native
    {
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool SetDllDirectory(string lpPathName);

        [DllImport("wpcap.dll", CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr pcap_lib_version();

        [DllImport("wpcap.dll", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int pcap_findalldevs(out IntPtr alldevs, IntPtr errbuf);

        [DllImport("wpcap.dll", CallingConvention = CallingConvention.Cdecl)]
        internal static extern void pcap_freealldevs(IntPtr alldevs);

        [DllImport("wpcap.dll", CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
        internal static extern IntPtr pcap_create(string source, IntPtr errbuf);

        [DllImport("wpcap.dll", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int pcap_set_snaplen(IntPtr p, int snaplen);

        [DllImport("wpcap.dll", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int pcap_set_promisc(IntPtr p, int promisc);

        [DllImport("wpcap.dll", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int pcap_set_timeout(IntPtr p, int timeoutMs);

        [DllImport("wpcap.dll", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int pcap_set_buffer_size(IntPtr p, int bufferSize);

        [DllImport("wpcap.dll", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int pcap_set_immediate_mode(IntPtr p, int immediate);

        [DllImport("wpcap.dll", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int pcap_activate(IntPtr p);

        [DllImport("wpcap.dll", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int pcap_datalink(IntPtr p);

        [DllImport("wpcap.dll", CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
        internal static extern int pcap_compile(IntPtr p, ref BpfProgram program, string expression, int optimize, uint netmask);

        [DllImport("wpcap.dll", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int pcap_setfilter(IntPtr p, ref BpfProgram program);

        [DllImport("wpcap.dll", CallingConvention = CallingConvention.Cdecl)]
        internal static extern void pcap_freecode(ref BpfProgram program);

        [DllImport("wpcap.dll", CallingConvention = CallingConvention.Cdecl)]
        internal static extern int pcap_next_ex(IntPtr p, out IntPtr header, out IntPtr data);

        [DllImport("wpcap.dll", CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr pcap_geterr(IntPtr p);

        [DllImport("wpcap.dll", CallingConvention = CallingConvention.Cdecl)]
        internal static extern void pcap_close(IntPtr p);
    }
}
