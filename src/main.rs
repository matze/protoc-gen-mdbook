use anyhow::{anyhow, Result};
use askama::Template;
use prost::Message;
use prost_types::compiler::code_generator_response::{Feature, File};
use prost_types::compiler::{CodeGeneratorRequest, CodeGeneratorResponse};
use prost_types::field_descriptor_proto as fdp;
use prost_types::{
    FieldDescriptorProto, FileDescriptorProto, MethodDescriptorProto, ServiceDescriptorProto,
    SourceCodeInfo,
};
use std::convert::From;
use std::fmt::Display;
use std::io::{Read, Write};

/// What the generator should generate. If the options is not set it will generate multiple pages.
enum Mode {
    /// Single page with the file name carried inside the enum value.
    SinglePage(String),
    /// Multiple pages, names are derived from the proto file descriptors by replacing slashes with
    /// dots and appending .md.
    MultiPage,
}

impl From<&CodeGeneratorRequest> for Mode {
    fn from(request: &CodeGeneratorRequest) -> Self {
        if let Some(name) = &request.parameter {
            Mode::SinglePage(name.to_string())
        } else {
            Mode::MultiPage
        }
    }
}

enum CallType {
    Unary,
    ServerStreaming,
    ClientStreaming,
    BidiStreaming,
}

impl From<&MethodDescriptorProto> for CallType {
    fn from(method: &MethodDescriptorProto) -> Self {
        match (method.server_streaming(), method.client_streaming()) {
            (true, true) => CallType::BidiStreaming,
            (true, false) => CallType::ServerStreaming,
            (false, true) => CallType::ClientStreaming,
            (false, false) => CallType::Unary,
        }
    }
}

impl Display for CallType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            CallType::Unary => write!(f, "unary"),
            CallType::ServerStreaming => write!(f, "server streaming"),
            CallType::ClientStreaming => write!(f, "client streaming"),
            CallType::BidiStreaming => write!(f, "bidi streaming"),
        }
    }
}

struct Field<'a> {
    name: &'a str,
    type_name: &'a str,
    number: i32,
    optional: bool,
    leading_comments: &'a str,
    trailing_comments: &'a str,
}

fn scalar_type_name(typ: fdp::Type) -> &'static str {
    match typ {
        fdp::Type::Double => "double",
        fdp::Type::Float => "float",
        fdp::Type::Int64 => "int64",
        fdp::Type::Uint64 => "uint64",
        fdp::Type::Int32 => "int32",
        fdp::Type::Fixed32 => "fixed32",
        fdp::Type::Fixed64 => "fixed64",
        fdp::Type::Bool => "bool",
        fdp::Type::String => "string",
        fdp::Type::Group => "group",
        fdp::Type::Message => "",
        fdp::Type::Bytes => "bytes",
        fdp::Type::Uint32 => "uint32",
        fdp::Type::Enum => "enum",
        fdp::Type::Sfixed32 => "sfixed32",
        fdp::Type::Sfixed64 => "sfixed64",
        fdp::Type::Sint32 => "sint32",
        fdp::Type::Sint64 => "sint64",
    }
}

/// Strip `package` name from `maybe_qualified` in case they match.
fn strip_qualified_package_name<'a, 'b>(maybe_qualified: &'a str, package: &'b str) -> &'a str {
    // Fully qualified package names start with a leading dot, so ignore that for the match.
    if maybe_qualified[1..].starts_with(package) {
        // Remove the package name as well as the leading and the final dot.
        &maybe_qualified[package.len() + 2..]
    } else {
        maybe_qualified
    }
}

mod filters {
    /// Split lines in `s` and prepend each line with `//` and join back.
    pub fn render_multiline_comment<T: std::fmt::Display>(s: T) -> askama::Result<String> {
        Ok(s.to_string()
            .lines()
            .map(|s| {
                let mut s = s.to_string();
                s.insert_str(0, "//");
                s
            })
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

impl<'a> Field<'a> {
    /// Construct field.
    fn from(
        field: &'a FieldDescriptorProto,
        package: &str,
        info: &'a SourceCodeInfo,
        path: &[i32],
    ) -> Self {
        let type_name = if field.type_name.is_some() {
            strip_qualified_package_name(field.type_name(), package)
        } else {
            scalar_type_name(field.r#type())
        };

        let location = info.location.iter().find(|l| l.path == *path);
        let leading_comments = location.map_or_else(|| "", |l| l.leading_comments());
        let trailing_comments = location.map_or_else(|| "", |l| l.trailing_comments());

        Self {
            name: field.name(),
            type_name,
            number: field.number(),
            optional: field.proto3_optional(),
            leading_comments,
            trailing_comments,
        }
    }
}

struct MessageType<'a> {
    name: &'a str,
    description: &'a str,
    fields: Vec<Field<'a>>,
}

impl<'a> MessageType<'a> {
    /// Construct message type matching `name` or a sensible default if it cannot be found.
    fn from(proto: &'a FileDescriptorProto, name: &'a str, info: &'a SourceCodeInfo) -> Self {
        proto
            .message_type
            .iter()
            .enumerate()
            .find_map(|(idx, m)| {
                name.ends_with(m.name()).then(|| {
                    let idx = idx as i32;
                    let description = get_description(info, &[4, idx]);

                    let mut fields = m
                        .field
                        .iter()
                        .enumerate()
                        .map(|(i, f)| Field::from(f, proto.package(), info, &[4, idx, 2, i as i32]))
                        .collect::<Vec<_>>();

                    fields.sort_by(|a, b| a.number.cmp(&b.number));

                    Self {
                        name: m.name(),
                        description,
                        fields,
                    }
                })
            })
            .unwrap_or(Self {
                name,
                description: "",
                fields: vec![],
            })
    }
}

struct Method<'a> {
    name: &'a str,
    call_type: CallType,
    description: &'a str,
    deprecated: bool,
    input_type: MessageType<'a>,
    output_type: MessageType<'a>,
}

impl<'a> Method<'a> {
    fn from(
        proto: &'a FileDescriptorProto,
        method: &'a MethodDescriptorProto,
        path: &mut Vec<i32>,
        idx: i32,
        info: &'a SourceCodeInfo,
    ) -> Self {
        path.push(idx);
        let description = get_description(info, path);
        path.pop();

        let input_type = MessageType::from(proto, method.input_type(), info);
        let output_type = MessageType::from(proto, method.output_type(), info);

        let deprecated = method
            .options
            .as_ref()
            .and_then(|opt| opt.deprecated)
            .unwrap_or(false);

        Self {
            name: method.name(),
            call_type: method.into(),
            description,
            deprecated,
            input_type,
            output_type,
        }
    }
}

struct Service<'a> {
    name: &'a str,
    package: &'a str,
    description: &'a str,
    deprecated: bool,
    methods: Vec<Method<'a>>,
}

#[derive(Template)]
#[template(path = "template.md")]
struct Page<'a> {
    services: Vec<Service<'a>>,
}

/// Get leading comments for the given `path` or empty string if not found matching.
fn get_description<'a>(info: &'a SourceCodeInfo, path: &[i32]) -> &'a str {
    info.location
        .iter()
        .find(|l| l.path == *path)
        .map_or_else(|| "", |l| l.leading_comments())
}

impl<'a> Service<'a> {
    fn from(
        proto: &'a FileDescriptorProto,
        service: &'a ServiceDescriptorProto,
        idx: usize,
        info: &'a SourceCodeInfo,
    ) -> Self {
        let mut path = vec![6, idx as i32];

        let deprecated = service
            .options
            .as_ref()
            .and_then(|opt| opt.deprecated)
            .unwrap_or(false);

        path.push(2);

        let methods = service
            .method
            .iter()
            .enumerate()
            .map(|(idx, method)| Method::from(proto, method, &mut path, idx as i32, info))
            .collect::<Vec<_>>();

        path.pop();

        Self {
            name: service.name(),
            package: proto.package(),
            description: get_description(info, &path),
            deprecated,
            methods,
        }
    }
}

/// Construct all `Service`s of file descriptor `name` in `request`.
fn get_services<'a>(request: &'a CodeGeneratorRequest, name: &str) -> Result<Vec<Service<'a>>> {
    let proto = request
        .proto_file
        .iter()
        .find(|p| p.name() == name)
        .ok_or_else(|| anyhow!("{name} not found"))?;

    let info = proto
        .source_code_info
        .as_ref()
        .ok_or_else(|| anyhow!("no source code info"))?;

    let services = proto
        .service
        .iter()
        .enumerate()
        .map(|(idx, service)| Service::from(proto, service, idx, info))
        .collect::<Vec<_>>();

    Ok(services)
}

fn main() -> Result<()> {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf)?;

    let request = CodeGeneratorRequest::decode(&*buf)?;

    let file = match Mode::from(&request) {
        Mode::SinglePage(name) => {
            let mut content = String::new();

            for name in &request.file_to_generate {
                let services = get_services(&request, name)?;
                content.push_str(&Page { services }.render()?);
            }

            vec![File {
                name: Some(name),
                content: Some(content),
                ..Default::default()
            }]
        }
        Mode::MultiPage => request
            .file_to_generate
            .iter()
            .map(|name| {
                let services = get_services(&request, name)?;
                let content = Some(Page { services }.render()?);

                Ok(File {
                    name: Some(format!("{}.md", name.replace('/', "."))),
                    content,
                    ..Default::default()
                })
            })
            .collect::<Result<Vec<_>, anyhow::Error>>()?,
    };

    let response = CodeGeneratorResponse {
        error: None,
        supported_features: Some(Feature::Proto3Optional as u64),
        file,
    };

    buf.clear();
    response.encode(&mut buf)?;
    std::io::stdout().write_all(&buf)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::filters::render_multiline_comment;
    use super::strip_qualified_package_name as strip;

    #[test]
    fn strip_package_name() {
        assert_eq!(strip(".foo.bar.Baz", "foo.bar"), "Baz");
        assert_eq!(strip(".foo.qux.Baz", "foo.bar"), ".foo.qux.Baz");
        assert_eq!(strip("Baz", "foo.bar"), "Baz");
    }

    #[test]
    fn render_multiline_comments() {
        assert_eq!(
            render_multiline_comment("foo\nbar").unwrap(),
            "//foo\n//bar"
        );
    }
}
