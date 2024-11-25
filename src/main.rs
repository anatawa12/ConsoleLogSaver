mod cls_file;
mod process_remote;

use crate::cls_file::{ClsFileBuilder, ClsHeadingBuilder};
use byteorder::{NativeEndian, ReadBytesExt};
use serde::Deserialize;
use std::env::args;

fn main() {
    let mut args = args();

    let unity_pid = args
        .nth(1)
        .expect("please specify pid")
        .parse::<process_remote::ProcessId>()
        .expect("Failed to parse unity pid");

    let buffer = process_remote::get_buffer(unity_pid).expect("Failed to get buffer");

    let mut reader = TransferDataReader::new(buffer);

    let version: i32 = reader.read_i32();
    if version == 1 {
        let mut cls_file_builder = ClsFileBuilder::new();
        cls_file_builder.add_header(
            "Vendor",
            concat!(
                "ConsoleLogSaver/",
                env!("CARGO_PKG_VERSION"),
                " (CLS-LLDB-RS)"
            ),
        );

        let unity_version = reader.read_string();
        cls_file_builder.add_header("Unity-Version", &unity_version);

        let os_description = reader.read_string();
        cls_file_builder.add_header("Editor-Platform", &os_description);

        let build_target = reader.read_string();
        cls_file_builder.add_header("Build-Target", &build_target);

        let current_directory = reader.read_string();

        append_upm(&mut cls_file_builder, &current_directory);
        append_vpm(&mut cls_file_builder, &current_directory);

        let mut cls_file_builder = cls_file_builder.begin_body();

        let length: i32 = reader.read_i32();
        for _ in 0..length {
            let log_message = reader.read_string();
            let mode = reader.read_i32();
            cls_file_builder.add_header("Mode", &format!("{mode}")); // TODO: transfer to name
            cls_file_builder.add_header("Mode-Raw", &format!("{mode:08x}"));
            cls_file_builder.add_content("log-element", &log_message);
        }

        print!("{}", cls_file_builder.build());
    } else {
        eprintln!("version mismatch ({version})");
    }
}

struct TransferDataReader {
    reader: std::io::Cursor<Vec<u8>>,
}

impl TransferDataReader {
    fn new(data: Vec<u8>) -> Self {
        Self {
            reader: std::io::Cursor::new(data),
        }
    }

    fn read_i32(&mut self) -> i32 {
        self.reader.read_i32::<NativeEndian>().unwrap()
    }

    fn read_string(&mut self) -> String {
        let char_length = self.read_i32();
        let mut buffer = vec![0u16; char_length as usize];
        self.reader
            .read_u16_into::<NativeEndian>(buffer.as_mut_slice())
            .unwrap();
        String::from_utf16(&buffer).expect("bad utf16 message")
    }
}

fn append_upm(builder: &mut ClsHeadingBuilder, cwd: &str) {
    #[derive(Deserialize)]
    struct PackageLock {
        dependencies: std::collections::BTreeMap<String, UpmLockedDependency>,
    }
    #[derive(Deserialize)]
    struct UpmLockedDependency {
        version: Option<String>,
    }

    let package_lock = std::path::Path::new(cwd).join("Packages/packages-lock.json");
    let Ok(package_lock) = std::fs::read(&package_lock) else {
        return;
    };
    let Ok(package_lock) = serde_json::from_slice::<PackageLock>(&package_lock) else {
        return;
    };
    for (dependency, lock_info) in package_lock.dependencies {
        if let Some(version) = lock_info.version {
            builder.add_header("Upm-Dependency", &format!("{dependency}@{version}"));
        }
    }
}

fn append_vpm(builder: &mut ClsHeadingBuilder, cwd: &str) {
    #[derive(Deserialize)]
    struct PackageLock {
        locked: std::collections::BTreeMap<String, VpmLockedDependency>,
    }
    #[derive(Deserialize)]
    struct VpmLockedDependency {
        version: Option<String>,
    }

    let package_lock = std::path::Path::new(cwd).join("Packages/vpm-manifest.json");
    let Ok(package_lock) = std::fs::read(&package_lock) else {
        return;
    };
    let Ok(package_lock) = serde_json::from_slice::<PackageLock>(&package_lock) else {
        return;
    };
    for (dependency, lock_info) in package_lock.locked {
        if let Some(version) = lock_info.version {
            builder.add_header("Vpm-Dependency", &format!("{dependency}@{version}"));
        }
    }
}
