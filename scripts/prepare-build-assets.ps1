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

# App.ico is generated from the user-supplied PNG master before committing.
# Do not transform it during CI: doing so can make Windows select/scale from the
# wrong ICO frame. Validate that the committed ICO contains the expected complete
# multi-resolution images and leave the bytes unchanged for compilation.
$IconPath = Join-Path $Root 'src\BPSR.ReadyAlert\Assets\App.ico'
$ico = [IO.File]::ReadAllBytes($IconPath)
if ($ico.Length -lt 22 -or [BitConverter]::ToUInt16($ico, 0) -ne 0 -or [BitConverter]::ToUInt16($ico, 2) -ne 1) {
    throw 'Assets/App.ico has an invalid ICO header.'
}

$expectedSizes = @(16, 20, 24, 32, 40, 48, 64, 96, 128, 256)
$count = [BitConverter]::ToUInt16($ico, 4)
if ($count -ne $expectedSizes.Count) {
    throw "Assets/App.ico expected $($expectedSizes.Count) image entries, found $count."
}

for ($i = 0; $i -lt $count; $i++) {
    $entryOffset = 6 + (16 * $i)
    if (($entryOffset + 16) -gt $ico.Length) { throw "Assets/App.ico entry #$i is truncated." }

    $widthByte = $ico[$entryOffset]
    $heightByte = $ico[$entryOffset + 1]
    $width = if ($widthByte -eq 0) { 256 } else { [int]$widthByte }
    $height = if ($heightByte -eq 0) { 256 } else { [int]$heightByte }
    $expected = $expectedSizes[$i]
    if ($width -ne $expected -or $height -ne $expected) {
        throw "Assets/App.ico entry #$i expected ${expected}x${expected}, found ${width}x${height}."
    }

    $size = [BitConverter]::ToUInt32($ico, $entryOffset + 8)
    $dataOffset = [BitConverter]::ToUInt32($ico, $entryOffset + 12)
    if ($size -le 0 -or ([uint64]$dataOffset + [uint64]$size) -gt [uint64]$ico.Length) {
        throw "Assets/App.ico entry #$i is invalid (offset=$dataOffset size=$size)."
    }
}

$iconHash = (Get-FileHash $IconPath -Algorithm SHA256).Hash.ToLowerInvariant()
Write-Host "App.ico validation passed with $count complete image entries. SHA-256: $iconHash"
Write-Host 'Build assets prepared. Npcap is an external runtime dependency and is not redistributed.' -ForegroundColor Green
