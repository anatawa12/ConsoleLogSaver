#![windows_subsystem = "windows"]

use libui::controls::{
    Button, Checkbox, Combobox, Group, Label, ProgressBar, ProgressBarValue, SelectionMode, Table,
    TableDataSource, TableModel, TableParameters, TableValue, TableValueType, VerticalBox,
};
use libui::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::thread;

fn main() {
    let ui = UI::init().expect("Couldn't initialize UI library");

    let mut win = Window::new(
        &ui.clone(),
        "Console Log Saver",
        600,
        400,
        WindowType::NoMenubar,
    );

    let data = Rc::new(RefCell::new(UnityProcessList::new()));
    let table_model = Rc::new(RefCell::new(TableModel::new(data.clone())));
    let layout = UILayout::new(table_model.clone());

    {
        let layout_rc = &layout;
        let mut layout = layout.borrow_mut();

        layout.do_break.on_clicked({
            let layout_weak = Rc::downgrade(layout_rc);
            let ui = ui.clone();
            move |_| {
                let Some(layout) = layout_weak.upgrade() else {
                    return;
                };
                let mut layout = layout.borrow_mut();

                layout.start_fetch();
                thread::spawn({
                    let queue = libui::EventQueueWithData::new(&ui, layout_weak.clone());
                    move || {
                        std::thread::sleep(std::time::Duration::from_secs(3));
                        queue.queue_main(|layout| {
                            let Some(layout) = layout.upgrade() else {
                                return;
                            };
                            let mut layout = layout.borrow_mut();
                            layout.finish_fetch("Finished");
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

struct UnityProcessList {
}

impl UnityProcessList {
    fn new() -> UnityProcessList {
        UnityProcessList {
        }
    }
}

impl TableDataSource for UnityProcessList {
    fn num_columns(&mut self) -> i32 {
        1
    }

    fn num_rows(&mut self) -> i32 {
        0
    }

    fn column_type(&mut self, column: i32) -> TableValueType {
        match column {
            0 => TableValueType::String,
            _ => unreachable!(),
        }
    }

    fn cell(&mut self, _: i32, _: i32) -> TableValue {
        unreachable!()
    }

    fn set_cell(&mut self, _: i32, _: i32, _: TableValue) {
        // unsupported
    }
}

struct UILayout {
    do_break: Button,
    vbox: VerticalBox,
    progress_txt: Label,
    progress_bar: ProgressBar,
}

impl UILayout {
    fn new(table_model: Rc<RefCell<TableModel>>) -> Rc<RefCell<Self>> {
        let result = Rc::<RefCell<Self>>::new({
            let mut vbox = VerticalBox::new();
            vbox.set_padded(true);

            let parameters = TableParameters::new(table_model);
            let mut table = Table::new(parameters);
            table.append_text_column("pid", 0, Table::COLUMN_READONLY);
            table.set_selection_mode(SelectionMode::ZeroOrOne);
            vbox.append(table.clone(), LayoutStrategy::Stretchy);

            let version_info = Label::new("A label");
            vbox.append(version_info.clone(), LayoutStrategy::Compact);

            let download_latest_version = Button::new("The Button");
            vbox.append(download_latest_version.clone(), LayoutStrategy::Compact);

            let mut security_settings_box = VerticalBox::new();
            let mut security_settings_group = Group::new("Group");

            let mut unity_version_required = Checkbox::new("Checkbox");
            unity_version_required.disable();
            security_settings_box.append(unity_version_required.clone(), LayoutStrategy::Compact);

            security_settings_group.set_child(security_settings_box);
            vbox.append(security_settings_group.clone(), LayoutStrategy::Compact);

            let progress_txt = Label::new("");
            vbox.append(progress_txt.clone(), LayoutStrategy::Compact);

            let mut progress_bar = ProgressBar::new();
            progress_bar.hide();
            progress_bar.set_value(ProgressBarValue::Indeterminate);
            vbox.append(progress_bar.clone(), LayoutStrategy::Compact);

            let do_break = Button::new("Do Break");
            vbox.append(do_break.clone(), LayoutStrategy::Compact);

            RefCell::new(UILayout {
                do_break,
                vbox,
                progress_txt,
                progress_bar,
            })
        });

        result
    }

    fn start_fetch(&mut self) {
        self.progress_txt.set_text("1st\n2nd");
        self.progress_txt.show();
        self.progress_bar.show();
    }

    fn finish_fetch(&mut self, message: &str) {
        self.progress_txt.set_text(message);
        self.progress_bar.hide();
    }
}
