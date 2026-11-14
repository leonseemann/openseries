# Windows drivers

`StealSeriesAudio/` — the M4b virtual audio driver
([design](../../docs/architecture/windows-virtual-driver.md), [ADR 0004](../../docs/adr/0004-windows-audio-strategy.md)).

It starts as an import of Microsoft's **SysVAD** sample (`audio/sysvad` of
[microsoft/Windows-driver-samples](https://github.com/microsoft/Windows-driver-samples), commit in
`StealSeriesAudio/UPSTREAM_COMMIT`), licensed **MS-PL** (`StealSeriesAudio/LICENSE-MS-PL`). These
files keep their MS-PL licence and notices; they are a separate component next to the MIT/Apache
Rust code and are not part of the Cargo workspace. Our changes are made in later, separate
commits so the diff to upstream stays reviewable.

Removed from the import: the sample APOs (`APO/`) and the keyword detector adapter — separate
projects we do not need.

Build (Visual Studio Build Tools 2022 with `Component.Microsoft.Windows.DriverKit.BuildTools` and
`Microsoft.VisualStudio.Component.VC.Runtimes.x86.x64.Spectre`, plus WDK 10.0.26100 —
**Observed** 2026-11-14: the unchanged import builds):

```powershell
powershell -ExecutionPolicy Bypass -File scripts\windows\driver.ps1
```

Notes: the script uses the 64-bit MSBuild (the WDK's InfVerif exists only as a 64-bit DLL; the
32-bit MSBuild fails with "x86\InfVerif.dll not found") and builds the `EndpointsCommon` library
before the driver. The component id `Microsoft.Windows.DriverKit` does not exist for the Build Tools;
the Build Tools one is `Component.Microsoft.Windows.DriverKit.BuildTools`.
