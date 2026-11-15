//! Embeds the app icon and version info into `stealseries.exe` (Windows targets only), so
//! Explorer, the taskbar, the Start menu and the installer show the StealSeries icon.

fn main() {
    println!("cargo:rerun-if-changed=bundle/icon/StealSeries.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon("bundle/icon/StealSeries.ico")
        .set("FileDescription", "StealSeries")
        .set("ProductName", "StealSeries")
        .set(
            "CompanyName",
            "StealSeries (independent, not affiliated with SteelSeries)",
        );
    if let Err(error) = resource.compile() {
        // Without the resource compiler the exe only lacks its icon; say so loudly.
        println!("cargo:warning=app icon not embedded: {error}");
    }
}
