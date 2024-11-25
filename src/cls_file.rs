pub struct ClsFileBuilder {
    building: String,
    separator: String,
}

impl ClsFileBuilder {
    pub fn new() -> ClsHeadingBuilder {
        let mut building = String::new();
        building.push_str("ConsoleLogSaverData/1.0\n");
        let separator = format!("================{}================", uuid::Uuid::new_v4().simple());
        building.push_str("Separator: ");
        building.push_str(&separator);
        building.push_str("\n");
        ClsHeadingBuilder {
            builder: ClsFileBuilder {
                building,
                separator,
            },
        }
    }

    fn add_header(&mut self, name: &str, value: &str) {
        if name.eq_ignore_ascii_case("separator") {
            panic!("reserved header name: separator")
        }

        check_header_name(name);
        check_header_value(name);

        self.building.push_str(name);
        self.building.push_str(": ");
        self.building.push_str(value);
        self.building.push_str("\n");
    }

    fn end_of_heading(&mut self) {
        self.building.push_str("\n");
    }

    fn end_of_section(&mut self) {
        self.building.push_str(self.separator.as_str());
        self.building.push_str("\n");
    }
}

pub struct ClsHeadingBuilder {
    builder: ClsFileBuilder,
}

impl ClsHeadingBuilder {
    pub fn add_header(mut self, name: &str, value: &str) -> ClsHeadingBuilder {
        self.builder.add_header(name, value);
        self
    }

    pub fn begin_body(mut self) -> ClsBodyBuilder {
        self.builder.end_of_heading();
        self.builder.end_of_section();
        ClsBodyBuilder {
            builder: self.builder,
            has_content: false,
        }
    }
}

pub struct ClsBodyBuilder {
    builder: ClsFileBuilder,
    has_content: bool,
}

impl ClsBodyBuilder {
    pub fn add_header(mut self, name: &str, value: &str) -> ClsBodyBuilder {
        if name.eq_ignore_ascii_case("content") {
            panic!("reserved header name: content")
        }

        self.has_content = true;
        self.builder.add_header(name, value);

        self
    }

    pub fn add_content(mut self, content_type: &str, content: &str) -> ClsBodyBuilder {
        self.builder.add_header("Content", content_type);
        self.builder.end_of_heading();
        self.builder.building.push_str(content);
        self.builder.end_of_section();
        self.has_content = false;
        self
    }

    pub fn build(mut self) -> String {
        if self.has_content {
            self.builder.end_of_heading();
            self.builder.end_of_section();
        }
        self.builder.building
    }
}

fn check_header_name(name: &str) {
    if name.len() == 0 {
        panic!("header name is empty")
    }
    if !name.bytes().all(|c| matches!(c, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'!' | b'#'
        | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
    )) {
        panic!("header name contains invalid characters")
    }
}

fn check_header_value(value: &str) {
    if value.contains('\r') || value.contains('\n') {
        panic!("header value contains newline")
    }
}
