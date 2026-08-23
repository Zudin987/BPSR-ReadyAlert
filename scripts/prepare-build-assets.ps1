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

$SoundDestination = Join-Path $Root 'src\BPSR.ReadyAlert\Assets\LetsDoThis.wav'
$SoundSha = '0befc4c0b6a40ef374fb75c6f4c658850439ee43fa9a3c0d74d904c76627048a'
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $SoundDestination) | Out-Null
Invoke-WebRequest -UseBasicParsing -Uri 'https://raw.githubusercontent.com/Blue-Protocol-Source/BPSR-ZDPS/master/BPSR-ZDPS/Data/Audio/LetsDoThis.wav' -OutFile $SoundDestination
$actualSound = (Get-FileHash $SoundDestination -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualSound -ne $SoundSha) { throw "LetsDoThis.wav SHA-256 mismatch: $actualSound" }

Write-Host 'WinDivert 2.2.2 + LetsDoThis.wav build inputs prepared and verified.' -ForegroundColor Green
