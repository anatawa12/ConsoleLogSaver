use eframe::egui;
use eframe::egui::Widget;
use egui_extras::Column;
use console_log_saver::*;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Console Log Saver",
        options,
        Box::new(|_| {
            Ok(Box::new(ConsoleLogSaverGui::new()))
        }),
    )
}

struct ConsoleLogSaverGui {
    unity_process: Vec<UnityProcess>,
    selected_pid: Option<ProcessId>,
    config: ConsoleLogSaverConfig,
    cls_thread: Option<std::thread::JoinHandle<()>>,
    to_copy: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

impl ConsoleLogSaverGui {
    fn new() -> Self {
        let mut result = Self {
            unity_process: Vec::new(),
            config: Default::default(),
            cls_thread: None,
            selected_pid: None,
            to_copy: std::sync::Arc::new(std::sync::Mutex::new(None)),
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
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.cls_thread.is_some() {
                ui.disable()
            }

            ui.heading("Console Log Saver");
            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .max_scroll_height(200.0)
                .column(Column::auto())
                .column(Column::remainder())
                .header(15.0, |mut header| {
                    header.col(|ui| {
                        ui.label("PID");
                    });
                    header.col(|ui| {
                        ui.label("Project Name (Project Path)");
                    });
                })
                .body(|body| {
                    body.rows(15.0, self.unity_process.len(), |mut row| {
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

            if ui.button("Refresh Unity").clicked() {
                self.reload_unity();
            }

            // TODO: version info

            ui.label("Security Settings");
            egui::ScrollArea::vertical().id_salt("Security Settings").show(ui, |ui| {
                ui.add_enabled_ui(false, |ui| {
                    let mut unchangeable = true;
                    ui.checkbox(&mut unchangeable, "Unity Version (Required)");
                });
                ui.checkbox(&mut self.config.hide_os_info, "Hide OS Info");
                ui.checkbox(&mut self.config.hide_user_name, "Hide User Name");
                ui.checkbox(&mut self.config.hide_user_home, "Hide User Home Path");
                ui.checkbox(&mut self.config.hide_aws_upload_signature, "Hide AWS Upload Signature");
            });

            ui.add_enabled_ui(self.selected_pid.is_some(), |ui| {
                if ui.button("Save to File").clicked() {
                    let pid = self.selected_pid.unwrap();
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name("logfile.txt")
                        .add_filter("Text Files (*.txt)", &["txt"])
                        .save_file() {
                        let config = self.config.clone();
                        self.cls_thread = Some(std::thread::Builder::new()
                            .spawn(move || {
                                let result = run_console_log_saver(pid, &config);
                                std::fs::write(path, result).expect("TODO: error handling");
                            }).expect("TODO: error handling"));
                    }
                }

                if ui.button("Copy to clipboard").clicked() {
                    let pid = self.selected_pid.unwrap();
                    let config = self.config.clone();
                    let clipboard_arc = self.to_copy.clone();
                    self.cls_thread = Some(std::thread::Builder::new()
                        .spawn(move || {
                            let result = run_console_log_saver(pid, &config);
                            clipboard_arc.lock().unwrap().replace(result);
                        }).expect("TODO: error handling"));
                }
            });
        });

        if self.cls_thread.is_some() {
            let mut always_open = true;
            let rect = ctx.input(|x| x.screen_rect());
            egui::Window::new("Modal Window")
                .title_bar(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .fixed_pos((rect.width() / 2.0, rect.height() / 2.0))
                .open(&mut always_open)
                .show(ctx, |ui| {
                    if let Some(handle) = &self.cls_thread {
                        if handle.is_finished() {
                            ui.vertical_centered(|ui| {
                                ui.label("Finished!");
                                if ui.button("Close").clicked() {
                                    self.cls_thread = None;
                                }
                            });
                        } else {
                            ui.vertical_centered(|ui| {
                                egui::Spinner::new()
                                    .ui(ui);
                                ui.label("Fetching Log...");
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
