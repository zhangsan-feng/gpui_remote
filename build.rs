fn main() {
    let build_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("cargo:rustc-env=BUILD_TIME={build_time}");

    println!("cargo:rerun-if-changed=assets/app/app.rc");
    println!("cargo:rerun-if-changed=assets/app/app-logo-v3.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("assets/app/app.rc", embed_resource::NONE)
            .manifest_required()
            .expect("failed to embed the application icon");
    }
}
