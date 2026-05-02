fn main() {
    // Compile a single root .slint that re-exports both windows so
    // `slint::include_modules!()` exposes them together.
    slint_build::compile("ui/app.slint").unwrap();
}
