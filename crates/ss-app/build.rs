//! Embeds the app icon and version info into `openseries.exe` (Windows targets only), so
//! Explorer, the taskbar, the Start menu and the installer show the OpenSeries icon.

fn main() {
    println!("cargo:rerun-if-changed=bundle/icon/OpenSeries.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon("bundle/icon/OpenSeries.ico")
        .set("FileDescription", "OpenSeries")
        .set("ProductName", "OpenSeries")
        .set(
            "CompanyName",
            "OpenSeries (independent, not affiliated with SteelSeries)",
        );
    if let Err(error) = resource.compile() {
        // Without the resource compiler the exe only lacks its icon; say so loudly.
        println!("cargo:warning=app icon not embedded: {error}");
    }
}
