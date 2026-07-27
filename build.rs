fn main() {
    let build_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("cargo:rustc-env=GPUI_REMOTE_BUILD_TIME={build_time}");
}
