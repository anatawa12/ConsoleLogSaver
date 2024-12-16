#![windows_subsystem = "windows"]

use console_log_saver::*;
use libui::controls::{
    Button, Checkbox, Combobox, Group, Label, ProgressBar, ProgressBarValue, SelectionMode, Table,
    TableDataSource, TableModel, TableParameters, TableValue, TableValueType, VerticalBox,
};
use libui::prelude::*;
use std::any::Any;
use std::cell::RefCell;
use std::ops::Deref;
use std::panic::catch_unwind;
use std::rc::Rc;
use std::thread;

fn main() {
    let ui = UI::init().expect("Couldn't initialize UI library");

    let mut win = Window::new(
        &ui.clone(),
        "Console Log Saver",
        600,
        500,
        WindowType::NoMenubar,
    );

    let data = Rc::new(RefCell::new(UnityProcessList::new()));
    let table_model = Rc::new(RefCell::new(TableModel::new(data.clone())));
    let layout = UILayout::new(&ui, table_model.clone());

    data.borrow_mut()
        .reload_unity(&mut table_model.borrow_mut());

    {
        let layout_rc = &layout;
        let mut layout = layout.borrow_mut();

        layout.download_latest_version.on_clicked(|_| {
            open::that("https://github.com/anatawa12/ConsoleLogSaver#readme").ok();
        });

        std::thread::spawn({
            let queue = libui::EventQueueWithData::new(&ui, Rc::downgrade(layout_rc));
            move || {
                let unwind = catch_unwind(|| check_for_update());

                queue.queue_main(move |layout| {
                    let Some(layout) = layout.upgrade() else {
                        return;
                    };
                    let mut layout = layout.borrow_mut();

                    match unwind {
                        Ok(Some((true, latest))) => {
                            layout.this_is_outdated(&latest);
                        }
                        Ok(Some((false, _))) => layout.this_is_latest(),
                        Ok(None) | Err(_) => layout.failed_to_get_latest(),
                    }
                });
            }
        });

        layout.set_messages(Messages::get_by_locale(SupportedLocale::default()));

        layout.refresh_unity_list.on_clicked({
            let data = data.clone();
            let table_model = table_model.clone();
            move |_| {
                data.borrow_mut()
                    .reload_unity(&mut table_model.borrow_mut());
            }
        });

        layout.save_to_file.on_clicked({
            let data = data.clone();
            let layout_weak = Rc::downgrade(layout_rc);
            let ui = ui.clone();
            move |_| {
                let Some(layout) = layout_weak.upgrade() else {
                    return;
                };
                let mut layout = layout.borrow_mut();
                //let Some(path) = win.save_file() else {
                let Some(path) = rfd::FileDialog::new()
                    .set_file_name("log.txt")
                    .add_filter(layout.messages.text_files_star_txt, &["txt"])
                    .save_file()
                else {
                    return;
                };
                let Some(&selecting) = layout.table.selection().get(0) else {
                    return;
                };
                let Some(pid) = data
                    .borrow()
                    .unity_process
                    .get(selecting as usize)
                    .map(|x| x.pid())
                else {
                    return;
                };

                let config = create_config(&layout);

                layout.start_fetch();
                let queue = libui::EventQueueWithData::new(&ui, layout_weak.clone());
                thread::spawn({
                    move || {
                        let unwind = catch_unwind(|| {
                            let result = run_console_log_saver(pid, &config);
                            std::fs::write(path, result.unwrap())
                        });

                        queue.queue_main(|layout| {
                            let Some(layout) = layout.upgrade() else {
                                return;
                            };
                            let mut layout = layout.borrow_mut();

                            match unwind {
                                Ok(Ok(())) => {
                                    let msg = layout.messages.finished;
                                    layout.finish_fetch(msg);
                                }
                                Ok(Err(e)) => {
                                    let msg = format!(
                                        "{}\n{}",
                                        layout.messages.error_getting_log_data, e
                                    );
                                    layout.finish_fetch(&msg);
                                }
                                Err(panic) => {
                                    let message = panic_to_str(panic.deref());
                                    let msg = format!(
                                        "{}\n{}",
                                        layout.messages.error_getting_log_data, message
                                    );
                                    layout.finish_fetch(&msg);
                                }
                            }
                        })
                    }
                });
            }
        });
        layout.copy_to_clipboard.on_clicked({
            let data = data.clone();
            let layout_weak = Rc::downgrade(layout_rc);
            let ui = ui.clone();
            move |_| {
                let Some(layout) = layout_weak.upgrade() else {
                    return;
                };
                let mut layout = layout.borrow_mut();
                let Some(&selecting) = layout.table.selection().get(0) else {
                    return;
                };
                let Some(pid) = data
                    .borrow()
                    .unity_process
                    .get(selecting as usize)
                    .map(|x| x.pid())
                else {
                    return;
                };

                let config = create_config(&layout);

                layout.start_fetch();
                thread::spawn({
                    let queue = libui::EventQueueWithData::new(&ui, layout_weak.clone());
                    move || {
                        let unwind = catch_unwind(|| run_console_log_saver(pid, &config));

                        queue.queue_main(|layout| {
                            let Some(layout) = layout.upgrade() else {
                                return;
                            };
                            let mut layout = layout.borrow_mut();

                            match unwind {
                                Ok(Ok(string)) => {
                                    arboard::Clipboard::new().unwrap().set_text(string).unwrap();
                                    let msg = layout.messages.finished;
                                    layout.finish_fetch(msg);
                                }
                                Ok(Err(e)) => {
                                    let msg = format!(
                                        "{}\n{}",
                                        layout.messages.error_getting_log_data, e
                                    );
                                    layout.finish_fetch(&msg);
                                }
                                Err(panic) => {
                                    let message = panic_to_str(panic.deref());
                                    let msg = format!(
                                        "{}\n{}",
                                        layout.messages.error_getting_log_data, message
                                    );
                                    layout.finish_fetch(&msg);
                                }
                            }
                        })
                    }
                });
            }
        });
    }

    // Actually put the button in the window
    win.set_child(layout.borrow().vbox.clone());

    // Show the window
    win.show();
    // Run the application
    ui.main();
}

fn panic_to_str<'a>(panic: &'a (dyn Any + Send + 'static)) -> &'a str {
    if let Some(s) = panic.downcast_ref::<&str>() {
        s
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s
    } else {
        "Unknown panic"
    }
}

struct UnityProcessList {
    unity_process: Vec<UnityProcess>,
}

impl UnityProcessList {
    fn new() -> UnityProcessList {
        UnityProcessList {
            unity_process: Vec::new(),
        }
    }

    fn reload_unity(&mut self, model: &mut TableModel) {
        let prev_data = std::mem::replace(&mut self.unity_process, find_unity_processes());
        self.unity_process = find_unity_processes();

        let mut prev_pid_iter = prev_data.iter();
        let mut new_iter = self.unity_process.iter();

        let mut index = 0;
        loop {
            match (prev_pid_iter.next(), new_iter.next()) {
                (Some(prev), Some(new)) if prev.pid() == new.pid() => {
                    if prev.project_path() != new.project_path() {
                        model.notify_row_changed(index)
                    }
                    index += 1;
                }

                (Some(_), Some(_)) => {
                    // pid changed, we should consider as added and inserted
                    model.notify_row_deleted(index);
                    model.notify_row_inserted(index);
                    index += 1;
                }

                (Some(_), None) => {
                    model.notify_row_deleted(index);
                }

                (None, Some(_)) => {
                    model.notify_row_inserted(index);
                    index += 1;
                }

                (None, None) => break,
            }
        }

        println!("unity process found: {:?}", &self.unity_process);
    }
}

impl TableDataSource for UnityProcessList {
    fn num_columns(&mut self) -> i32 {
        2
    }

    fn num_rows(&mut self) -> i32 {
        self.unity_process.len().try_into().unwrap_or(i32::MAX)
    }

    fn column_type(&mut self, column: i32) -> TableValueType {
        match column {
            0 => TableValueType::String,
            1 => TableValueType::String,
            _ => unreachable!(),
        }
    }

    fn cell(&mut self, column: i32, row: i32) -> TableValue {
        let row = &self.unity_process[row as usize];
        match column {
            0 => TableValue::String(row.pid().to_string()),
            1 => TableValue::String(format!(
                "{} ({})",
                row.project_path().file_name().unwrap().to_string_lossy(),
                row.project_path().to_string_lossy()
            )),
            _ => unreachable!(),
        }
    }

    fn set_cell(&mut self, _: i32, _: i32, _: TableValue) {
        // unsupported
    }
}

fn create_config(layout: &UILayout) -> ConsoleLogSaverConfig {
    let mut config = ConsoleLogSaverConfig::default();
    config.hide_os_info = layout.hide_os_info.checked();
    config.hide_user_name = layout.hide_user_name.checked();
    config.hide_user_home = layout.hide_user_home_path.checked();
    config.hide_aws_upload_signature = layout.hide_aws_upload_signature.checked();
    config
}

enum VersionInfo {
    Fetching,
    Latest,
    Outdated(String),
    Error,
}

struct UILayout {
    language: Combobox,
    table: Table,
    refresh_unity_list: Button,
    version_info: Label,
    download_latest_version: Button,
    security_settings_group: Group,
    unity_version_required: Checkbox,
    hide_os_info: Checkbox,
    hide_user_name: Checkbox,
    hide_user_home_path: Checkbox,
    hide_aws_upload_signature: Checkbox,
    save_to_file: Button,
    copy_to_clipboard: Button,
    vbox: VerticalBox,
    progress_txt: Label,
    progress_bar: ProgressBar,
    messages: &'static Messages,
    version_info_state: VersionInfo,
}

impl UILayout {
    fn new(ui: &UI, table_model: Rc<RefCell<TableModel>>) -> Rc<RefCell<Self>> {
        let m = Messages::en();

        let result = Rc::<RefCell<Self>>::new({
            let mut vbox = VerticalBox::new();
            vbox.set_padded(true);

            let language = Combobox::new();
            vbox.append(language.clone(), LayoutStrategy::Compact);

            let parameters = TableParameters::new(table_model);
            let mut table = Table::new(parameters);
            table.append_text_column(m.pid, 0, Table::COLUMN_READONLY);
            table.append_text_column(m.project_name_project_path, 1, Table::COLUMN_READONLY);
            table.set_column_width(1, 1000);
            table.set_selection_mode(SelectionMode::ZeroOrOne);
            vbox.append(table.clone(), LayoutStrategy::Stretchy);

            let refresh_unity_list = Button::new("");
            vbox.append(refresh_unity_list.clone(), LayoutStrategy::Compact);

            let version_info = Label::new("");
            vbox.append(version_info.clone(), LayoutStrategy::Compact);

            let download_latest_version = Button::new("");
            vbox.append(download_latest_version.clone(), LayoutStrategy::Compact);

            let mut security_settings_box = VerticalBox::new();
            let mut security_settings_group = Group::new("");

            let mut unity_version_required = Checkbox::new("");
            unity_version_required.disable();
            security_settings_box.append(unity_version_required.clone(), LayoutStrategy::Compact);

            let hide_os_info = Checkbox::new("");
            security_settings_box.append(hide_os_info.clone(), LayoutStrategy::Compact);

            let hide_user_name = Checkbox::new("");
            security_settings_box.append(hide_user_name.clone(), LayoutStrategy::Compact);

            let hide_user_home_path = Checkbox::new("");
            security_settings_box.append(hide_user_home_path.clone(), LayoutStrategy::Compact);

            let hide_aws_upload_signature = Checkbox::new("");
            security_settings_box
                .append(hide_aws_upload_signature.clone(), LayoutStrategy::Compact);

            security_settings_group.set_child(security_settings_box);
            vbox.append(security_settings_group.clone(), LayoutStrategy::Compact);

            let progress_txt = Label::new("");
            vbox.append(progress_txt.clone(), LayoutStrategy::Compact);

            let mut progress_bar = ProgressBar::new();
            progress_bar.hide();
            progress_bar.set_value(ProgressBarValue::Indeterminate);
            vbox.append(progress_bar.clone(), LayoutStrategy::Compact);

            let save_to_file = Button::new("");
            vbox.append(save_to_file.clone(), LayoutStrategy::Compact);

            let copy_to_clipboard = Button::new("");
            vbox.append(copy_to_clipboard.clone(), LayoutStrategy::Compact);

            RefCell::new(UILayout {
                language,
                table,
                refresh_unity_list,
                version_info,
                download_latest_version,
                security_settings_group,
                unity_version_required,
                hide_os_info,
                hide_user_name,
                hide_user_home_path,
                hide_aws_upload_signature,
                save_to_file,
                copy_to_clipboard,
                vbox,
                progress_txt,
                progress_bar,
                version_info_state: VersionInfo::Fetching,
                messages: Messages::en(),
            })
        });

        // add event handlers
        {
            let mut layout = result.borrow_mut();

            for &x in SupportedLocale::values() {
                layout
                    .language
                    .append(Messages::get_by_locale(x).locale_name);
            }
            layout.language.set_selected(0);
            layout.language.on_selected(ui, {
                let weak = Rc::downgrade(&result);
                move |idx| {
                    let messages = Messages::get_by_locale(SupportedLocale::values()[idx as usize]);
                    if let Some(layout) = weak.upgrade() {
                        layout.borrow_mut().set_messages(messages)
                    }
                }
            });

            layout.table.on_selection_changed({
                let weak = Rc::downgrade(&result);
                move |table| {
                    if let Some(layout) = weak.upgrade() {
                        let mut layout = layout.borrow_mut();
                        let selection = table.selection();
                        if selection.len() == 1 {
                            layout.save_to_file.enable();
                            layout.copy_to_clipboard.enable();
                        } else {
                            layout.save_to_file.disable();
                            layout.copy_to_clipboard.disable();
                        }
                    }
                }
            });

            layout.save_to_file.disable();
            layout.copy_to_clipboard.disable();

            // set default values
            let default_config = ConsoleLogSaverConfig::default();
            layout.hide_os_info.set_checked(default_config.hide_os_info);
            layout
                .hide_user_name
                .set_checked(default_config.hide_user_name);
            layout
                .hide_user_home_path
                .set_checked(default_config.hide_user_home);
            layout
                .hide_aws_upload_signature
                .set_checked(default_config.hide_aws_upload_signature);

            layout.set_messages(Messages::en());
        }

        result
    }

    fn set_messages(&mut self, m: &'static Messages) {
        self.messages = m;
        self.reset_messages();
    }

    fn reset_messages(&mut self) {
        let m = self.messages;
        self.refresh_unity_list.set_text(m.refresh_unity_list);

        match &self.version_info_state {
            VersionInfo::Fetching => {
                self.version_info.set_text(
                    &m.version_checking_for_updates
                        .replace("{0}", CURRENT_VERSION),
                );
            }
            VersionInfo::Latest => {
                self.version_info
                    .set_text(&m.version_it_is_latest.replace("{0}", CURRENT_VERSION));
            }
            VersionInfo::Outdated(latest_version) => {
                self.version_info.set_text(
                    &m.version_found_new_version
                        .replace("{0}", CURRENT_VERSION)
                        .replace("{1}", latest_version),
                );
            }
            VersionInfo::Error => {
                self.version_info.set_text(
                    &m.version_failed_to_fetch_latest_version
                        .replace("{0}", CURRENT_VERSION),
                );
            }
        }
        self.download_latest_version
            .set_text(m.download_latest_version);

        self.security_settings_group
            .set_title(self.messages.security_settings);
        self.unity_version_required
            .set_text(m.unity_version_required);
        self.hide_os_info.set_text(m.hide_os_info);
        self.hide_user_name.set_text(m.hide_user_name);
        self.hide_user_home_path.set_text(m.hide_user_home_path);
        self.hide_aws_upload_signature
            .set_text(m.hide_aws_upload_signature);
        self.save_to_file.set_text(m.save_to_file);
        self.copy_to_clipboard.set_text(m.copy_to_clipboard);
    }

    fn start_fetch(&mut self) {
        self.progress_txt.set_text(self.messages.fetching_log);
        self.progress_txt.show();
        self.progress_bar.show();
        self.vbox.disable();
    }

    fn finish_fetch(&mut self, message: &str) {
        self.progress_txt.set_text(message);
        self.progress_bar.hide();
        self.vbox.enable();
    }

    fn this_is_outdated(&mut self, latest: &str) {
        self.version_info_state = VersionInfo::Outdated(latest.to_string());
        self.reset_messages();
    }

    fn this_is_latest(&mut self) {
        self.version_info_state = VersionInfo::Latest;
        self.reset_messages();
    }

    fn failed_to_get_latest(&mut self) {
        self.version_info_state = VersionInfo::Error;
        self.reset_messages();
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum SupportedLocale {
    English,
    Japanese,
}

impl SupportedLocale {
    fn values() -> &'static [SupportedLocale] {
        &[SupportedLocale::English, SupportedLocale::Japanese]
    }

    fn default() -> SupportedLocale {
        for locale in sys_locale::get_locales() {
            if locale.starts_with("en") {
                return SupportedLocale::English;
            }
            if locale.starts_with("ja") {
                return SupportedLocale::English;
            }
        }

        SupportedLocale::English
    }
}

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
