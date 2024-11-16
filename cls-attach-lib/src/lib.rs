use std::ffi::{c_char, c_int, c_void};
use std::ptr::{null, null_mut};

macro_rules! structs {
    ($($name: ident)*) => {
        $(#[repr(C)] struct $name;)*
    };
}

structs!(MonoDomain MonoAssemblyName MonoAssembly MonoImage MonoClass MonoClassField MonoMethod MonoObject MonoString MonoMethodDesc);

type mono_bool = i32; // int32_t

#[link(name = "monobdwgc-2.0", kind = "dylib")]
extern "C" {
    fn mono_domain_get() -> *mut MonoDomain;
    fn mono_assembly_name_new(name: *const c_char) -> *mut MonoAssemblyName;
    fn mono_assembly_loaded(name: *const MonoAssemblyName) -> *mut MonoAssembly;
    fn mono_assembly_get_image(assembly: *mut MonoAssembly) -> *mut MonoImage;
    fn mono_class_from_name(image: *mut MonoImage, namespace: *const c_char, name: *const c_char) -> *mut MonoClass;
    fn mono_class_get_field_from_name(class: *mut MonoClass, name: *const c_char) -> *mut MonoClassField;
    fn mono_object_new(domain: *mut MonoDomain, klass: *mut MonoClass) -> *mut MonoObject;
    fn mono_runtime_object_init(this_obj: *mut MonoObject);
    fn mono_field_get_value(obj: *mut MonoObject, field: *mut MonoClassField, value: *mut c_void);
    fn mono_string_chars(s: *mut MonoString) -> *mut u16;
    fn mono_string_length(s: *mut MonoString) -> c_int;
    fn mono_method_desc_new(name: *const c_char, include_namespace: mono_bool) -> *mut MonoMethodDesc;
    fn mono_method_desc_search_in_class(desc: *mut MonoMethodDesc, klass: *mut MonoClass) -> *mut MonoMethod;
    fn mono_runtime_invoke(method: *mut MonoMethod, obj: *mut c_void, params: *mut *mut c_void, exc: *mut *mut MonoObject) -> *mut MonoObject;
    fn mono_object_unbox(obj: *mut MonoObject) -> *mut c_void;
}

#[no_mangle]
static mut CONSOLE_LOG_SAVER_SAVED_LOCATION: *mut u8 = null_mut();

macro_rules! cs {
    ($string: literal) => {
        concat!($string, "\0").as_ptr() as *const c_char
    };
}

#[no_mangle]
extern "C" fn CONSOLE_LOG_SAVER_SAVE() {
    unsafe {
        let domain = mono_domain_get();
        let assembly_name = mono_assembly_name_new(cs!("UnityEditor"));
        let assembly = mono_assembly_loaded(assembly_name);
        let image = mono_assembly_get_image(assembly);

        let LogEntryClass = mono_class_from_name(image, cs!("UnityEditor"), cs!("LogEntry"));
        let LogEntryClass_message = mono_class_get_field_from_name(LogEntryClass, cs!("message"));
        let LogEntryClass_line = mono_class_get_field_from_name(LogEntryClass, cs!("line"));
        let LogEntryClass_mode = mono_class_get_field_from_name(LogEntryClass, cs!("mode"));

        let LogEntriesClass = mono_class_from_name(image, cs!("UnityEditor"), cs!("LogEntries"));
        let StartGettingEntries = mono_method_desc_search_in_class(mono_method_desc_new(cs!("int:StartGettingEntries()"), 1), LogEntriesClass);
        let EndGettingEntries = mono_method_desc_search_in_class(mono_method_desc_new(cs!(":EndGettingEntries()"), 1), LogEntriesClass);
        let GetEntryInternal = mono_method_desc_search_in_class(mono_method_desc_new(cs!(":GetEntryInternal(int,UnityEditor.LogEntry)"), 1), LogEntriesClass);


        let logentry = mono_object_new(domain, LogEntryClass);
        mono_runtime_object_init(logentry);

        let count = *(mono_object_unbox(mono_runtime_invoke(StartGettingEntries, null_mut(), null_mut(), null_mut())) as *const i32);
        // 
        // int32_t $message_length[count];
        // char16_t *$message_chars[count];
        // 
        // void *$message_obj;
        // int $line, $mode;

        for mut index in 0..count {
            let mut message_obj: *mut MonoString = null_mut();
            let mut line: i32 = 0;
            let mut mode: i32 = 0;
            let args: &mut [*mut c_void] = &mut [
                &mut index as *mut _ as *mut _,
                logentry as *mut _,
            ];
            mono_runtime_invoke(GetEntryInternal, null_mut(), args.as_mut_ptr(), null_mut());
            mono_field_get_value(logentry, LogEntryClass_message, &mut message_obj as *mut _ as *mut _);
            mono_field_get_value(logentry, LogEntryClass_line, &mut line as *mut _ as *mut _);
            mono_field_get_value(logentry, LogEntryClass_mode, &mut mode as *mut _ as *mut _);

            //$message_length[index] = $mono_string_length($message_obj);
            //$message_chars[index]  = $mono_string_chars($message_obj);
        }

        mono_runtime_invoke(EndGettingEntries, null_mut(), null_mut(), null_mut());

        /*
        struct Result {
            int32_t count;
            int32_t *message_length;
            char16_t **message_chars;
        };

        struct Result $result = {
            .count = count,
            .message_length = $message_length,
            .message_chars  = $message_chars,
        };
         */
    }
}

