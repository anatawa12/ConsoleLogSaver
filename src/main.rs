use lldb::{lldb_addr_t, lldb_offset_t, lldb_pid_t, FunctionNameType, SBAttachInfo, SBData, SBDebugger, SBError, SBExpressionOptions, SBFrame, SBTarget, SBValue};
use std::env::args;

fn main() {
    let mut args = args();

    let unity_pid = args.nth(1).expect("please specify pid").parse::<lldb_pid_t>().expect("Failed to parse unity pid");

    SBDebugger::initialize();

    let debugger = SBDebugger::create(false);
    debugger.set_asynchronous(false);

    // reading symbol table took several time
    let target = debugger
        .create_target("", None, None, true)
        .unwrap();

    let attach_info = SBAttachInfo::new_with_pid(unity_pid);

    let process = target.attach(attach_info).unwrap();

    let update = target
        .find_functions("SceneTracker::Update", FunctionNameType::AUTO.bits())
        .iter()
        .next()
        .unwrap()
        .symbol()
        .start_address()
        .unwrap();

    let breakpoint = target.breakpoint_create_by_sbaddress(update);
    breakpoint.set_enabled(true);
    breakpoint.set_oneshot(true);

    process.continue_execution().unwrap();

    let thread = process.threads().nth(0).unwrap();
    let frame = thread.frames().next().unwrap();

    let ctx = LLDBContext::new(&target, &frame);

    ctx.define_function("mono_domain_get", "void *", &[]);
    ctx.define_function("mono_assembly_name_new", "void *", &["const char *"]);
    ctx.define_function("mono_assembly_loaded", "void *", &["void *"]);
    ctx.define_function("mono_assembly_get_image", "void *", &["void *"]);
    ctx.define_function(
        "mono_class_from_name",
        "void *",
        &["void *", "const char *", "const char *"],
    );
    ctx.define_function(
        "mono_class_get_field_from_name",
        "void *",
        &["void *", "const char *"],
    );
    ctx.define_function("mono_object_new", "void *", &["void *", "void *"]);
    ctx.define_function("mono_runtime_object_init", "void", &["void *"]);
    ctx.define_function(
        "mono_field_get_value",
        "void",
        &["void *", "void *", "void *"],
    );
    ctx.define_function("mono_string_chars", "char16_t *", &["void *"]);
    ctx.define_function("mono_string_length", "int", &["void *"]);
    ctx.define_function(
        "mono_method_desc_new",
        "void *",
        &["const char *", "int32_t"],
    );
    ctx.define_function(
        "mono_method_desc_search_in_class",
        "void *",
        &["void *", "void *"],
    );
    ctx.define_function(
        "mono_runtime_invoke",
        "void *",
        &["void *", "void *", "void **", "void **"],
    );
    ctx.define_function("mono_object_unbox", "void *", &["void *"]);

    let domain = ctx.eval("$mono_domain_get()");
    let assembly_name = ctx.eval(r#"$mono_assembly_name_new("UnityEditor")"#);
    let assembly = ctx.eval(&format!(
        "$mono_assembly_loaded({})",
        assembly_name.name().unwrap()
    ));
    let image = ctx.eval(&format!(
        "$mono_assembly_get_image({})",
        assembly.name().unwrap()
    ));

    ctx.eval("void *$exception;").clear();

    let log_entry_class = ctx.load_class(&image, "UnityEditor", "LogEntry");
    let log_entry_class_message = ctx.load_field(&log_entry_class, "message");
    let _log_entry_class_file = ctx.load_field(&log_entry_class, "file");
    let log_entry_class_line = ctx.load_field(&log_entry_class, "line");
    let log_entry_class_mode = ctx.load_field(&log_entry_class, "mode");

    let log_entries_class = ctx.load_class(&image, "UnityEditor", "LogEntries");
    let start_getting_entries_method =
        ctx.load_method(&log_entries_class, "int:StartGettingEntries()");
    let end_getting_entries_method = ctx.load_method(&log_entries_class, ":EndGettingEntries()");
    let get_entry_internal_method = ctx.load_method(
        &log_entries_class,
        ":GetEntryInternal(int,UnityEditor.LogEntry)",
    );

    let logentry = ctx.eval(&format!(
        "$mono_object_new({domain}, {LogEntryClass})",
        domain = domain.name().unwrap(),
        LogEntryClass = log_entry_class.name().unwrap()
    ));
    ctx.eval(&format!(
        "$mono_runtime_object_init({logentry})",
        logentry = logentry.name().unwrap()
    ))
    .clear();

    ctx.eval("void *$message_obj;").clear();
    ctx.eval("char16_t *$chars;").clear();
    ctx.eval("int $line, $mode;").clear();

    let count = ctx.unbox_int(ctx.invoke_method(&start_getting_entries_method, None, &[]));

    for index in 0..count {
        ctx.invoke_method(
            &get_entry_internal_method,
            None,
            &[
                MethodArg::Literal(index as i64),
                MethodArg::Object(&logentry),
            ],
        )
        .clear();

        ctx.eval(&format!(
            "$mono_field_get_value({logentry}, {LogEntryClass_message}, &$message_obj)",
            logentry = logentry.name().unwrap(),
            LogEntryClass_message = log_entry_class_message.name().unwrap()
        ))
        .clear();
        ctx.eval(&format!(
            "$mono_field_get_value({logentry}, {LogEntryClass_line}, &$line)",
            logentry = logentry.name().unwrap(),
            LogEntryClass_line = log_entry_class_line.name().unwrap()
        ))
        .clear();
        ctx.eval(&format!(
            "$mono_field_get_value({logentry}, {LogEntryClass_mode}, &$mode)",
            logentry = logentry.name().unwrap(),
            LogEntryClass_mode = log_entry_class_mode.name().unwrap()
        ))
        .clear();

        let line = ctx.get_string("$message_obj");
        println!("{line}");
        println!();
    }

    ctx.invoke_method(&end_getting_entries_method, None, &[])
        .clear();

    process.detach().unwrap();

    SBDebugger::terminate();
}

struct LLDBContext<'a> {
    target: &'a SBTarget,
    frame: &'a SBFrame,
}

impl<'a> LLDBContext<'a> {
    fn new(target: &'a SBTarget, frame: &'a SBFrame) -> Self {
        Self { target, frame }
    }
}

impl LLDBContext<'_> {
    fn eval(&self, expr: &str) -> SBValue {
        let options = SBExpressionOptions::new();
        let result = self.frame.evaluate_expression(expr, &options);
        // 0x1001 is kNoResult, which is not an error
        // https://github.com/llvm/llvm-project/blob/d6e65a66095cc3c93ea78669bc41d0885780e8ea/lldb/include/lldb/Expression/UserExpression.h#L274
        if result
            .error()
            .map(|x| x.is_failure() && x.error() != 0x1001)
            .unwrap_or(false)
        {
            panic!("{}", result.error().unwrap());
        }
        result
    }

    fn define_function(&self, name: &str, ret: &str, args: &[&str]) {
        let addr = self
            .target
            .find_functions(name, FunctionNameType::AUTO.bits())
            .iter()
            .nth(0)
            .unwrap()
            .symbol()
            .start_address()
            .unwrap()
            .load_address(self.target);
        self.eval(&format!(
            "typedef {ret} (*${name}Type)({args});",
            args = args.join(", ")
        ))
        .clear();
        self.eval(&format!("${name}Type ${name} = (${name}Type){addr}"))
            .clear()
    }

    fn load_class(&self, image: &SBValue, namespace: &str, name: &str) -> SBValue {
        let class = self.eval(&format!(
            r#"$mono_class_from_name({image}, "{namespace}", "{name}")"#,
            image = image.name().unwrap()
        ));

        if class.data().unwrap().get_address(0).unwrap() == 0 {
            panic!("failed to load class {namespace}.{name}");
        }

        class
    }

    fn load_field(&self, class: &SBValue, name: &str) -> SBValue {
        let field = self.eval(&format!(
            "$mono_class_get_field_from_name({class}, \"{name}\")",
            class = class.name().unwrap()
        ));
        if class.data().unwrap().get_address(0).unwrap() == 0 {
            panic!("failed to load field {name}");
        }
        field
    }

    fn load_method(&self, class: &SBValue, desc: &str) -> SBValue {
        let field = self.eval(&format!(
            "$mono_method_desc_search_in_class($mono_method_desc_new(\"{desc}\", 1), {class})",
            class = class.name().unwrap()
        ));
        if class.data().unwrap().get_address(0).unwrap() == 0 {
            panic!("failed to load method {desc}");
        }
        field
    }

    fn invoke_method(
        &self,
        method: &SBValue,
        this: Option<&SBValue>,
        args: &[MethodArg],
    ) -> SBValue {
        let this_arg = this.map(|x| x.name().unwrap()).unwrap_or("NULL");

        let mut arg_values = vec![];
        let mut new_values = vec![];
        for x in args {
            match x {
                MethodArg::Object(x) => {
                    arg_values.push(x.name().unwrap().to_string());
                }
                MethodArg::Primitive(x) => {
                    arg_values.push(format!("&{}", x.name().unwrap()));
                }
                MethodArg::Literal(v) => {
                    let value = self.eval(&format!("{v}"));
                    arg_values.push(format!("&{}", value.name().unwrap()));
                    new_values.push(value);
                }
            }
        }

        let result = self.eval(&format!(
            "$mono_runtime_invoke({method}, {this_arg}, (void *[]){{{args}}}, &$exception)",
            method = method.name().unwrap(),
            this_arg = this_arg,
            args = arg_values.join(", ")
        ));
        let exception = self.eval("$exception");

        for value in new_values {
            value.clear()
        }

        if exception.data().unwrap().get_address(0).unwrap() != 0 {
            panic!("exception thrown");
        }

        result
    }

    fn unbox_int(&self, value: SBValue) -> i32 {
        let value = self.eval(&format!(
            "*((int32_t *)$mono_object_unbox({value}))",
            value = value.name().unwrap()
        ));
        let result = value.get_signed().unwrap() as i32;
        value.clear();
        result
    }

    fn get_string(&self, string_obj: &str) -> String {
        let chars = self.eval(&format!("$mono_string_chars({string_obj})"));
        let str_len_val = self.eval(&format!("$mono_string_length({string_obj})"));
        let str_len = str_len_val.get_signed().unwrap();
        str_len_val.clear();

        let message_value = self.eval(&format!(
            "*((char16_t (*)[{str_len}]){chars})",
            chars = chars.name().unwrap()
        ));
        let data = message_value.data().unwrap();
        let mut buffer = vec![0u16; str_len as usize];
        data.read_raw(0, bytemuck::cast_slice_mut(&mut buffer))
            .unwrap();

        String::from_utf16(buffer.as_slice()).unwrap()
    }
}

enum MethodArg<'a> {
    Object(&'a SBValue),
    #[allow(dead_code)]
    Primitive(&'a SBValue),
    Literal(i64),
}

unsafe trait SBDataExt {
    fn data_ref(&self) -> lldb::sys::SBDataRef;

    fn get_address(&self, offset: lldb_offset_t) -> Result<lldb_addr_t, SBError> {
        unsafe {
            let error = SBError::default();
            let result = lldb::sys::SBDataGetAddress(self.data_ref(), error.raw, offset);
            if error.is_success() {
                Ok(result)
            } else {
                Err(error)
            }
        }
    }

    fn read_raw(&self, offset: lldb_offset_t, buffer: &mut [u8]) -> Result<(), SBError> {
        unsafe {
            let error = SBError::default();
            lldb::sys::SBDataReadRawData(
                self.data_ref(),
                error.raw,
                offset,
                buffer.as_mut_ptr() as *mut _,
                buffer.len(),
            );
            lldb::sys::SBDataGetAddress(self.data_ref(), error.raw, offset);
            if error.is_success() {
                Ok(())
            } else {
                Err(error)
            }
        }
    }
}

unsafe impl SBDataExt for SBData {
    fn data_ref(&self) -> lldb::sys::SBDataRef {
        self.raw
    }
}

unsafe trait SBValueExt {
    fn data_ref(&self) -> lldb::sys::SBValueRef;

    fn get_signed(&self) -> Result<i64, SBError> {
        unsafe {
            let error = SBError::default();
            let result = lldb::sys::SBValueGetValueAsSigned(self.data_ref(), error.raw, 0);
            if error.is_success() {
                Ok(result)
            } else {
                Err(error)
            }
        }
    }
}

unsafe impl SBValueExt for SBValue {
    fn data_ref(&self) -> lldb::sys::SBValueRef {
        self.raw
    }
}
