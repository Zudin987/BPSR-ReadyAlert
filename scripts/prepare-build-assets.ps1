$ErrorActionPreference = 'Stop'

$Version = '2.2.2'
$Root = Split-Path -Parent $PSScriptRoot
$Destination = Join-Path $Root 'src\BPSR.ReadyAlert\vendor\WinDivert'
$Zip = Join-Path $env:TEMP "WinDivert-$Version-A.zip"
$Extract = Join-Path $env:TEMP "BPSR-ReadyAlert-WinDivert-$Version"

$DllSha = 'c1e060ee19444a259b2162f8af0f3fe8c4428a1c6f694dce20de194ac8d7d9a2'
$SysSha = '8da085332782708d8767bcace5327a6ec7283c17cfb85e40b03cd2323a90ddc2'

Remove-Item $Extract -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $Destination | Out-Null

Write-Host "Preparing WinDivert $Version..."
Invoke-WebRequest -UseBasicParsing -Uri "https://github.com/basil00/WinDivert/releases/download/v$Version/WinDivert-$Version-A.zip" -OutFile $Zip
Expand-Archive -Path $Zip -DestinationPath $Extract -Force

$PackageRoot = Join-Path $Extract "WinDivert-$Version-A"
Copy-Item (Join-Path $PackageRoot 'x64\WinDivert.dll') (Join-Path $Destination 'WinDivert.dll') -Force
Copy-Item (Join-Path $PackageRoot 'x64\WinDivert64.sys') (Join-Path $Destination 'WinDivert64.sys') -Force
Copy-Item (Join-Path $PackageRoot 'LICENSE') (Join-Path $Destination 'LICENSE') -Force

$actualDll = (Get-FileHash (Join-Path $Destination 'WinDivert.dll') -Algorithm SHA256).Hash.ToLowerInvariant()
$actualSys = (Get-FileHash (Join-Path $Destination 'WinDivert64.sys') -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualDll -ne $DllSha) { throw "WinDivert.dll SHA-256 mismatch: $actualDll" }
if ($actualSys -ne $SysSha) { throw "WinDivert64.sys SHA-256 mismatch: $actualSys" }

# Rebuild the user-supplied LetsDoThis alert from a compact base64 MP3 source.
$SoundDestination = Join-Path $Root 'src\BPSR.ReadyAlert\Assets\LetsDoThis.wav'
$AudioSource = Join-Path $Root 'assets-src\LetsDoThis.user.mp3.b64'
$Mp3Temp = Join-Path $env:TEMP 'BPSR-ReadyAlert-LetsDoThis.mp3'
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $SoundDestination) | Out-Null

if (-not (Test-Path $AudioSource)) {
    throw 'Bundled user-supplied LetsDoThis audio source is missing.'
}

Write-Host 'Reconstructing bundled user-supplied LetsDoThis sound...'
$encoded = (Get-Content $AudioSource -Raw) -replace '[^A-Za-z0-9+/=]', ''
# Normalize padding so a line-ending or contents-API edit cannot break decoding.
$encoded = $encoded -replace '=', ''
switch ($encoded.Length % 4) {
    0 { }
    2 { $encoded += '==' }
    3 { $encoded += '=' }
    1 { throw "Bundled LetsDoThis base64 has an impossible length ($($encoded.Length))." }
}
try {
    $mp3Bytes = [Convert]::FromBase64String($encoded)
} catch {
    throw "Bundled LetsDoThis audio is not valid base64: $($_.Exception.Message)"
}
[IO.File]::WriteAllBytes($Mp3Temp, $mp3Bytes)

if ((Get-Item $Mp3Temp).Length -lt 1024) {
    throw 'Bundled LetsDoThis audio is unexpectedly small.'
}

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

Write-Host 'WinDivert 2.2.2 + bundled LetsDoThis alert prepared and verified.' -ForegroundColor Green
