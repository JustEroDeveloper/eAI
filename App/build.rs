fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        // Point this to where your .ico file is located
        res.set_icon("assets/eai-lite.ico");
        res.compile().unwrap();
    }
}
