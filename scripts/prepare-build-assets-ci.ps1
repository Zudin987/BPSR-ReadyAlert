$ErrorActionPreference = 'Stop'

function Find-Ffmpeg {
    $cmd = Get-Command ffmpeg -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }

    $candidates = @(
        'C:\ProgramData\chocolatey\bin\ffmpeg.exe',
        'C:\tools\ffmpeg\bin\ffmpeg.exe',
        'C:\Program Files\ffmpeg\bin\ffmpeg.exe'
    )
    foreach ($candidate in $candidates) {
        if (Test-Path $candidate) { return $candidate }
    }
    return $null
}

$ffmpeg = Find-Ffmpeg
if (-not $ffmpeg) {
    Write-Host 'ffmpeg is not preinstalled; installing with bounded retries...'
    $lastExit = 0
    for ($attempt = 1; $attempt -le 3 -and -not $ffmpeg; $attempt++) {
        Write-Host "Chocolatey ffmpeg install attempt $attempt/3"
        choco install ffmpeg -y --no-progress --limit-output
        $lastExit = $LASTEXITCODE

        # Chocolatey updates machine PATH outside this PowerShell process. Refresh
        # it before probing so a successful install is not mistaken for failure.
        $machinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        $env:Path = "$machinePath;$userPath"
        $ffmpeg = Find-Ffmpeg

        if (-not $ffmpeg -and $attempt -lt 3) {
            Start-Sleep -Seconds (5 * $attempt)
        }
    }
    if (-not $ffmpeg) {
        throw "ffmpeg installation failed after 3 attempts (last Chocolatey exit code $lastExit)."
    }
}

# Ensure the original deterministic asset builder sees the verified executable.
$ffmpegDir = Split-Path -Parent $ffmpeg
if (($env:Path -split ';') -notcontains $ffmpegDir) {
    $env:Path = "$ffmpegDir;$env:Path"
}

& (Join-Path $PSScriptRoot 'prepare-build-assets.ps1')
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}
