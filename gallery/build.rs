fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=src/assets/icons/widget_gallery_exe.ico");

    winresource::WindowsResource::new()
        .set_icon("src/assets/icons/widget_gallery_exe.ico")
        .compile()
}
