fn main() {
    println!("cargo:rerun-if-changed=native/render_emoji.c");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        // render-icons is macOS-only (Core Text); `generate` doesn't need
        // this native code at all, so let other platforms build fine.
        return;
    }

    cc::Build::new()
        .file("native/render_emoji.c")
        .compile("render_emoji");

    for framework in ["CoreText", "CoreGraphics", "CoreFoundation", "ImageIO"] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}
