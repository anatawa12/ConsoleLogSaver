fn main() {
    println!("cargo:rerun-if-env-changed=CLS_MONO_PATH");
    if let Ok(cls_path) = std::env::var("CLS_MONO_PATH") {
        if !cls_path.is_empty() {
            println!("cargo:rustc-link-search=native={cls_path}");
        }
    }
}
