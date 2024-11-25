use std::fs;

fn match_libname(name: &str) -> Option<String> {
    if name.starts_with("liblldb") && name.ends_with(".a") {
        return Some(name[3..name.len() - 2].into());
    }
    if name.starts_with("libLLVM") && name.ends_with(".a") {
        return Some(name[3..name.len() - 2].into());
    }
    if name.starts_with("lldb") && name.ends_with(".lib") {
        return Some(name[..name.len() - 4].into());
    }
    if name.starts_with("liblldb") && name.ends_with(".lib") {
        return Some(name[..name.len() - 4].into());
    }
    if name.starts_with("LLVM") && name.ends_with(".lib") {
        return Some(name[..name.len() - 4].into());
    }
    None
}

fn main() {
    println!("cargo:rerun-if-env-changed=LLDB_LIB_DIR");
    if let Ok(llvm_lib_path) = std::env::var("LLDB_LIB_DIR") {
        println!("cargo:rustc-link-search={llvm_lib_path}");

        for x in fs::read_dir(&llvm_lib_path)
            .expect("failed to stat libdir from llvm-config")
            .filter_map(|entry| match_libname(entry.unwrap().file_name().to_str().unwrap()))
        {
            println!("cargo:rustc-link-lib=static={x}");
            println!("cargo::rerun-if-changed={llvm_lib_path}/lib{x}.a");
        }
    }

    let target = std::env::var("TARGET").unwrap();
    if target.contains("apple-darwin") {
        println!("cargo:rustc-link-lib=framework=ApplicationServices");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=dylib=compression");
    } else if target.contains("-windows-") {
        println!("cargo:rustc-link-lib=dylib=Rpcrt4");
    }
}
