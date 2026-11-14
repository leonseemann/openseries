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

Build (Visual Studio Build Tools 2022 + WDK 10.0.26100 + the "Windows Driver Kit" and
"Spectre-mitigated libs" components):

```powershell
msbuild drivers\windows\StealSeriesAudio\TabletAudioSample\TabletAudioSample.vcxproj /p:Configuration=Debug /p:Platform=x64
```
