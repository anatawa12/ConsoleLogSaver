#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_void};
use std::ptr::null_mut;

macro_rules! structs {
    ($($name: ident)*) => {
        $(#[repr(C)] struct $name {_dummy: u8})*
    };
}

structs!(MonoDomain MonoAssemblyName MonoAssembly MonoImage MonoClass MonoClassField MonoMethod MonoObject MonoString MonoMethodDesc);

type mono_bool = i32; // int32_t

#[cfg_attr(windows, link(name = "mono-2.0-bdwgc", kind = "raw-dylib"))]
#[cfg_attr(unix, link(name = "monobdwgc-2.0", kind = "dylib"))]
extern "C" {
    fn mono_domain_get() -> *mut MonoDomain;
    fn mono_assembly_name_new(name: *const c_char) -> *mut MonoAssemblyName;
    fn mono_assembly_loaded(name: *const MonoAssemblyName) -> *mut MonoAssembly;
    fn mono_assembly_get_image(assembly: *mut MonoAssembly) -> *mut MonoImage;
    fn mono_class_from_name(
        image: *mut MonoImage,
        namespace: *const c_char,
        name: *const c_char,
    ) -> *mut MonoClass;
    fn mono_class_get_field_from_name(
        class: *mut MonoClass,
        name: *const c_char,
    ) -> *mut MonoClassField;
    fn mono_object_new(domain: *mut MonoDomain, klass: *mut MonoClass) -> *mut MonoObject;
    fn mono_runtime_object_init(this_obj: *mut MonoObject);
    fn mono_field_get_value(obj: *mut MonoObject, field: *mut MonoClassField, value: *mut c_void);
    fn mono_string_chars(s: *mut MonoString) -> *mut u16;
    fn mono_string_length(s: *mut MonoString) -> c_int;
    fn mono_method_desc_new(
        name: *const c_char,
        include_namespace: mono_bool,
    ) -> *mut MonoMethodDesc;
    fn mono_method_desc_search_in_class(
        desc: *mut MonoMethodDesc,
        klass: *mut MonoClass,
    ) -> *mut MonoMethod;
    fn mono_runtime_invoke(
        method: *mut MonoMethod,
        obj: *mut c_void,
        params: *mut *mut c_void,
        exc: *mut *mut MonoObject,
    ) -> *mut MonoObject;
    fn mono_object_unbox(obj: *mut MonoObject) -> *mut c_void;
}

#[no_mangle]
static mut CONSOLE_LOG_SAVER_SAVED_LOCATION: *mut u8 = null_mut();
/*
// Current result format
struct String {
  i32 length;
  u16 chars[length];
}

struct Entry {
  String message;
}

struct Result {
  u64 byte_length; // excluding this field
  i32 version; // ensure data is not corrupt
  i32 length;
  Entry entries[length];
}
 */

macro_rules! cs {
    ($string: literal) => {
        concat!($string, "\0").as_ptr() as *const c_char
    };
}

struct TransferDataBuilder {
    builder: Vec<u8>,
}

impl TransferDataBuilder {
    fn new() -> Self {
        let mut builder = Vec::new();
        builder.extend_from_slice(&[0u8; 8]); // capacity space
        builder.extend_from_slice(&[0u8; 8]); // length space
        builder.extend_from_slice(&1i32.to_ne_bytes());
        TransferDataBuilder { builder }
    }

    fn write_i32(&mut self, value: i32) {
        self.builder.extend_from_slice(&value.to_ne_bytes());
    }

    fn write_string(&mut self, chars_slicee: &[u16]) {
        self.write_i32(chars_slicee.len() as i32);
        self.builder
            .extend_from_slice(bytemuck::cast_slice(chars_slicee));
    }

    fn build_to_ptr(self) -> *mut u8 {
        let capacity = self.builder.capacity();
        let len = self.builder.len();
        let leaked = self.builder.leak();
        let result_data_length = (len - 8) as u64;
        leaked[0..][..8].copy_from_slice(&(capacity as u64).to_ne_bytes());
        leaked[8..][..8].copy_from_slice(&result_data_length.to_ne_bytes());

        unsafe { leaked.as_mut_ptr().add(8) }
    }

    unsafe fn from_ptr(location: *mut u8) -> Self {
        let vec_start = unsafe { location.sub(8) };
        let capacity_len_buffer = unsafe { std::slice::from_raw_parts(vec_start, 16) };
        let mut capacity_len = [0u64; 2];
        bytemuck::cast_slice_mut(&mut capacity_len).copy_from_slice(capacity_len_buffer);

        let capacity = capacity_len[0] as usize;
        let len = capacity_len[1] as usize;

        let vec = unsafe { Vec::from_raw_parts(vec_start, len, capacity) };
        TransferDataBuilder { builder: vec }
    }
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
        let StartGettingEntries = mono_method_desc_search_in_class(
            mono_method_desc_new(cs!("int:StartGettingEntries()"), 1),
            LogEntriesClass,
        );
        let EndGettingEntries = mono_method_desc_search_in_class(
            mono_method_desc_new(cs!(":EndGettingEntries()"), 1),
            LogEntriesClass,
        );
        let GetEntryInternal = mono_method_desc_search_in_class(
            mono_method_desc_new(cs!(":GetEntryInternal(int,UnityEditor.LogEntry)"), 1),
            LogEntriesClass,
        );

        let logentry = mono_object_new(domain, LogEntryClass);
        mono_runtime_object_init(logentry);

        let count = *(mono_object_unbox(mono_runtime_invoke(
            StartGettingEntries,
            null_mut(),
            null_mut(),
            null_mut(),
        )) as *const i32);
        //
        // int32_t $message_length[count];
        // char16_t *$message_chars[count];
        //
        // void *$message_obj;
        // int $line, $mode;

        // array size will be set to this place later
        let mut data_builder = TransferDataBuilder::new();
        data_builder.write_i32(count);

        for mut index in 0..count {
            let mut message_obj: *mut MonoString = null_mut();
            let mut line: i32 = 0;
            let mut mode: i32 = 0;
            let args: &mut [*mut c_void] =
                &mut [&mut index as *mut _ as *mut _, logentry as *mut _];
            mono_runtime_invoke(GetEntryInternal, null_mut(), args.as_mut_ptr(), null_mut());
            mono_field_get_value(
                logentry,
                LogEntryClass_message,
                &mut message_obj as *mut _ as *mut _,
            );
            mono_field_get_value(logentry, LogEntryClass_line, &mut line as *mut _ as *mut _);
            mono_field_get_value(logentry, LogEntryClass_mode, &mut mode as *mut _ as *mut _);

            let length = mono_string_length(message_obj);
            let chars_ptr = mono_string_chars(message_obj);
            let chars_slice = std::slice::from_raw_parts(chars_ptr, length as usize);
            data_builder.write_string(chars_slice);
            data_builder.write_i32(mode);
        }

        mono_runtime_invoke(EndGettingEntries, null_mut(), null_mut(), null_mut());

        // Note: RustRover would report error for this line but it's false positive
        CONSOLE_LOG_SAVER_SAVED_LOCATION = data_builder.build_to_ptr();
    }
}

#[no_mangle]
extern "C" fn CONSOLE_LOG_SAVER_FREE_MEM() {
    let location = unsafe { CONSOLE_LOG_SAVER_SAVED_LOCATION };
    if !location.is_null() {
        drop(unsafe { TransferDataBuilder::from_ptr(location) });
    }
}
