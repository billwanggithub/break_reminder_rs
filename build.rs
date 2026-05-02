fn main() {
    // Compile a single root .slint that re-exports both windows so
    // `slint::include_modules!()` exposes them together.
    slint_build::compile("ui/app.slint").unwrap();

    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/app.ico");
        if let Err(e) = res.compile() {
            eprintln!("warning: winres failed to embed icon: {e}");
        }
    }
}
