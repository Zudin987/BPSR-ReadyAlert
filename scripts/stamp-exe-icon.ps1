param(
    [Parameter(Mandatory = $true)][string]$ExePath,
    [Parameter(Mandatory = $true)][string]$IconPath
)

$ErrorActionPreference = 'Stop'
$ExePath = (Resolve-Path $ExePath).Path
$IconPath = (Resolve-Path $IconPath).Path

Add-Type @"
using System;
using System.Runtime.InteropServices;

public static class ReadyAlertResourceApi
{
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr BeginUpdateResource(string pFileName, bool bDeleteExistingResources);

    [DllImport("kernel32.dll", EntryPoint = "UpdateResourceW", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool UpdateResource(IntPtr hUpdate, IntPtr lpType, IntPtr lpName, ushort wLanguage, byte[] lpData, uint cbData);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool EndUpdateResource(IntPtr hUpdate, bool fDiscard);
}
"@

$ico = [IO.File]::ReadAllBytes($IconPath)
if ($ico.Length -lt 22) { throw 'ICO is too small.' }
if ([BitConverter]::ToUInt16($ico, 0) -ne 0 -or [BitConverter]::ToUInt16($ico, 2) -ne 1) {
    throw 'Invalid ICO header.'
}

$count = [BitConverter]::ToUInt16($ico, 4)
if ($count -lt 1) { throw 'ICO contains no images.' }
if ($ico.Length -lt (6 + 16 * $count)) { throw 'ICO directory is truncated.' }

# GRPICONDIR = ICONDIR header + count * GRPICONDIRENTRY (14 bytes each).
$group = New-Object byte[] (6 + (14 * $count))
[Array]::Copy($ico, 0, $group, 0, 6)

$handle = [ReadyAlertResourceApi]::BeginUpdateResource($ExePath, $false)
if ($handle -eq [IntPtr]::Zero) {
    throw "BeginUpdateResource failed: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
}

$committed = $false
try {
    for ($i = 0; $i -lt $count; $i++) {
        $src = 6 + (16 * $i)
        $size = [BitConverter]::ToUInt32($ico, $src + 8)
        $offset = [BitConverter]::ToUInt32($ico, $src + 12)
        if (($offset + $size) -gt $ico.Length) { throw "ICO image $i is truncated." }

        $image = New-Object byte[] $size
        [Array]::Copy($ico, [int]$offset, $image, 0, [int]$size)
        $resourceId = $i + 1

        if (-not [ReadyAlertResourceApi]::UpdateResource(
            $handle, [IntPtr]3, [IntPtr]$resourceId, 0, $image, [uint32]$image.Length)) {
            throw "UpdateResource(RT_ICON $resourceId) failed: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
        }

        $dst = 6 + (14 * $i)
        # width,height,color-count,reserved,planes,bit-count,bytes-in-resource
        [Array]::Copy($ico, $src, $group, $dst, 12)
        $idBytes = [BitConverter]::GetBytes([uint16]$resourceId)
        [Array]::Copy($idBytes, 0, $group, $dst + 12, 2)
    }

    # C# app icons commonly use 32512. Also write ID 1 for shells/tools that prefer it.
    foreach ($groupId in @(1, 32512)) {
        if (-not [ReadyAlertResourceApi]::UpdateResource(
            $handle, [IntPtr]14, [IntPtr]$groupId, 0, $group, [uint32]$group.Length)) {
            throw "UpdateResource(RT_GROUP_ICON $groupId) failed: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
        }
    }

    if (-not [ReadyAlertResourceApi]::EndUpdateResource($handle, $false)) {
        throw "EndUpdateResource failed: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
    }
    $committed = $true
}
finally {
    if (-not $committed -and $handle -ne [IntPtr]::Zero) {
        [void][ReadyAlertResourceApi]::EndUpdateResource($handle, $true)
    }
}

Write-Host "Stamped Explorer icon into $ExePath using $count ICO image(s)." -ForegroundColor Green
