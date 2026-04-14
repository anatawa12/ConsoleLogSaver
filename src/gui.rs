#![windows_subsystem = "windows"]

use console_log_saver::*;
use std::any::Any;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::ops::Deref;
use std::panic::catch_unwind;
// Re-import the two-parameter Result so it shadows console_log_saver::Result.
use std::result::Result;
use std::sync::{Arc, Mutex};
use std::thread;
use tcl::*;
use tk::cmd::*;
use tk::*;

// ---------------------------------------------------------------------------
// Background-thread → main-thread message types
// ---------------------------------------------------------------------------

enum BgMessage {
    VersionResult(Option<(bool, String)>),
    FetchSaveDone(Result<(), String>),
    FetchCopyDone(Result<String, String>),
}

// ---------------------------------------------------------------------------
// Version-state
// ---------------------------------------------------------------------------

enum VersionInfo {
    Fetching,
    Latest,
    Outdated(String),
    Error,
}

// ---------------------------------------------------------------------------
// Locale support
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Eq, PartialEq)]
enum SupportedLocale {
    English,
    Japanese,
}

impl SupportedLocale {
    fn values() -> &'static [SupportedLocale] {
        &[SupportedLocale::English, SupportedLocale::Japanese]
    }

    fn detect() -> SupportedLocale {
        // On Unix-like systems, check standard locale environment variables.
        // On Windows, this falls back to English since Windows uses a different
        // locale API; Tk itself handles display correctly in either case.
        for var in &["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(val) = std::env::var(var) {
                if val.starts_with("ja") {
                    return SupportedLocale::Japanese;
                }
                if val.starts_with("en") {
                    return SupportedLocale::English;
                }
            }
        }
        SupportedLocale::English
    }
}

// ---------------------------------------------------------------------------
// Localised strings
// ---------------------------------------------------------------------------

#[derive(Copy, Clone)]
struct Messages {
    locale_name: &'static str,
    pid: &'static str,
    project_name_project_path: &'static str,
    refresh_unity_list: &'static str,
    version_checking_for_updates: &'static str,
    version_it_is_latest: &'static str,
    version_failed_to_fetch_latest_version: &'static str,
    version_found_new_version: &'static str,
    download_latest_version: &'static str,
    security_settings: &'static str,
    unity_version_required: &'static str,
    hide_os_info: &'static str,
    hide_user_name: &'static str,
    hide_user_home_path: &'static str,
    hide_aws_upload_signature: &'static str,
    save_to_file: &'static str,
    text_files_star_txt: &'static str,
    copy_to_clipboard: &'static str,
    finished: &'static str,
    error_getting_log_data: &'static str,
    fetching_log: &'static str,
}

impl Messages {
    const fn en() -> &'static Messages {
        &const {
            Self {
                locale_name: "English",
                pid: "PID",
                project_name_project_path: "Project Name (Project Path)",
                refresh_unity_list: "Refresh Unity List",
                version_checking_for_updates: "Version {0}. Checking for updates...",
                version_it_is_latest: "Version {0}. It's Latest.",
                version_failed_to_fetch_latest_version:
                    "Version {0} Failed to fetch latest version.",
                version_found_new_version: "Version {0}. Found new version {1}.",
                download_latest_version: "Download Latest Version",
                security_settings: "Security Settings",
                unity_version_required: "Unity Version (Required)",
                hide_os_info: "Hide OS Info",
                hide_user_name: "Hide User Name",
                hide_user_home_path: "Hide User Home Path",
                hide_aws_upload_signature: "Hide AWS Upload Signature",
                save_to_file: "Save to File",
                text_files_star_txt: "Text Files (*.txt)",
                copy_to_clipboard: "Copy to Clipboard",
                finished: "Finished!",
                error_getting_log_data: "Error getting log data",
                fetching_log: "Fetching log...\nThis may take several tens of seconds...",
            }
        }
    }

    const fn ja() -> &'static Messages {
        &const {
            Self {
                locale_name: "日本語",
                pid: "PID",
                project_name_project_path: "Project名 (Projectの場所)",
                refresh_unity_list: "Unityの一覧を更新する",
                security_settings: "Security Settings",
                unity_version_required: "Unityのバージョン (Required)",
                hide_os_info: "OSの情報を隠す",
                hide_user_name: "ユーザ名を隠す",
                hide_user_home_path: "ユーザホームのパスを隠す",
                hide_aws_upload_signature: "AWS Upload Signatureを隠す",
                save_to_file: "ファイルに保存",
                text_files_star_txt: "テキストファイル (*.txt)",
                copy_to_clipboard: "コピーする",
                finished: "完了!",
                error_getting_log_data: "エラーが発生しました",
                fetching_log: "ログを取得中...\n数十秒かかることがあります...",
                ..*Self::en()
            }
        }
    }

    fn get_by_locale(locale: SupportedLocale) -> &'static Messages {
        match locale {
            SupportedLocale::English => Self::en(),
            SupportedLocale::Japanese => Self::ja(),
        }
    }
}

// Polling interval (ms) for checking background-thread results on the main thread.
const POLL_INTERVAL_MS: u32 = 100;

// ---------------------------------------------------------------------------
// Thread-local state (all on the main thread)
// ---------------------------------------------------------------------------

thread_local! {
    static PROCESSES: RefCell<Vec<UnityProcess>> = RefCell::new(Vec::new());
    static VERSION_STATE: RefCell<VersionInfo> = RefCell::new(VersionInfo::Fetching);
    static CURRENT_MESSAGES: RefCell<&'static Messages> = RefCell::new(Messages::en());
    static MSG_QUEUE: Arc<Mutex<VecDeque<BgMessage>>> =
        Arc::new(Mutex::new(VecDeque::new()));
}

fn get_queue() -> Arc<Mutex<VecDeque<BgMessage>>> {
    MSG_QUEUE.with(|q| q.clone())
}

fn panic_to_str(panic: &(dyn Any + Send + 'static)) -> String {
    if let Some(s) = panic.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else {
        "Unknown panic".to_string()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn version_text(state: &VersionInfo, m: &Messages) -> String {
    match state {
        VersionInfo::Fetching => m
            .version_checking_for_updates
            .replace("{0}", CURRENT_VERSION),
        VersionInfo::Latest => m.version_it_is_latest.replace("{0}", CURRENT_VERSION),
        VersionInfo::Outdated(latest) => m
            .version_found_new_version
            .replace("{0}", CURRENT_VERSION)
            .replace("{1}", latest),
        VersionInfo::Error => m
            .version_failed_to_fetch_latest_version
            .replace("{0}", CURRENT_VERSION),
    }
}

/// Reload the process list and repopulate the treeview.
fn reload_tree(interp: &tcl::Interp, tree_path: &str) {
    let new_procs = find_unity_processes();
    PROCESSES.with(|p| *p.borrow_mut() = new_procs);

    // Delete all existing rows.
    if let Ok(children_obj) = interp.eval((tree_path, "children", "")) {
        let children: Vec<String> = children_obj
            .get_elements()
            .map(|it| it.map(|e| e.to_string()).collect())
            .unwrap_or_default();
        if !children.is_empty() {
            let _ = interp.run((tree_path, "delete", children));
        }
    }

    // Insert each process.
    PROCESSES.with(|p| {
        for proc in p.borrow().iter() {
            let pid_str = proc.pid().to_string();
            let path_str = format!(
                "{} ({})",
                proc.project_path()
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                proc.project_path().to_string_lossy()
            );
            let values = tcl::Obj::from(vec![pid_str, path_str]);
            let _ = interp.run((tree_path, "insert", "", "end", "-values", values));
        }
    });
}

/// Returns the index (into PROCESSES) of the currently selected treeview row.
fn selected_index(interp: &tcl::Interp, tree_path: &str) -> Option<usize> {
    let sel = interp.eval((tree_path, "selection")).ok()?;
    let sel_str = sel.to_string();
    if sel_str.is_empty() {
        return None;
    }
    let children_obj = interp.eval((tree_path, "children", "")).ok()?;
    let children: Vec<String> = children_obj
        .get_elements()
        .map(|it| it.map(|e| e.to_string()).collect())
        .unwrap_or_default();
    children.iter().position(|id| id == &sel_str)
}

/// Read the security-settings config from Tcl variables.
fn read_config(interp: &tcl::Interp) -> ConsoleLogSaverConfig {
    let mut config = ConsoleLogSaverConfig::default();
    config.hide_os_info = interp.get_int("cls_hide_os").map(|v| v != 0).unwrap_or(false);
    config.hide_user_name = interp
        .get_int("cls_hide_user")
        .map(|v| v != 0)
        .unwrap_or(true);
    config.hide_user_home = interp
        .get_int("cls_hide_home")
        .map(|v| v != 0)
        .unwrap_or(true);
    config.hide_aws_upload_signature = interp
        .get_int("cls_hide_aws")
        .map(|v| v != 0)
        .unwrap_or(true);
    config
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> TkResult<()> {
    let tk = make_tk!()?;
    let root = tk.root();

    root.set_wm_title("Console Log Saver")?;
    tk.run(("wm", "geometry", ".", "600x500"))?;
    tk.run(("wm", "minsize", ".", "400", "380"))?;

    // ── Language combobox ──────────────────────────────────────────────────
    let locale_names: Vec<&'static str> = SupportedLocale::values()
        .iter()
        .map(|l| Messages::get_by_locale(*l).locale_name)
        .collect();
    let lang_combo = root
        .add_ttk_combobox(-values(locale_names.as_slice()))?
        .pack(-fill("x") -padx(5) -pady(2))?;
    lang_combo.set_state(TtkState::ReadOnly)?;
    let default_locale = SupportedLocale::detect();
    let default_idx = SupportedLocale::values()
        .iter()
        .position(|&l| l == default_locale)
        .unwrap_or(0) as i32;
    lang_combo.set_current(TtkComboboxIndex::Number(default_idx))?;

    let m = Messages::get_by_locale(default_locale);
    CURRENT_MESSAGES.with(|cm| *cm.borrow_mut() = m);

    // ── Treeview + scrollbar ───────────────────────────────────────────────
    let tree_frame = root
        .add_ttk_frame(())?
        .pack(-fill("both") -expand(1) -padx(5) -pady(2))?;

    let tree = tree_frame
        .add_ttk_treeview(-columns("pid path") -show("headings") -selectmode("browse"))?
        .pack(-fill("both") -expand(1) -side("left"))?;

    let tree_path: &'static str = tree.path();

    tk.run((tree_path, "heading", "pid", "-text", m.pid))?;
    tk.run((tree_path, "heading", "path", "-text", m.project_name_project_path))?;
    tk.run((tree_path, "column", "pid", "-width", "80", "-stretch", "0"))?;
    tk.run((tree_path, "column", "path", "-width", "490", "-stretch", "1"))?;

    let scrl = tree_frame
        .add_ttk_scrollbar(-orient("vertical"))?
        .pack(-side("right") -fill("y"))?;
    let scrl_path: &'static str = scrl.path();
    tk.run((scrl_path, "configure", "-command", format!("{} yview", tree_path)))?;
    tk.run((tree_path, "configure", "-yscrollcommand", format!("{} set", scrl_path)))?;

    // ── Refresh button ─────────────────────────────────────────────────────
    let refresh_btn = root
        .add_ttk_button(-text(m.refresh_unity_list))?
        .pack(-fill("x") -padx(5) -pady(2))?;

    // ── Version info label ─────────────────────────────────────────────────
    let version_txt = version_text(&VersionInfo::Fetching, m);
    let version_lbl = root
        .add_ttk_label(-text(version_txt.as_str()))?
        .pack(-fill("x") -padx(5))?;

    // ── Download button ────────────────────────────────────────────────────
    let dl_btn = root
        .add_ttk_button(-text(m.download_latest_version))?
        .pack(-fill("x") -padx(5) -pady(2))?;

    // ── Security settings labelframe ───────────────────────────────────────
    let sec_frame = root
        .add_ttk_labelframe(-text(m.security_settings))?
        .pack(-fill("x") -padx(5) -pady(2))?;

    let default_config = ConsoleLogSaverConfig::default();
    tk.set("cls_unity_ver", 1i32);
    tk.set("cls_hide_os", if default_config.hide_os_info { 1i32 } else { 0i32 });
    tk.set("cls_hide_user", if default_config.hide_user_name { 1i32 } else { 0i32 });
    tk.set("cls_hide_home", if default_config.hide_user_home { 1i32 } else { 0i32 });
    tk.set(
        "cls_hide_aws",
        if default_config.hide_aws_upload_signature { 1i32 } else { 0i32 },
    );

    let unity_ver_chk = sec_frame
        .add_ttk_checkbutton(-text(m.unity_version_required) -variable("cls_unity_ver"))?
        .pack(-anchor("w") -padx(5))?;
    unity_ver_chk.set_state(TtkState::Disabled)?;

    let hide_os_chk = sec_frame
        .add_ttk_checkbutton(-text(m.hide_os_info) -variable("cls_hide_os"))?
        .pack(-anchor("w") -padx(5))?;
    let hide_user_chk = sec_frame
        .add_ttk_checkbutton(-text(m.hide_user_name) -variable("cls_hide_user"))?
        .pack(-anchor("w") -padx(5))?;
    let hide_home_chk = sec_frame
        .add_ttk_checkbutton(-text(m.hide_user_home_path) -variable("cls_hide_home"))?
        .pack(-anchor("w") -padx(5))?;
    let hide_aws_chk = sec_frame
        .add_ttk_checkbutton(-text(m.hide_aws_upload_signature) -variable("cls_hide_aws"))?
        .pack(-anchor("w") -padx(5))?;

    // ── Progress label (always packed, text changes) ───────────────────────
    let progress_lbl = root
        .add_ttk_label(-text(""))?
        .pack(-fill("x") -padx(5))?;

    // ── Save / Copy buttons ────────────────────────────────────────────────
    let save_btn = root
        .add_ttk_button(-text(m.save_to_file))?
        .pack(-fill("x") -padx(5) -pady(2))?;
    save_btn.set_state(TtkState::Disabled)?;

    let copy_btn = root
        .add_ttk_button(-text(m.copy_to_clipboard))?
        .pack(-fill("x") -padx(5) -pady(2))?;
    copy_btn.set_state(TtkState::Disabled)?;

    // ── Progress bar (hidden until a fetch begins) ─────────────────────────
    // Create but don't pack yet; we'll pack it dynamically before save_btn.
    let progress_bar = root
        .add_ttk_progressbar(-orient("horizontal") -mode("indeterminate"))?;
    let progress_bar_path: &'static str = progress_bar.path();
    let save_btn_path: &'static str = save_btn.path();

    // ── Initial data load ──────────────────────────────────────────────────
    reload_tree(&***tk, tree_path);

    // ======================================================================
    // Event handlers
    // ======================================================================

    // ── Language selection ─────────────────────────────────────────────────
    lang_combo.bind(
        event::virtual_event("ComboboxSelected"),
        tclosure!(tk, move || -> TkResult<()> {
            let idx = lang_combo.current()? as usize;
            let locale =
                SupportedLocale::values()[idx.min(SupportedLocale::values().len() - 1)];
            let new_m = Messages::get_by_locale(locale);
            CURRENT_MESSAGES.with(|cm| *cm.borrow_mut() = new_m);

            let interp = tcl_interp!();

            // Update all localised widget texts.
            VERSION_STATE.with(|vs| {
                let txt = version_text(&vs.borrow(), new_m);
                let _ = version_lbl.configure(-text(txt.as_str()));
            });
            let _ = refresh_btn.configure(-text(new_m.refresh_unity_list));
            let _ = dl_btn.configure(-text(new_m.download_latest_version));
            let _ = interp.run((sec_frame.path(), "configure", "-text", new_m.security_settings));
            let _ = unity_ver_chk.configure(-text(new_m.unity_version_required));
            let _ = hide_os_chk.configure(-text(new_m.hide_os_info));
            let _ = hide_user_chk.configure(-text(new_m.hide_user_name));
            let _ = hide_home_chk.configure(-text(new_m.hide_user_home_path));
            let _ = hide_aws_chk.configure(-text(new_m.hide_aws_upload_signature));
            let _ = save_btn.configure(-text(new_m.save_to_file));
            let _ = copy_btn.configure(-text(new_m.copy_to_clipboard));
            let _ = interp.run((tree_path, "heading", "pid", "-text", new_m.pid));
            let _ = interp.run((
                tree_path,
                "heading",
                "path",
                "-text",
                new_m.project_name_project_path,
            ));
            Ok(())
        }),
    )?;

    // ── Treeview selection changed ─────────────────────────────────────────
    tree.bind(
        event::virtual_event("TreeviewSelect"),
        tclosure!(tk, move || -> TkResult<()> {
            let interp = tcl_interp!();
            if selected_index(&interp, tree_path).is_some() {
                save_btn.set_state(TtkState::NotDisabled)?;
                copy_btn.set_state(TtkState::NotDisabled)?;
            } else {
                save_btn.set_state(TtkState::Disabled)?;
                copy_btn.set_state(TtkState::Disabled)?;
            }
            Ok(())
        }),
    )?;

    // ── Refresh button ─────────────────────────────────────────────────────
    refresh_btn.configure(-command(tclosure!(tk, move || -> TkResult<()> {
        let interp = tcl_interp!();
        reload_tree(&interp, tree_path);
        save_btn.set_state(TtkState::Disabled)?;
        copy_btn.set_state(TtkState::Disabled)?;
        Ok(())
    })))?;

    // ── Download latest version ────────────────────────────────────────────
    dl_btn.configure(-command(tclosure!(tk, || -> TkResult<()> {
        open::that("https://github.com/anatawa12/ConsoleLogSaver#readme").ok();
        Ok(())
    })))?;

    // ── Save to file ───────────────────────────────────────────────────────
    save_btn.configure(-command(tclosure!(tk, move || -> TkResult<()> {
        let interp = tcl_interp!();

        let Some(idx) = selected_index(&interp, tree_path) else {
            return Ok(());
        };
        let pid = PROCESSES.with(|p| p.borrow().get(idx).map(|pr| pr.pid()));
        let Some(pid) = pid else {
            return Ok(());
        };

        let m = CURRENT_MESSAGES.with(|cm| *cm.borrow());

        // Show file-save dialog (blocks until the user responds).
        // Build the -filetypes list in Tcl format: {{Label .ext} {All Files *}}
        // Each pair of {{ }} produces a literal { } in the format string.
        let filetypes = format!("{{{{{} .txt}} {{All Files *}}}}", m.text_files_star_txt);
        let path_obj = interp.eval((
            "tk_getSaveFile",
            "-defaultextension",
            ".txt",
            "-initialfile",
            "log.txt",
            "-filetypes",
            filetypes.as_str(),
        ))?;
        let path_str = path_obj.to_string();
        if path_str.is_empty() {
            return Ok(()); // user cancelled
        }
        let path = std::path::PathBuf::from(&path_str);

        let config = read_config(&interp);

        // Show progress UI.
        progress_lbl.configure(-text(m.fetching_log))?;
        interp.run((
            "pack",
            progress_bar_path,
            "-fill",
            "x",
            "-padx",
            "5",
            "-before",
            save_btn_path,
        ))?;
        progress_bar.start(TtkProgressbarInterval::default())?;
        refresh_btn.set_state(TtkState::Disabled)?;
        save_btn.set_state(TtkState::Disabled)?;
        copy_btn.set_state(TtkState::Disabled)?;

        let queue = get_queue();
        thread::spawn(move || {
            let result = catch_unwind(|| {
                run_console_log_saver(pid, &config)
                    .map_err(|e| e.to_string())
                    .and_then(|data| {
                        std::fs::write(&path, data).map_err(|e| e.to_string())
                    })
            });
            let msg = match result {
                Ok(Ok(())) => BgMessage::FetchSaveDone(Ok(())),
                Ok(Err(e)) => BgMessage::FetchSaveDone(Err(e)),
                Err(panic) => {
                    BgMessage::FetchSaveDone(Err(panic_to_str(panic.deref())))
                }
            };
            queue.lock().unwrap().push_back(msg);
        });

        Ok(())
    })))?;

    // ── Copy to clipboard ──────────────────────────────────────────────────
    copy_btn.configure(-command(tclosure!(tk, move || -> TkResult<()> {
        let interp = tcl_interp!();

        let Some(idx) = selected_index(&interp, tree_path) else {
            return Ok(());
        };
        let pid = PROCESSES.with(|p| p.borrow().get(idx).map(|pr| pr.pid()));
        let Some(pid) = pid else {
            return Ok(());
        };

        let m = CURRENT_MESSAGES.with(|cm| *cm.borrow());
        let config = read_config(&interp);

        // Show progress UI.
        progress_lbl.configure(-text(m.fetching_log))?;
        interp.run((
            "pack",
            progress_bar_path,
            "-fill",
            "x",
            "-padx",
            "5",
            "-before",
            save_btn_path,
        ))?;
        progress_bar.start(TtkProgressbarInterval::default())?;
        refresh_btn.set_state(TtkState::Disabled)?;
        save_btn.set_state(TtkState::Disabled)?;
        copy_btn.set_state(TtkState::Disabled)?;

        let queue = get_queue();
        thread::spawn(move || {
            let result =
                catch_unwind(|| run_console_log_saver(pid, &config).map_err(|e| e.to_string()));
            let msg = match result {
                Ok(Ok(text)) => BgMessage::FetchCopyDone(Ok(text)),
                Ok(Err(e)) => BgMessage::FetchCopyDone(Err(e)),
                Err(panic) => {
                    BgMessage::FetchCopyDone(Err(panic_to_str(panic.deref())))
                }
            };
            queue.lock().unwrap().push_back(msg);
        });

        Ok(())
    })))?;

    // ======================================================================
    // Background-message polling (runs every 100 ms on the main thread)
    // ======================================================================

    let _ = tclosure!(tk, cmd: "cls_poll_bg", move || -> TkResult<()> {
        let interp = tcl_interp!();

        let msgs: Vec<BgMessage> = {
            let arc = get_queue();
            let mut q = arc.lock().unwrap();
            q.drain(..).collect()
        };

        for msg in msgs {
            let m = CURRENT_MESSAGES.with(|cm| *cm.borrow());
            match msg {
                BgMessage::VersionResult(result) => {
                    VERSION_STATE.with(|vs| {
                        *vs.borrow_mut() = match result {
                            Some((true, ref latest)) => VersionInfo::Outdated(latest.clone()),
                            Some((false, _)) => VersionInfo::Latest,
                            None => VersionInfo::Error,
                        };
                    });
                    let new_text = VERSION_STATE.with(|vs| version_text(&vs.borrow(), m));
                    let _ = version_lbl.configure(-text(new_text.as_str()));
                }

                BgMessage::FetchSaveDone(result) => {
                    progress_bar.stop()?;
                    interp.run(("pack", "forget", progress_bar_path))?;
                    let status_msg = match result {
                        Ok(()) => m.finished.to_string(),
                        Err(e) => format!("{}\n{}", m.error_getting_log_data, e),
                    };
                    progress_lbl.configure(-text(status_msg.as_str()))?;
                    refresh_btn.set_state(TtkState::NotDisabled)?;
                    if selected_index(&interp, tree_path).is_some() {
                        save_btn.set_state(TtkState::NotDisabled)?;
                        copy_btn.set_state(TtkState::NotDisabled)?;
                    }
                }

                BgMessage::FetchCopyDone(result) => {
                    progress_bar.stop()?;
                    interp.run(("pack", "forget", progress_bar_path))?;
                    let status_msg = match &result {
                        Ok(_) => m.finished.to_string(),
                        Err(e) => format!("{}\n{}", m.error_getting_log_data, e),
                    };
                    if let Ok(text) = result {
                        let text_str = text.as_str();
                        interp.run(("clipboard", "clear"))?;
                        interp.run(("clipboard", "append", text_str))?;
                    }
                    progress_lbl.configure(-text(status_msg.as_str()))?;
                    refresh_btn.set_state(TtkState::NotDisabled)?;
                    if selected_index(&interp, tree_path).is_some() {
                        save_btn.set_state(TtkState::NotDisabled)?;
                        copy_btn.set_state(TtkState::NotDisabled)?;
                    }
                }
            }
        }

        interp.after(POLL_INTERVAL_MS.try_into().unwrap(), ("cls_poll_bg",))?;
        Ok(())
    });

    tk.after(POLL_INTERVAL_MS.try_into().unwrap(), ("cls_poll_bg",))?;

    // ── Check for updates in a background thread ───────────────────────────
    {
        let queue = get_queue();
        thread::spawn(move || {
            let result = catch_unwind(check_for_update);
            let opt = match result {
                Ok(val) => val,
                Err(_) => None,
            };
            queue.lock().unwrap().push_back(BgMessage::VersionResult(opt));
        });
    }

    Ok(main_loop())
}
