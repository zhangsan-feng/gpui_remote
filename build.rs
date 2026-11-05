fn main() {
    let build_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("cargo:rustc-env=BUILD_TIME={build_time}");

    // Keep this path absent so Cargo refreshes BUILD_TIME on every build invocation.
    let always_rerun =
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR is set"))
            .join("always-rerun");
    println!("cargo:rerun-if-changed={}", always_rerun.display());
    println!("cargo:rerun-if-changed=assets/app/app.rc");
    println!("cargo:rerun-if-changed=assets/app/app-logo-v3.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("assets/app/app.rc", embed_resource::NONE)
            .manifest_required()
            .expect("failed to embed the application icon");
    }
}
