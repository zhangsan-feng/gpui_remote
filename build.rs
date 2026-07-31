fn main() {
    let build_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("cargo:rustc-env=BUILD_TIME={build_time}");
}
