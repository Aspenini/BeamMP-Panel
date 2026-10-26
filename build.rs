fn main() {
    println!("cargo:rerun-if-changed=icons/BeamMP-Panel.ico");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Build scripts run on the host; resources belong to the target platform.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("icons/BeamMP-Panel.ico")
            .set("ProductName", "BeamMP Panel")
            .set("FileDescription", "BeamMP Panel")
            .set("OriginalFilename", "BeamMP-Panel.exe");
        res.compile()
            .expect("failed to compile Windows executable resources");
    }
}
