#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ui.h>

#ifdef _WIN32
#include <windows.h>
#define SLEEP(ms) Sleep(ms)
int WINAPI WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR lpCmdLine, int nCmdShow) {
    return main(__argc, __argv);
}
#else
#include <unistd.h>
#include <pthread.h>
#define SLEEP(ms) sleep(ms/1000)
#endif

#define MARGINED 1

// Table model handlers
static int table_model_num_columns(uiTableModelHandler *h, uiTableModel *m) {
    return 1;
}

static int table_model_num_rows(uiTableModelHandler *h, uiTableModel *m) {
    return 0; // No rows as per Rust implementation
}

static uiTableValueType table_model_column_type(uiTableModelHandler *h, uiTableModel *m, int column) {
    return uiTableValueTypeString;
}

static uiTableValue* table_model_cell_value(uiTableModelHandler *h, uiTableModel *m, int row, int col) {
    // Should be unreachable as per Rust implementation
    return uiNewTableValueString("");
}

static void table_model_set_cell_value(uiTableModelHandler *h, uiTableModel *m, int row, int col, const uiTableValue *value) {
    // Unsupported as per Rust implementation
}

// Threading helper structure
typedef struct {
    uiLabel* progress_txt;
    uiProgressBar* progress_bar;
} ThreadData;

void back_to_main_thread(void *arg) {
    ThreadData* data = (ThreadData*)arg;

    uiLabelSetText(data->progress_txt, "Finished");
    uiControlHide(uiControl(data->progress_bar));
}

// Thread function
#ifdef _WIN32
DWORD WINAPI do_break_thread(LPVOID arg) {
#else
void* do_break_thread(void* arg) {
#endif
    ThreadData* data = (ThreadData*)arg;

    // Sleep for 3 seconds
    SLEEP(3000);

    uiQueueMain(back_to_main_thread, data); // Wake up the event loop

    return 0;
}

// Button click handler
static void on_do_break_clicked(uiButton *b, void *data) {
    ThreadData* thread_data = (ThreadData*)data;
    
  #if MARGINED
    uiLabelSetText(thread_data->progress_txt, "1st\n2nd");
  #else
    uiLabelSetText(thread_data->progress_txt, "1st 2nd");
  #endif
    uiControlShow(uiControl(thread_data->progress_txt));
    uiControlShow(uiControl(thread_data->progress_bar));

    #ifdef _WIN32
    CreateThread(NULL, 0, do_break_thread, thread_data, 0, NULL);
    #else
    pthread_t thread;
    pthread_create(&thread, NULL, do_break_thread, thread_data);
    pthread_detach(thread);
    #endif
}

// Window close handler
static int on_window_closing(uiWindow *w, void *data) {
    uiQuit();
    return 1;
}

uiWindow *win;
uiBox *vbox;
uiTableModel *table_model;
uiTable *table;
uiLabel *version_info;
uiButton *download_latest_version;
uiBox *security_settings_box;
uiGroup *security_settings_group;
uiCheckbox *unity_version_required;
uiLabel *progress_txt;
uiProgressBar *progress_bar;
uiButton *do_break;

// Main function
int main(int argc, char **argv) {
    ThreadData *thread_data;

    uiInitOptions options = {
            .Size = sizeof(uiInitOptions),
    };
    // Initialize libui
    const char *error = uiInit(&options);
    if (error != NULL) {
        fprintf(stderr, "Error initializing libui: %s\n", error);
        uiFreeInitError(error);
        return 1;
    }

    // Create the table model
    uiTableModelHandler table_model_handler = {
        .NumColumns = table_model_num_columns,
        .NumRows = table_model_num_rows,
        .ColumnType = table_model_column_type,
        .CellValue = table_model_cell_value,
        .SetCellValue = table_model_set_cell_value
    };
    table_model = uiNewTableModel(&table_model_handler);
    //uiTableModelSetClosure(table_model, process_list);

    // Create the main window (600x400 with no menu bar)
    win = uiNewWindow("Console Log Saver", 600, 400, 0);
#if MARGINED
    uiWindowSetMargined(win, 1);
#endif
    
    // Create vertical box
    vbox = uiNewVerticalBox();
    uiBoxSetPadded(vbox, 1);
    
    // Create table
    uiTableParams params = {
        .Model = table_model,
    };
    table = uiNewTable(&params);
    uiTableAppendTextColumn(table, "pid", 0, uiTableModelColumnNeverEditable, NULL);
    uiBoxAppend(vbox, uiControl(table), 1);
    
    // Create version info label
    version_info = uiNewLabel("A label");
    uiBoxAppend(vbox, uiControl(version_info), 0);
    
    // Create download button
    download_latest_version = uiNewButton("The Button");
    uiBoxAppend(vbox, uiControl(download_latest_version), 0);
    
    // Create security settings group
    security_settings_group = uiNewGroup("Group");
    security_settings_box = uiNewVerticalBox();
    uiGroupSetMargined(security_settings_group, 1);
    
    unity_version_required = uiNewCheckbox("Checkbox");
    uiControlDisable(uiControl(unity_version_required));
    uiBoxAppend(security_settings_box, uiControl(unity_version_required), 0);
    
    uiGroupSetChild(security_settings_group, uiControl(security_settings_box));
    uiBoxAppend(vbox, uiControl(security_settings_group), 0);
    
    // Create progress text and progress bar
    progress_txt = uiNewLabel("");
    uiBoxAppend(vbox, uiControl(progress_txt), 0);
    
    progress_bar = uiNewProgressBar();
    uiControlHide(uiControl(progress_bar));
    uiProgressBarSetValue(progress_bar, -1); // Indeterminate
    uiBoxAppend(vbox, uiControl(progress_bar), 0);
    
    // Create Do Break button and set up thread data
    do_break = uiNewButton("Do Break");
    uiBoxAppend(vbox, uiControl(do_break), 0);
    
    thread_data = malloc(sizeof(ThreadData));
    thread_data->progress_txt = progress_txt;
    thread_data->progress_bar = progress_bar;
    
    uiButtonOnClicked(do_break, on_do_break_clicked, thread_data);
    
    // Set the window content
    uiWindowSetChild(win, uiControl(vbox));
    
    // Set up window close handler
    uiWindowOnClosing(win, on_window_closing, NULL);
    
    // Show the window
    uiControlShow(uiControl(win));
    
    // Run the main loop
    uiMain();
    
    // Clean up
    free(thread_data);
    uiFreeTableModel(table_model);
    uiUninit();
    
    return 0;
}
