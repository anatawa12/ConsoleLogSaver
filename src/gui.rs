#![windows_subsystem = "windows"]

use libui::controls::{
    Button, Checkbox, Group, Label, ProgressBar, ProgressBarValue, SelectionMode, Table,
    TableDataSource, TableModel, TableParameters, TableValue, TableValueType, VerticalBox,
};
use libui::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

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

    let mut do_break = Button::new("Do Break");
    vbox.append(do_break.clone(), LayoutStrategy::Compact);

    do_break.on_clicked({
        let ui = ui.clone();
        let mut progress_txt = progress_txt.clone();
        let mut progress_bar = progress_bar.clone();
        move |_| {
            progress_txt.set_text("1st\n2nd");
            progress_txt.show();
            progress_bar.show();
            std::thread::spawn({
                let queue = libui::EventQueueWithData::new(&ui, (progress_txt.clone(), progress_bar.clone()));
                move || {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    queue.queue_main(|(progress_txt, progress_bar)| {
                        progress_txt.clone().set_text("Finished");
                        progress_bar.clone().hide();
                    })
                }
            });
        }
    });

    // Actually put the button in the window
    win.set_child(vbox.clone());

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
