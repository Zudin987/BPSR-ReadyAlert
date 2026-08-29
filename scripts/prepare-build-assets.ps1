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
if (-not $ffmpeg) { throw 'ffmpeg is required to prepare alert WAV assets but could not be found.' }

& $ffmpeg.Source -hide_banner -loglevel error -y -i $Mp3Temp -ac 1 -ar 48000 -c:a pcm_s16le $SoundDestination
if ($LASTEXITCODE -ne 0) { throw "ffmpeg failed to prepare LetsDoThis.wav (exit code $LASTEXITCODE)." }
if (-not (Test-Path $SoundDestination) -or (Get-Item $SoundDestination).Length -le 44) {
    throw 'LetsDoThis.wav was not generated correctly.'
}
$header = [IO.File]::ReadAllBytes($SoundDestination)[0..3]
if ([Text.Encoding]::ASCII.GetString($header) -ne 'RIFF') { throw 'Generated LetsDoThis.wav does not have a valid RIFF header.' }
Write-Host "Bundled alert WAV prepared. SHA-256: $((Get-FileHash $SoundDestination -Algorithm SHA256).Hash.ToLowerInvariant())"

# Reconstruct the four user-supplied v1.3.1 core alert sounds from one lossless
# FLAC archive. The source is split only because repository writes are text-only;
# lexical sorting preserves the exact original base64 stream.
$CoreSourceChunks = @(
    Get-ChildItem -Path $AudioSourceDir -File |
        Where-Object { $_.Name -like 'CoreAlerts.user.flac.tar.xz.b64.*' } |
        Sort-Object Name
)
if ($CoreSourceChunks.Count -lt 21) {
    throw "Bundled core alert source is incomplete: expected at least 21 ordered chunks, found $($CoreSourceChunks.Count)."
}

$CoreEncoded = ($CoreSourceChunks | ForEach-Object { (Get-Content $_.FullName -Raw).Trim() }) -join ''
$CoreEncoded = $CoreEncoded -replace '\s', ''
if ($CoreEncoded.Length -ne 330060) {
    throw "Bundled core alert base64 length mismatch: expected 330060, got $($CoreEncoded.Length)."
}

try {
    $CoreArchiveBytes = [Convert]::FromBase64String($CoreEncoded)
} catch {
    throw "Bundled core alert source is not valid base64: $($_.Exception.Message)"
}
if ($CoreArchiveBytes.Length -ne 248004) {
    throw "Bundled core alert archive length mismatch: expected 248004 bytes, got $($CoreArchiveBytes.Length)."
}

$CoreArchiveTemp = Join-Path $env:TEMP 'BPSR-ReadyAlert-CoreAlerts.tar.xz'
$CoreExtractTemp = Join-Path $env:TEMP ('BPSR-ReadyAlert-CoreAlerts-' + [Guid]::NewGuid().ToString('N'))
[IO.File]::WriteAllBytes($CoreArchiveTemp, $CoreArchiveBytes)
$CoreArchiveHash = (Get-FileHash $CoreArchiveTemp -Algorithm SHA256).Hash.ToLowerInvariant()
if ($CoreArchiveHash -ne '9d1291d7220d05835dddacf5bbd709058bfc290c5a5b1e793fe3141c38cc424d') {
    throw "Bundled core alert archive SHA-256 mismatch: expected 9d1291d7220d05835dddacf5bbd709058bfc290c5a5b1e793fe3141c38cc424d, got $CoreArchiveHash."
}

New-Item -ItemType Directory -Force -Path $CoreExtractTemp | Out-Null
try {
    $tar = Get-Command tar.exe -ErrorAction SilentlyContinue
    if (-not $tar) { $tar = Get-Command tar -ErrorAction SilentlyContinue }
    if (-not $tar) { throw 'tar is required to unpack bundled core alert audio sources but could not be found.' }

    & $tar.Source -xf $CoreArchiveTemp -C $CoreExtractTemp
    if ($LASTEXITCODE -ne 0) { throw "tar failed to unpack bundled core alert sources (exit code $LASTEXITCODE)." }

    $CoreMappings = @(
        @{ Source = 'queue.flac';         Destination = 'Queue.wav' },
        @{ Source = 'ready-check.flac';   Destination = 'ReadyCheck.wav' },
        @{ Source = 'party-invite.flac';  Destination = 'PartyInvite.wav' },
        @{ Source = 'party-request.flac'; Destination = 'PartyRequest.wav' }
    )

    foreach ($mapping in $CoreMappings) {
        $source = Join-Path $CoreExtractTemp $mapping.Source
        $destination = Join-Path $Root ('src\BPSR.ReadyAlert\Assets\' + $mapping.Destination)
        if (-not (Test-Path $source)) { throw "Bundled core alert source is missing $($mapping.Source)." }

        & $ffmpeg.Source -hide_banner -loglevel error -y -i $source -ac 1 -ar 44100 -c:a pcm_s16le $destination
        if ($LASTEXITCODE -ne 0) { throw "ffmpeg failed to prepare $($mapping.Destination) (exit code $LASTEXITCODE)." }
        if (-not (Test-Path $destination) -or (Get-Item $destination).Length -le 44) {
            throw "$($mapping.Destination) was not generated correctly."
        }
        $wavHeader = [IO.File]::ReadAllBytes($destination)[0..3]
        if ([Text.Encoding]::ASCII.GetString($wavHeader) -ne 'RIFF') {
            throw "Generated $($mapping.Destination) does not have a valid RIFF header."
        }
        Write-Host "$($mapping.Destination) prepared. SHA-256: $((Get-FileHash $destination -Algorithm SHA256).Hash.ToLowerInvariant())"
    }
} finally {
    Remove-Item $CoreArchiveTemp -Force -ErrorAction SilentlyContinue
    Remove-Item $CoreExtractTemp -Recurse -Force -ErrorAction SilentlyContinue
}

# Exact icon policy: do not resize, crop, pad or re-render the supplied artwork.
# The 16x16 frame is used by Explorer Details/Small Icons and the system tray;
# the 48x48 frame is used by normal Explorer icon views.
function Get-SingleIcoFrame([string]$Base64Path, [int]$ExpectedSize) {
    $rawText = (Get-Content $Base64Path -Raw) -replace '\s', ''
    $ico = [Convert]::FromBase64String($rawText)
    if ($ico.Length -lt 22 -or [BitConverter]::ToUInt16($ico, 0) -ne 0 -or [BitConverter]::ToUInt16($ico, 2) -ne 1) {
        throw "$Base64Path is not a valid ICO file."
    }
    $count = [BitConverter]::ToUInt16($ico, 4)
    if ($count -ne 1) { throw "$Base64Path must contain exactly one icon frame; found $count." }
    $w = [int]$ico[6]; if ($w -eq 0) { $w = 256 }
    $h = [int]$ico[7]; if ($h -eq 0) { $h = 256 }
    if ($w -ne $ExpectedSize -or $h -ne $ExpectedSize) {
        throw "$Base64Path expected ${ExpectedSize}x${ExpectedSize}; found ${w}x${h}."
    }
    $planes = [BitConverter]::ToUInt16($ico, 10)
    $bitCount = [BitConverter]::ToUInt16($ico, 12)
    $size = [BitConverter]::ToUInt32($ico, 14)
    $offset = [BitConverter]::ToUInt32($ico, 18)
    if ($size -le 0 -or ([uint64]$offset + [uint64]$size) -gt [uint64]$ico.Length) {
        throw "$Base64Path has a truncated icon payload."
    }
    $payload = New-Object byte[] $size
    [Array]::Copy($ico, [int]$offset, $payload, 0, [int]$size)
    return [pscustomobject]@{ Size=$ExpectedSize; Planes=$planes; BitCount=$bitCount; Payload=$payload }
}

$frame16 = Get-SingleIcoFrame (Join-Path $AudioSourceDir 'AppExact16.ico.b64') 16
$frame48 = Get-SingleIcoFrame (Join-Path $AudioSourceDir 'AppExact48.ico.b64') 48
$frames = @($frame16, $frame48)
$IconPath = Join-Path $Root 'src\BPSR.ReadyAlert\Assets\App.ico'
$stream = [IO.File]::Open($IconPath, [IO.FileMode]::Create, [IO.FileAccess]::Write, [IO.FileShare]::None)
$writer = [IO.BinaryWriter]::new($stream)
try {
    $writer.Write([uint16]0)
    $writer.Write([uint16]1)
    $writer.Write([uint16]$frames.Count)
    $dataOffset = 6 + (16 * $frames.Count)
    foreach ($frame in $frames) {
        $dimensionByte = if ($frame.Size -eq 256) { [byte]0 } else { [byte]$frame.Size }
        $writer.Write($dimensionByte)
        $writer.Write($dimensionByte)
        $writer.Write([byte]0)
        $writer.Write([byte]0)
        $writer.Write([uint16]$frame.Planes)
        $writer.Write([uint16]$frame.BitCount)
        $writer.Write([uint32]$frame.Payload.Length)
        $writer.Write([uint32]$dataOffset)
        $dataOffset += $frame.Payload.Length
    }
    foreach ($frame in $frames) { $writer.Write($frame.Payload) }
} finally {
    $writer.Dispose()
    $stream.Dispose()
}

# Verify the final app icon contains exactly the two supplied frames in the intended order.
$final = [IO.File]::ReadAllBytes($IconPath)
$count = [BitConverter]::ToUInt16($final, 4)
if ($count -ne 2) { throw "Final App.ico expected 2 frames, found $count." }
$actualSizes = @()
for ($i = 0; $i -lt $count; $i++) {
    $entry = 6 + (16 * $i)
    $w = [int]$final[$entry]; if ($w -eq 0) { $w = 256 }
    $actualSizes += $w
}
if (($actualSizes -join ',') -ne '16,48') { throw "Final App.ico frame order is $($actualSizes -join ','); expected 16,48." }
Write-Host 'App.ico rebuilt from exact supplied frames: 16x16 + 48x48. No scaling or padding applied.' -ForegroundColor Green
Write-Host 'Build assets prepared. Npcap is an external runtime dependency and is not redistributed.' -ForegroundColor Green
