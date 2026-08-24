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

# First repair any historical malformed ICO directory so System.Drawing can safely
# open the source artwork. WinForms tolerated the early frames, while Explorer did not.
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

# Rebuild a clean multi-resolution ICO for Explorer. The artwork gets a small
# transparent safety margin (8 px on the 256 px master) so Windows medium/large
# icon views cannot clip the face at the image edge. Explicitly writing every
# common size avoids Explorer selecting/scaling a bad or incomplete ICO frame.
Add-Type -AssemblyName System.Drawing

$sourceIcon = $null
$sourceBitmap = $null
$master = $null
try {
    $sourceIcon = [System.Drawing.Icon]::new($IconPath, 256, 256)
    $sourceBitmap = $sourceIcon.ToBitmap()
    $master = [System.Drawing.Bitmap]::new(256, 256, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)

    $graphics = [System.Drawing.Graphics]::FromImage($master)
    try {
        $graphics.Clear([System.Drawing.Color]::Transparent)
        $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
        $graphics.DrawImage($sourceBitmap, 8, 8, 240, 240)
    } finally {
        $graphics.Dispose()
    }

    $sizes = @(16, 20, 24, 32, 40, 48, 64, 96, 128, 256)
    $pngFrames = New-Object System.Collections.Generic.List[byte[]]

    foreach ($dimension in $sizes) {
        $frame = [System.Drawing.Bitmap]::new($dimension, $dimension, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        $frameGraphics = [System.Drawing.Graphics]::FromImage($frame)
        try {
            $frameGraphics.Clear([System.Drawing.Color]::Transparent)
            $frameGraphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            $frameGraphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
            $frameGraphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
            $frameGraphics.DrawImage($master, 0, 0, $dimension, $dimension)

            $memory = [IO.MemoryStream]::new()
            try {
                $frame.Save($memory, [System.Drawing.Imaging.ImageFormat]::Png)
                $pngFrames.Add($memory.ToArray())
            } finally {
                $memory.Dispose()
            }
        } finally {
            $frameGraphics.Dispose()
            $frame.Dispose()
        }
    }

    $tempIcon = $IconPath + '.new'
    $stream = [IO.File]::Open($tempIcon, [IO.FileMode]::Create, [IO.FileAccess]::Write, [IO.FileShare]::None)
    $writer = [IO.BinaryWriter]::new($stream)
    try {
        $writer.Write([uint16]0)
        $writer.Write([uint16]1)
        $writer.Write([uint16]$sizes.Count)

        $dataOffset = 6 + (16 * $sizes.Count)
        for ($i = 0; $i -lt $sizes.Count; $i++) {
            $dimension = $sizes[$i]
            $dimensionByte = if ($dimension -eq 256) { [byte]0 } else { [byte]$dimension }
            $bytes = $pngFrames[$i]

            $writer.Write($dimensionByte)
            $writer.Write($dimensionByte)
            $writer.Write([byte]0)
            $writer.Write([byte]0)
            $writer.Write([uint16]1)
            $writer.Write([uint16]32)
            $writer.Write([uint32]$bytes.Length)
            $writer.Write([uint32]$dataOffset)
            $dataOffset += $bytes.Length
        }

        foreach ($bytes in $pngFrames) {
            $writer.Write($bytes)
        }
    } finally {
        $writer.Dispose()
        $stream.Dispose()
    }

    Move-Item -Force $tempIcon $IconPath
    Write-Host "Rebuilt App.ico with padded artwork and $($sizes.Count) explicit icon sizes."
} finally {
    if ($master) { $master.Dispose() }
    if ($sourceBitmap) { $sourceBitmap.Dispose() }
    if ($sourceIcon) { $sourceIcon.Dispose() }
}

# Final ICO directory validation: every declared image must be fully contained.
$finalIco = [IO.File]::ReadAllBytes($IconPath)
$finalCount = [BitConverter]::ToUInt16($finalIco, 4)
if ($finalCount -ne 10) { throw "Rebuilt App.ico expected 10 image entries, found $finalCount." }
for ($i = 0; $i -lt $finalCount; $i++) {
    $entryOffset = 6 + (16 * $i)
    $size = [BitConverter]::ToUInt32($finalIco, $entryOffset + 8)
    $dataOffset = [BitConverter]::ToUInt32($finalIco, $entryOffset + 12)
    if ($size -le 0 -or ([uint64]$dataOffset + [uint64]$size) -gt [uint64]$finalIco.Length) {
        throw "Rebuilt App.ico entry #$i is invalid (offset=$dataOffset size=$size)."
    }
}
Write-Host "App.ico final validation passed with $finalCount complete image entries."

Write-Host 'Build assets prepared. Npcap is an external runtime dependency and is not redistributed.' -ForegroundColor Green
