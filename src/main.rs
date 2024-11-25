mod cls_file;
mod process_remote;

use crate::cls_file::{ClsFileBuilder, ClsHeadingBuilder};
use byteorder::{NativeEndian, ReadBytesExt};
use regex::Regex;
use serde::Deserialize;
use std::borrow::Cow;
use std::ops::Deref;
use std::path::Component;
use std::process::exit;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, UpdateKind};
use crate::process_remote::ProcessId;

fn main() {
    let mut settings = ConsoleLogSaverConfig::default();
    let mut pid = None;

    let mut args = std::env::args();
    let exe = args.next().unwrap();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--hide-user-name" => settings.hide_user_name = true,
            "--show-user-name" => settings.hide_user_name = false,
            "--hide-user-home" => settings.hide_user_home = true,
            "--show-user-home" => settings.hide_user_home = false,
            "--hide-os-info" => settings.hide_os_info = true,
            "--show-os-info" => settings.hide_os_info = false,
            "--hide-aws-upload-signature" => settings.hide_aws_upload_signature = true,
            "--show-aws-upload-signature" => settings.hide_aws_upload_signature = false,
            "--list" => {
                print_processes();
                exit(0);
            }
            "--help" | "-h" => {
                print_help(&exe, 0)
            }
            "--pid" => {
                let Some(pid_str) = args.next() else {
                    eprintln!("No opeand found for --pid");
                    exit(1);
                };

                let Some(parsed) = pid_str.parse::<ProcessId>().ok() else {
                    eprintln!("Invalid process id: {pid_str}");
                    exit(1);
                };
                pid = Some(parsed);
            }
            "--port" => {
                eprintln!("console log saver no longer uses mono wire protocol so specifying port is not supported");
                exit(1);
            }
            arg if arg.starts_with("-") => {
                eprintln!("unknown option: {}", arg);
                exit(1);
            }
            pid_str => {
                let Some(parsed) = pid_str.parse::<ProcessId>().ok() else {
                    eprintln!("Invalid process id: {pid_str}");
                    exit(1);
                };
                pid = Some(parsed);
            }
        }
    }

    if pid.is_none() {
        let unity_processes = find_unity_processes();
        if unity_processes.is_empty() {
            eprintln!("No unity processes found");
            exit(1);
        }
        let process = &unity_processes[0];
        if unity_processes.len() > 1 {
            eprintln!("WARNING: Multiple Unity Editors found. using {} for {}", process.pid, process.project_path.display());
        }
        pid = Some(process.pid);
    }

    let pid = pid.unwrap();
    print!("{}", run_console_log_saver(pid, &settings));
}

fn print_processes() {
    for process in find_unity_processes() {
        eprintln!("{} for {}", process.pid, process.project_path.display());
    }
}

pub fn print_help(exe: &str, exit_code: i32) {
    eprintln!("{exe} [OPTIONS] <unity pid>");
    eprintln!("ConsoleLogSaver {}", env!("CARGO_PKG_VERSION"));
    eprintln!("ConsoleLogSaver with lldb native debugger");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("\t--hide-user-name: enable Hide User Name log filter");
    eprintln!("\t--show-user-name: disable Hide User Name log filter");
    eprintln!("\t--hide-user-home: enable Hide User Home log filter");
    eprintln!("\t--show-user-home: disable Hide User Home log filter");
    eprintln!("\t--hide-os-info: enable Hide OS Info flag");
    eprintln!("\t--show-os-info: disable Hide OS Info flag");
    eprintln!("\t--hide-aws-upload-signature: enable Hide AWS Upload Signature flag");
    eprintln!("\t--show-aws-upload-signature: disable Hide AWS Upload Signature flag");
    eprintln!("\t--pid <pid>: specify pid of unity");
    eprintln!("\t--list: list unity processes and exit");
    eprintln!("\t--help: show this message and exit");
    
    exit(exit_code);
}

struct UnityProcess {
    pid: ProcessId,
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
        let Some(project_path) = cmd.get(index + 1) else {
            continue;
        };
        let project_path = std::path::Path::new(project_path);

        unity_processes.push(UnityProcess {
            pid: pid.as_u32() as ProcessId,
            project_path: project_path.to_owned(),
        })
    }

    unity_processes
}

struct ConsoleLogSaverConfig {
    hide_user_name: bool,
    hide_user_home: bool,
    hide_os_info: bool,
    hide_aws_upload_signature: bool,
}

impl Default for ConsoleLogSaverConfig {
    fn default() -> Self {
        Self {
            hide_user_name: true,
            hide_user_home: true,
            hide_os_info: false,
            hide_aws_upload_signature: true,
        }
    }
}

struct ReplaceSet {
    pairs: Vec<(&'static Regex, &'static str)>,
}

impl ReplaceSet {
    fn new(config: &ConsoleLogSaverConfig) -> Self {
        let mut regex_pairs = vec![];

        if config.hide_user_home {
            static REGEX: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                let home = home::home_dir().expect("failed to get home directory");
                let mut regex = String::new();

                let mut last_separator = true;
                for x in home.components() {
                    match x {
                        Component::Prefix(prefix) => {
                            regex.push_str(&regex::escape(&prefix.as_os_str().to_string_lossy()));
                        }
                        Component::RootDir => {
                            regex.push_str(&r#"[/\\]"#);
                            last_separator = true;
                        }
                        Component::Normal(normal) => {
                            if !last_separator {
                                regex.push_str(&r#"[/\\]"#);
                            }
                            regex.push_str(&regex::escape(&normal.to_string_lossy()));
                            last_separator = false;
                        }
                        Component::CurDir => panic!("should not happen"),
                        Component::ParentDir => panic!("should not happen"),
                    }
                }

                eprintln!("{}", regex);

                regex::RegexBuilder::new(&regex)
                    .case_insensitive(true)
                    .build()
                    .expect("failed to create regex")
            });
            regex_pairs.push((REGEX.deref(), "user-home"));
        }

        if config.hide_user_name {
            static REGEX: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                let user_name = whoami::username();
                regex::RegexBuilder::new(&regex::escape(&user_name))
                    .case_insensitive(true)
                    .build()
                    .expect("failed to create regex")
            });
            regex_pairs.push((REGEX.deref(), "user-name"));
        }

        if config.hide_aws_upload_signature {
            static REGEX: std::sync::LazyLock<Regex> =
                std::sync::LazyLock::new(|| Regex::new(r"(?<prefix>Signature=)[^&\s]+").unwrap());
            regex_pairs.push((REGEX.deref(), "signature-param"));
        }

        // always hidden data

        {
            // AWSAccessKeyId
            static REGEX: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                Regex::new(r"(?<prefix>AWSAccessKeyId=)[^&\s]+").unwrap()
            });
            regex_pairs.push((REGEX.deref(), "aws-access-key-id-param"));
        }

        {
            // AWSAccessKeyId
            static REGEX: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                Regex::new(r##"(?<prefix>"assetUrl"\s*:\s*")((?:[^\u0000-\u001F"\\]|\\(?:u[a-fA-F0-9]{4}|[^"\\/bfnrt]))*)(?<suffix>")"##).unwrap()
            });
            regex_pairs.push((REGEX.deref(), "asset-url"));
        }

        Self { pairs: regex_pairs }
    }

    fn replace_all<'a, 'b>(&'a self, input: Cow<'b, str>) -> Cow<'b, str> {
        let mut output = input;

        for (regex, replacement) in &self.pairs {
            let replacer = |captures: &regex::Captures| {
                let mut result = String::new();
                if let Some(prefix) = captures.name("prefix") {
                    result.push_str(prefix.as_str())
                }
                result.push_str("${");
                result.push_str(replacement);
                result.push_str("}");
                if let Some(suffix) = captures.name("suffix") {
                    result.push_str(suffix.as_str())
                }
                result
            };
            match output {
                Cow::Borrowed(borrowed) => {
                    output = regex.replace_all(borrowed, replacer);
                }
                Cow::Owned(owned) => match regex.replace_all(&owned, replacer) {
                    Cow::Borrowed(borrowed) => {
                        debug_assert_eq!(borrowed, &owned);
                        output = Cow::Owned(owned);
                    }
                    Cow::Owned(owned) => {
                        output = Cow::Owned(owned);
                    }
                },
            }
        }

        output
    }
}

fn run_console_log_saver(pid: ProcessId, config: &ConsoleLogSaverConfig) -> String {
    let buffer = process_remote::get_buffer(pid).expect("Failed to get buffer");

    let replacer = ReplaceSet::new(&config);

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

    if !config.hide_os_info {
        let os_description = reader.read_string();
        cls_file_builder.add_header("Editor-Platform", &os_description);
    }

    if config.hide_user_name {
        cls_file_builder.add_header("Hidden-Data", "user-name");
    }

    if config.hide_user_home {
        cls_file_builder.add_header("Hidden-Data", "user-home");
    }

    cls_file_builder.add_header("Hidden-Data", "aws-access-key-id-param");
    cls_file_builder.add_header("Hidden-Data", "asset-url");

    if config.hide_aws_upload_signature {
        cls_file_builder.add_header("Hidden-Data", "signature-param");
    }

    let build_target = reader.read_string();
    cls_file_builder.add_header("Build-Target", &build_target);

    let current_directory = reader.read_string();

    append_upm(&mut cls_file_builder, &current_directory, &replacer);
    append_vpm(&mut cls_file_builder, &current_directory);

    let mut cls_file_builder = cls_file_builder.begin_body();

    let length: i32 = reader.read_i32();
    for _ in 0..length {
        let log_message = reader.read_string();
        let mode = reader.read_i32();
        cls_file_builder.add_header("Mode", &format!("{mode}")); // TODO: transfer to name
        cls_file_builder.add_header("Mode-Raw", &format!("{mode:08x}"));
        cls_file_builder.add_content(
            "log-element",
            &replacer.replace_all(Cow::Borrowed(&log_message)),
        );
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

fn append_upm(builder: &mut ClsHeadingBuilder, cwd: &str, replacer: &ReplaceSet) {
    #[derive(Deserialize)]
    struct PackageLock {
        dependencies: std::collections::BTreeMap<String, UpmLockedDependency>,
    }
    #[derive(Deserialize)]
    struct UpmLockedDependency {
        version: Option<String>,
    }

    enum UpmDependencyType {
        NpmRemote,
        HttpsGit,
        SshGit,
        GitGit,
        FileGit,
        FileRelative,
        FileAbsolute,
    }

    impl UpmDependencyType {
        fn detect_from_version(version: &str) -> Self {
            if version.starts_with("file://")
                || version.contains(".git")
                || version.starts_with("git+")
            {
                // it's some git URLs
                let version = version.strip_prefix("git+").unwrap_or(version);

                if version.starts_with("https:") {
                    return UpmDependencyType::HttpsGit;
                }
                if version.starts_with("ssh:") {
                    return UpmDependencyType::SshGit;
                }
                if version.starts_with("file:") {
                    return UpmDependencyType::FileGit;
                }
                if version.starts_with("git:") {
                    return UpmDependencyType::GitGit;
                }
            }

            if let Some(path) = version.strip_prefix("file:") {
                // it's some file URLs
                let path = std::path::Path::new(path);
                if path.has_root() {
                    return UpmDependencyType::FileAbsolute;
                } else {
                    return UpmDependencyType::FileRelative;
                }
            }

            UpmDependencyType::NpmRemote
        }
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
            let mut version = Cow::Borrowed(version.as_str());
            match UpmDependencyType::detect_from_version(&version) {
                UpmDependencyType::NpmRemote
                | UpmDependencyType::HttpsGit
                | UpmDependencyType::SshGit
                | UpmDependencyType::GitGit => {
                    // Those are remote, so it's very unlikely to include personal information
                }
                UpmDependencyType::FileGit | UpmDependencyType::FileAbsolute => {
                    // file git is mostly absolute path
                    // an absolute path may include user home
                    let replaced = replacer.replace_all(version);
                    version = replaced;
                }
                UpmDependencyType::FileRelative => {
                    // relative path mostly doesn't include user home
                }
            }
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
