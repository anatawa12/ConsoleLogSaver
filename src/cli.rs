use console_log_saver::{
    find_unity_processes, run_console_log_saver, ConsoleLogSaverConfig, ProcessId,
};
use std::process::exit;

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
            "--help" | "-h" => print_help(&exe, 0),
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
            eprintln!(
                "WARNING: Multiple Unity Editors found. using {} for {}",
                process.pid(),
                process.project_path().display()
            );
        }
        pid = Some(process.pid());
    }

    let pid = pid.unwrap();
    match run_console_log_saver(pid, &settings) {
        Ok(log) => print!("{log}"),
        Err(err) => eprintln!("failed to run console log: {err}"),
    }
}

fn print_processes() {
    for process in find_unity_processes() {
        eprintln!("{} for {}", process.pid(), process.project_path().display());
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
