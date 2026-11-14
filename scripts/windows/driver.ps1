# Builds the M4b virtual audio driver (drivers/windows/StealSeriesAudio).
# See docs/architecture/windows-virtual-driver.md §3.
#
#   powershell -ExecutionPolicy Bypass -File scripts\windows\driver.ps1 [-Configuration Debug|Release]
#
# Needs Visual Studio Build Tools 2022 with the components
#   Microsoft.VisualStudio.Component.VC.Runtimes.x86.x64.Spectre and
#   Component.Microsoft.Windows.DriverKit.BuildTools, plus WDK 10.0.26100.
# Uses the 64-bit MSBuild: the WDK ships InfVerif only as a 64-bit DLL.
param(
    [ValidateSet("Debug", "Release")]
    [string]$Configuration = "Debug"
)
$ErrorActionPreference = "Stop"

$vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
$buildTools = & $vswhere -products * -requires Component.Microsoft.Windows.DriverKit.BuildTools -property installationPath | Select-Object -First 1
if (-not $buildTools) {
    throw "Build Tools with the Windows Driver Kit component not found (see the header of this script)."
}
$msbuild = Join-Path $buildTools "MSBuild\Current\Bin\amd64\MSBuild.exe"
$root = Join-Path $PSScriptRoot "..\..\drivers\windows\StealSeriesAudio" | Resolve-Path

# EndpointsCommon is a static library the driver links against, so it is built first.
foreach ($project in "EndpointsCommon\EndpointsCommon.vcxproj", "TabletAudioSample\TabletAudioSample.vcxproj") {
    & $msbuild (Join-Path $root $project) "/p:Configuration=$Configuration" /p:Platform=x64 /m /nologo /v:minimal
    if ($LASTEXITCODE -ne 0) { throw "build failed: $project" }
}
Get-ChildItem (Join-Path $root "TabletAudioSample\x64\$Configuration") -Recurse -Include *.sys, *.inf, *.cat |
    ForEach-Object { $_.FullName }
