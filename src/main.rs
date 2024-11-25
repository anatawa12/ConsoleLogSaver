mod cls_file;
mod process_remote;

use crate::cls_file::{ClsFileBuilder, ClsHeadingBuilder};
use byteorder::{NativeEndian, ReadBytesExt};
use serde::Deserialize;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, UpdateKind};

fn main() {
    let unity_processes = find_unity_processes();

    let unity_pid = unity_processes.first().unwrap().pid;
    print!("{}", run_console_log_saver(unity_pid));
}

struct UnityProcess {
    pid: process_remote::ProcessId,
    project_path: std::path::PathBuf,
}

fn find_unity_processes() -> Vec<UnityProcess> {
    #[cfg(target_os = "macos")]
    let exe_name: &std::path::Path = "Contents/MacOS/Unity".as_ref();
    #[cfg(target_os = "windows")]
    let exe_name: &std::path::Path = "Unity.exe".as_ref();
    #[cfg(target_os = "linux")]
    let exe_name: &std::path::Path = "Unity".as_ref();

    let mut sysinfo = sysinfo::System::new();

    sysinfo.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::new()
            .with_cmd(UpdateKind::Always)
            .with_exe(UpdateKind::Always),
    );
    sysinfo.refresh_processes(ProcessesToUpdate::All, true);

    let mut unity_processes = Vec::new();
    for (pid, proc) in sysinfo.processes() {
        let Some(exe) = proc.exe() else { continue };
        if !exe.ends_with(exe_name) {
            continue;
        }
        let cmd = proc.cmd();
        if cmd.iter().any(|x| x == "-srvPort") {
            continue; // it looks asset importer worker
        }
        let Some(index) = cmd.iter().position(|x| x == "-projectPath") else {
            continue;
        };
        let Some(project_path) = cmd.get(index) else {
            continue;
        };
        let project_path = std::path::Path::new(project_path);

        eprintln!("Process {}", pid);
        eprintln!("Cmd: {:?}", project_path);
        unity_processes.push(UnityProcess {
            pid: pid.as_u32() as process_remote::ProcessId,
            project_path: project_path.to_owned(),
        })
    }

    unity_processes
}

fn run_console_log_saver(pid: process_remote::ProcessId) -> String {
    let buffer = process_remote::get_buffer(pid).expect("Failed to get buffer");

    let mut reader = TransferDataReader::new(buffer);

    let version: i32 = reader.read_i32();
    if version != 1 {
        panic!("version mismatch ({version})");
    }

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

    cls_file_builder.build()
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
