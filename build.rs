fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("icons/BeamMP-Panel.ico");
        res.compile().unwrap();
    }
}

