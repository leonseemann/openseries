# Installs or removes the test-signed StealSeries audio driver (M4b development only).
# docs/architecture/windows-virtual-driver.md §3. Run in an ADMINISTRATOR PowerShell:
#
#   powershell -ExecutionPolicy Bypass -File scripts\windows\driver-install.ps1            # install
#   powershell -ExecutionPolicy Bypass -File scripts\windows\driver-install.ps1 -Uninstall # remove
#
# Requires test-signing mode (`bcdedit /set testsigning on`, reboot). Build first with
# scripts\windows\driver.ps1. Every step is reversible with -Uninstall.
param(
    [switch]$Uninstall,
    [ValidateSet("Debug", "Release")]
    [string]$Configuration = "Debug"
)
$ErrorActionPreference = "Stop"

$hardwareId = "Root\StealSeriesAudio"
$devcon = Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10\Tools\10.0.26100.0\x64\devcon.exe"
$package = Join-Path $PSScriptRoot "..\..\drivers\windows\StealSeriesAudio\TabletAudioSample\x64\$Configuration\package" |
    Resolve-Path -ErrorAction SilentlyContinue

$principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw "Run this script in an administrator PowerShell."
}

function Remove-StealSeriesDriver {
    # Remove the root-enumerated device, then every driver package that came from our INF.
    & $devcon remove $hardwareId | Out-Host
    $published = pnputil /enum-drivers | Out-String
    $blocks = $published -split "(\r?\n){2,}"
    foreach ($block in $blocks) {
        if ($block -match "stealseriesaudio\.inf" -and $block -match "(oem\d+\.inf)") {
            pnputil /delete-driver $Matches[1] /uninstall /force | Out-Host
        }
    }
}

if ($Uninstall) {
    Remove-StealSeriesDriver
    Write-Host "StealSeries audio driver removed. The test certificate stays trusted; remove it in"
    Write-Host "certlm.msc (Trusted Root / Trusted Publishers, 'WDKTestCert') if you no longer need it."
    return
}

$bcd = bcdedit /enum "{current}" | Out-String
if ($bcd -notmatch "testsigning\s+(Yes|Ja)") {
    throw "Test-signing mode is off. Run 'bcdedit /set testsigning on' as administrator and reboot first."
}
if (-not $package) {
    throw "No driver package found. Build it first: scripts\windows\driver.ps1"
}

# Trust the WDK test certificate the package is signed with (machine-wide, test PCs only).
$cer = Join-Path $package "StealSeriesAudio.cer"
certutil -addstore Root $cer | Out-Host
certutil -addstore TrustedPublisher $cer | Out-Host

# Replace an older installation, then create the root device with our INF.
Remove-StealSeriesDriver
& $devcon install (Join-Path $package "StealSeriesAudio.inf") $hardwareId | Out-Host
if ($LASTEXITCODE -ne 0) { throw "devcon install failed ($LASTEXITCODE)" }
Write-Host "Installed. Sound settings should now list Game, Chat, Media, Aux (StealSeries Audio)"
Write-Host "and Microphone (StealSeries Audio)."
