$ErrorActionPreference = 'Stop'

$Root = Split-Path -Parent $PSScriptRoot

# Rebuild the user-supplied LetsDoThis alert from exact, ordered base64 chunks.
$SoundDestination = Join-Path $Root 'src\BPSR.ReadyAlert\Assets\LetsDoThis.wav'
$AudioSourceDir = Join-Path $Root 'assets-src'
$Mp3Temp = Join-Path $env:TEMP 'BPSR-ReadyAlert-LetsDoThis.mp3'
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $SoundDestination) | Out-Null

$chunks = @(
    Get-ChildItem -Path $AudioSourceDir -File |
        Where-Object { $_.Name -match '^LetsDoThis\.user\.mp3\.b64\.\d{3}$' } |
        Sort-Object Name
)
if ($chunks.Count -ne 13) {
    throw "Expected 13 bundled LetsDoThis audio chunks, found $($chunks.Count)."
}

Write-Host "Reconstructing bundled user-supplied LetsDoThis sound from $($chunks.Count) chunks..."
$encoded = ($chunks | ForEach-Object { (Get-Content $_.FullName -Raw).Trim() }) -join ''
$encoded = $encoded -replace '\s', ''
if ($encoded.Length -ne 10812) {
    throw "Bundled LetsDoThis base64 length mismatch: expected 10812, got $($encoded.Length)."
}
if (($encoded.Length % 4) -ne 0) {
    throw "Bundled LetsDoThis base64 length is not divisible by 4: $($encoded.Length)."
}

try {
    $mp3Bytes = [Convert]::FromBase64String($encoded)
} catch {
    throw "Bundled LetsDoThis audio is not valid base64: $($_.Exception.Message)"
}
if ($mp3Bytes.Length -ne 8108) {
    throw "Bundled LetsDoThis MP3 length mismatch: expected 8108 bytes, got $($mp3Bytes.Length)."
}
[IO.File]::WriteAllBytes($Mp3Temp, $mp3Bytes)

$ffmpeg = Get-Command ffmpeg -ErrorAction SilentlyContinue
if (-not $ffmpeg) {
    Write-Host 'ffmpeg is not present on this runner; installing it with Chocolatey...'
    choco install ffmpeg -y --no-progress
    $ffmpeg = Get-Command ffmpeg -ErrorAction SilentlyContinue
}
if (-not $ffmpeg) {
    throw 'ffmpeg is required to prepare the embedded WAV but could not be found.'
}

& $ffmpeg.Source -hide_banner -loglevel error -y -i $Mp3Temp -ac 1 -ar 48000 -c:a pcm_s16le $SoundDestination
if ($LASTEXITCODE -ne 0) {
    throw "ffmpeg failed to prepare LetsDoThis.wav (exit code $LASTEXITCODE)."
}
if (-not (Test-Path $SoundDestination) -or (Get-Item $SoundDestination).Length -le 44) {
    throw 'LetsDoThis.wav was not generated correctly.'
}

$header = [IO.File]::ReadAllBytes($SoundDestination)[0..3]
if ([Text.Encoding]::ASCII.GetString($header) -ne 'RIFF') {
    throw 'Generated LetsDoThis.wav does not have a valid RIFF header.'
}
$actualSound = (Get-FileHash $SoundDestination -Algorithm SHA256).Hash.ToLowerInvariant()
Write-Host "Bundled alert WAV prepared. SHA-256: $actualSound"

# Repair the historical App.ico if its directory advertises a truncated image.
# WinForms was tolerant enough to use the early frames for the tray, but Windows
# Explorer rejects the malformed ICO as an application icon. Keep every complete
# image entry and rebuild a clean ICO directory with corrected offsets.
$IconPath = Join-Path $Root 'src\BPSR.ReadyAlert\Assets\App.ico'
$ico = [IO.File]::ReadAllBytes($IconPath)
if ($ico.Length -lt 22 -or [BitConverter]::ToUInt16($ico, 0) -ne 0 -or [BitConverter]::ToUInt16($ico, 2) -ne 1) {
    throw 'Assets/App.ico has an invalid ICO header.'
}

$declaredCount = [BitConverter]::ToUInt16($ico, 4)
$valid = @()
for ($i = 0; $i -lt $declaredCount; $i++) {
    $entryOffset = 6 + (16 * $i)
    if (($entryOffset + 16) -gt $ico.Length) { break }
    $size = [BitConverter]::ToUInt32($ico, $entryOffset + 8)
    $dataOffset = [BitConverter]::ToUInt32($ico, $entryOffset + 12)
    if ($size -gt 0 -and ([uint64]$dataOffset + [uint64]$size) -le [uint64]$ico.Length) {
        $valid += [pscustomobject]@{ EntryOffset = $entryOffset; Size = [int]$size; DataOffset = [int]$dataOffset }
    } else {
        Write-Host "Dropping truncated App.ico image entry #$i (offset=$dataOffset size=$size)."
    }
}

if ($valid.Count -lt 1) { throw 'Assets/App.ico contains no complete images.' }
if ($valid.Count -ne $declaredCount) {
    $newLength = 6 + (16 * $valid.Count) + (($valid | Measure-Object -Property Size -Sum).Sum)
    $fixed = New-Object byte[] $newLength
    [Array]::Copy($ico, 0, $fixed, 0, 4)
    [Array]::Copy([BitConverter]::GetBytes([uint16]$valid.Count), 0, $fixed, 4, 2)
    $cursor = 6 + (16 * $valid.Count)

    for ($i = 0; $i -lt $valid.Count; $i++) {
        $item = $valid[$i]
        $dstEntry = 6 + (16 * $i)
        [Array]::Copy($ico, $item.EntryOffset, $fixed, $dstEntry, 12)
        [Array]::Copy([BitConverter]::GetBytes([uint32]$cursor), 0, $fixed, $dstEntry + 12, 4)
        [Array]::Copy($ico, $item.DataOffset, $fixed, $cursor, $item.Size)
        $cursor += $item.Size
    }

    [IO.File]::WriteAllBytes($IconPath, $fixed)
    Write-Host "Repaired App.ico: kept $($valid.Count) of $declaredCount image entries."
} else {
    Write-Host "App.ico validation passed with $declaredCount image entries."
}

Write-Host 'Build assets prepared. Npcap is an external runtime dependency and is not redistributed.' -ForegroundColor Green
