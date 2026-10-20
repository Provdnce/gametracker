// Embeds the program icon into the Windows .exe (Explorer, taskbar, window title).
fn main() {
    println!("cargo:rerun-if-changed=resources/gametracker.rc");
    println!("cargo:rerun-if-changed=resources/gametracker.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("resources/gametracker.rc", embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }
}
