# Builds the M4b virtual audio driver (drivers/windows/OpenSeriesAudio).
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
$root = Join-Path $PSScriptRoot "..\..\drivers\windows\OpenSeriesAudio" | Resolve-Path

# EndpointsCommon is a static library the driver links against, so it is built first.
foreach ($project in "EndpointsCommon\EndpointsCommon.vcxproj", "TabletAudioSample\TabletAudioSample.vcxproj") {
    & $msbuild (Join-Path $root $project) "/p:Configuration=$Configuration" /p:Platform=x64 /m /nologo /v:minimal
    if ($LASTEXITCODE -ne 0) { throw "build failed: $project" }
}
# Package: driver + INF + catalog, signed with the WDK test certificate the build created
# (CN=WDKTestCert <user>, in CurrentUser\My). Loads only in test-signing mode.
$out = Join-Path $root "TabletAudioSample\x64\$Configuration"
$package = Join-Path $out "package"
Remove-Item $package -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory $package | Out-Null
Copy-Item (Join-Path $out "OpenSeriesAudio.sys"), (Join-Path $out "OpenSeriesAudio.inf"), (Join-Path $out "OpenSeriesAudio.cer") $package

$kitBin = Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10\bin\10.0.26100.0"
# The INF targets Windows 11 22H2 (NT...10.0...22621) and newer.
& (Join-Path $kitBin "x86\Inf2Cat.exe") "/driver:$package" /os:10_NI_X64,10_GE_X64 /uselocaltime
if ($LASTEXITCODE -ne 0) { throw "Inf2Cat failed" }

$cert = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2 (Join-Path $package "OpenSeriesAudio.cer")
& (Join-Path $kitBin "x64\signtool.exe") sign /sha1 $cert.Thumbprint /fd SHA256 /s My (Join-Path $package "openseriesaudio.cat")
if ($LASTEXITCODE -ne 0) { throw "signing the catalog failed" }

Get-ChildItem $package | ForEach-Object { $_.FullName }
