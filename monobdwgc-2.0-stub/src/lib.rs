#![allow(dead_code)]
#![allow(unused)]
#![allow(non_camel_case_types)]

use std::ffi::*;
use std::mem::zeroed;

macro_rules! structs {
    ($($name: ident)*) => {
        $(#[repr(C)] #[allow(dead_code)] pub struct $name;)*
    };
}

structs!(MonoDomain MonoAssemblyName MonoAssembly MonoImage MonoClass MonoClassField MonoMethod MonoObject MonoString MonoMethodDesc MonoProperty);

type mono_bool = i32; // int32_t

#[no_mangle]
pub extern "C" fn mono_domain_get() -> *mut MonoDomain {
    unsafe { zeroed() }
}
#[no_mangle]
pub extern "C" fn mono_assembly_name_new(name: *const c_char) -> *mut MonoAssemblyName {
    unsafe { zeroed() }
}
#[no_mangle]
pub extern "C" fn mono_assembly_loaded(name: *const MonoAssemblyName) -> *mut MonoAssembly {
    unsafe { zeroed() }
}
#[no_mangle]
pub extern "C" fn mono_assembly_get_image(assembly: *mut MonoAssembly) -> *mut MonoImage {
    unsafe { zeroed() }
}
#[no_mangle]
pub extern "C" fn mono_class_from_name(
    image: *mut MonoImage,
    namespace: *const c_char,
    name: *const c_char,
) -> *mut MonoClass {
    unsafe { zeroed() }
}
#[no_mangle]
pub extern "C" fn mono_class_get_field_from_name(
    class: *mut MonoClass,
    name: *const c_char,
) -> *mut MonoClassField {
    unsafe { zeroed() }
}
#[no_mangle]
pub extern "C" fn mono_class_get_property_from_name(
    class: *mut MonoClass,
    name: *const c_char,
) -> *mut MonoProperty {
    unsafe { zeroed() }
}
#[no_mangle]
pub extern "C" fn mono_property_get_get_method(prop: *mut MonoProperty) -> *mut MonoMethod {
    unsafe { zeroed() }
}
#[no_mangle]
pub extern "C" fn mono_property_get_set_method(prop: *mut MonoProperty) -> *mut MonoMethod {
    unsafe { zeroed() }
}
#[no_mangle]
pub extern "C" fn mono_object_new(
    domain: *mut MonoDomain,
    klass: *mut MonoClass,
) -> *mut MonoObject {
    unsafe { zeroed() }
}
#[no_mangle]
pub extern "C" fn mono_object_to_string(
    obj: *mut MonoObject,
    exc: *mut *mut MonoObject,
) -> *mut MonoString {
    unsafe { zeroed() }
}
#[no_mangle]
pub extern "C" fn mono_runtime_object_init(this_obj: *mut MonoObject) {
    unsafe { zeroed() }
}
#[no_mangle]
pub extern "C" fn mono_field_get_value(
    obj: *mut MonoObject,
    field: *mut MonoClassField,
    value: *mut c_void,
) {
    unsafe { zeroed() }
}
#[no_mangle]
pub extern "C" fn mono_string_chars(s: *mut MonoString) -> *mut u16 {
    unsafe { zeroed() }
}
#[no_mangle]
pub extern "C" fn mono_string_length(s: *mut MonoString) -> c_int {
    unsafe { zeroed() }
}
#[no_mangle]
pub extern "C" fn mono_method_desc_new(
    name: *const c_char,
    include_namespace: mono_bool,
) -> *mut MonoMethodDesc {
    unsafe { zeroed() }
}
#[no_mangle]
pub extern "C" fn mono_method_desc_search_in_class(
    desc: *mut MonoMethodDesc,
    klass: *mut MonoClass,
) -> *mut MonoMethod {
    unsafe { zeroed() }
}
#[no_mangle]
pub extern "C" fn mono_runtime_invoke(
    method: *mut MonoMethod,
    obj: *mut c_void,
    params: *mut *mut c_void,
    exc: *mut *mut MonoObject,
) -> *mut MonoObject {
    unsafe { zeroed() }
}
#[no_mangle]
pub extern "C" fn mono_object_unbox(obj: *mut MonoObject) -> *mut c_void {
    unsafe { zeroed() }
}
