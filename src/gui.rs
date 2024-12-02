#![windows_subsystem = "windows"]

use console_log_saver::*;
use eframe::egui;
use eframe::egui::{FontData, FontTweak, Widget};
use egui_extras::Column;
use std::result::Result;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Console Log Saver",
        options,
        Box::new(|ctx| {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "Noto-Sans-JP".to_owned(),
                FontData::from_static(include_bytes!("../font/NotoSansJP-Light.ttf")).tweak(
                    FontTweak {
                        //scale: 0.81, // Make smaller
                        ..Default::default()
                    },
                ),
            );

            fonts.families.insert(
                egui::FontFamily::Proportional,
                vec!["Noto-Sans-JP".to_owned()],
            );

            ctx.egui_ctx.set_fonts(fonts);

            Ok(Box::new(ConsoleLogSaverGui::new()))
        }),
    )
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
}

#[derive(Copy, Clone)]
struct Messages {
    locale_name: &'static str,
    console_log_saver: &'static str,
    pid: &'static str,
    project_name_project_path: &'static str,
    refresh_unity_list: &'static str,
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
    close: &'static str,
    fetching_log: &'static str,
    this_may_take_several_tens_of_seconds: &'static str,
}

impl Messages {
    const fn en() -> &'static Messages {
        &const {
            Self {
                locale_name: "English",
                console_log_saver: "Console Log Saver",
                pid: "PID",
                project_name_project_path: "Project Name (Project Path)",
                refresh_unity_list: "Refresh Unity List",
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
                close: "Close",
                fetching_log: "Fetching log...",
                this_may_take_several_tens_of_seconds: "This may take several tens of seconds...",
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
                close: "閉じる",
                fetching_log: "ログを取得中...",
                this_may_take_several_tens_of_seconds: "数十秒かかることがあります...",
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

enum ClsThreadState {
    NoThread,
    Running(std::thread::JoinHandle<Result<(), String>>),
    Succeeded,
    Error(String),
}

enum ClsThreadInfo<'a> {
    Running,
    Succeeded,
    Error(&'a str),
}

impl ClsThreadState {
    fn to_state_mut(&mut self) -> Option<ClsThreadInfo> {
        match self {
            ClsThreadState::NoThread => None,
            ClsThreadState::Running(running) if !running.is_finished() => {
                Some(ClsThreadInfo::Running)
            }
            ClsThreadState::Running(_) => {
                // it's marked running, but is actually finished so get result
                let state = std::mem::replace(self, ClsThreadState::Error("panic getting result".into()));
                let ClsThreadState::Running(running) = state else {
                    unreachable!()
                };
                *self = match running.join() {
                    Ok(Ok(())) => ClsThreadState::Succeeded,
                    Ok(Err(msg)) => ClsThreadState::Error(msg),
                    Err(panic) => {
                        if let Some(s) = panic.downcast_ref::<&str>() {
                            ClsThreadState::Error(s.to_string())
                        } else if let Some(s) = panic.downcast_ref::<String>() {
                            ClsThreadState::Error(s.to_string())
                        } else {
                            ClsThreadState::Error("Unknown panic (non string/str payload)".into())
                        }
                    }
                };
                // run again
                self.to_state_mut()
            }
            ClsThreadState::Succeeded => Some(ClsThreadInfo::Succeeded),
            ClsThreadState::Error(msg) => Some(ClsThreadInfo::Error(msg)),
        }
    }

    pub(crate) fn exists(&self) -> bool {
        !matches!(self, ClsThreadState::NoThread)
    }
}

struct ConsoleLogSaverGui {
    unity_process: Vec<UnityProcess>,
    selected_pid: Option<ProcessId>,
    config: ConsoleLogSaverConfig,
    cls_thread: ClsThreadState,
    to_copy: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    locale: SupportedLocale,
}

impl ConsoleLogSaverGui {
    fn new() -> Self {
        fn language_default() -> SupportedLocale {
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

        let mut result = Self {
            unity_process: Vec::new(),
            config: Default::default(),
            cls_thread: ClsThreadState::NoThread,
            selected_pid: None,
            to_copy: std::sync::Arc::new(std::sync::Mutex::new(None)),
            locale: language_default(),
        };

        result.reload_unity();

        result
    }

    fn reload_unity(&mut self) {
        self.unity_process = find_unity_processes();
        self.selected_pid = None;
    }
}

impl eframe::App for ConsoleLogSaverGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let m = Messages::get_by_locale(self.locale);

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.cls_thread.exists() {
                ui.disable()
            }

            ui.heading(m.console_log_saver);

            egui::ComboBox::from_id_salt("language")
                .selected_text(m.locale_name)
                .show_ui(ui, |ui| {
                    for &x in SupportedLocale::values() {
                        ui.selectable_value(
                            &mut self.locale,
                            x,
                            Messages::get_by_locale(x).locale_name,
                        );
                    }
                });

            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .max_scroll_height(200.0)
                .column(Column::auto())
                .column(Column::remainder())
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.label(m.pid);
                    });
                    header.col(|ui| {
                        ui.label(m.project_name_project_path);
                    });
                })
                .body(|body| {
                    body.rows(20.0, self.unity_process.len(), |mut row| {
                        let x = &self.unity_process[row.index()];
                        row.set_selected(Some(x.pid()) == self.selected_pid);
                        row.col(|ui| {
                            if ui.label(x.pid().to_string()).clicked() {
                                self.selected_pid = Some(x.pid());
                            }
                        });
                        row.col(|ui| {
                            if ui.label(x.project_path().display().to_string()).clicked() {
                                self.selected_pid = Some(x.pid());
                            }
                        });
                    });
                });

            if ui.button(m.refresh_unity_list).clicked() {
                self.reload_unity();
            }

            // TODO: version info

            ui.label(m.security_settings);
            egui::ScrollArea::vertical()
                .id_salt("Security Settings")
                .show(ui, |ui| {
                    ui.add_enabled_ui(false, |ui| {
                        let mut unchangeable = true;
                        ui.checkbox(&mut unchangeable, m.unity_version_required);
                    });
                    ui.checkbox(&mut self.config.hide_os_info, m.hide_os_info);
                    ui.checkbox(&mut self.config.hide_user_name, m.hide_user_name);
                    ui.checkbox(&mut self.config.hide_user_home, m.hide_user_home_path);
                    ui.checkbox(
                        &mut self.config.hide_aws_upload_signature,
                        m.hide_aws_upload_signature,
                    );
                });

            ui.add_enabled_ui(self.selected_pid.is_some(), |ui| {
                if ui.button(m.save_to_file).clicked() {
                    let pid = self.selected_pid.unwrap();
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name("logfile.txt")
                        .add_filter(m.text_files_star_txt, &["txt"])
                        .save_file()
                    {
                        let config = self.config.clone();
                        self.cls_thread = ClsThreadState::Running(
                            std::thread::Builder::new()
                                .spawn(move || {
                                    let log = run_console_log_saver(pid, &config)
                                        .map_err(|x| x.to_string())?;
                                    std::fs::write(path, log)
                                        .map_err(|x| format!("failed to save file: {}", x))?;
                                    Ok(())
                                })
                                .expect("TODO: error handling"),
                        );
                    }
                }

                if ui.button(m.copy_to_clipboard).clicked() {
                    let pid = self.selected_pid.unwrap();
                    let config = self.config.clone();
                    let clipboard_arc = self.to_copy.clone();
                    self.cls_thread = ClsThreadState::Running(
                        std::thread::Builder::new()
                            .spawn(move || {
                                let log = run_console_log_saver(pid, &config)
                                    .map_err(|x| x.to_string())?;
                                clipboard_arc.lock().unwrap().replace(log);
                                Ok(())
                            })
                            .expect("TODO: error handling"),
                    );
                }
            });
        });

        if let Some(info) = self.cls_thread.to_state_mut() {
            let mut always_open = true;
            let rect = ctx.input(|x| x.screen_rect());
            egui::Window::new("")
                .title_bar(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .fixed_pos((rect.width() / 2.0, rect.height() / 2.0))
                .open(&mut always_open)
                .show(ctx, |ui| {
                    match info {
                        ClsThreadInfo::Running => {
                            ui.vertical_centered(|ui| {
                                egui::Spinner::new().ui(ui);
                                ui.label(m.fetching_log);
                                ui.label(m.this_may_take_several_tens_of_seconds);
                            });
                        }
                        ClsThreadInfo::Succeeded => {
                            ui.vertical_centered(|ui| {
                                ui.label(m.finished);
                                if ui.button(m.close).clicked() {
                                    self.cls_thread = ClsThreadState::NoThread;
                                }
                            });
                        }
                        ClsThreadInfo::Error(msg) => {
                            ui.vertical_centered(|ui| {
                                ui.label(m.error_getting_log_data);
                                ui.label(msg);
                                if ui.button(m.close).clicked() {
                                    self.cls_thread = ClsThreadState::NoThread;
                                }
                            });
                        }
                    }
                });
        }

        if let Ok(mut to_clip) = self.to_copy.try_lock() {
            if let Some(to_clip) = to_clip.take() {
                ctx.output_mut(|o| o.copied_text = to_clip);
            }
        }
    }
}
