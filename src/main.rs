use anyhow::{anyhow, Result};
use askama::Template;
use prost::Message;
use prost_types::compiler::code_generator_response::{Feature, File};
use prost_types::compiler::{CodeGeneratorRequest, CodeGeneratorResponse};
use prost_types::source_code_info::Location;
use prost_types::{
    FileDescriptorProto, MethodDescriptorProto, ServiceDescriptorProto, SourceCodeInfo,
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

struct Method {
    name: String,
    call_type: CallType,
    description: String,
    deprecated: bool,
    input_type: String,
    output_type: String,
}

impl Method {
    fn from(
        method: &MethodDescriptorProto,
        path: &mut Vec<i32>,
        idx: i32,
        info: &SourceCodeInfo,
    ) -> Self {
        path.push(idx);
        let description = get_description(get_location(info, path));
        path.pop();

        let deprecated = method
            .options
            .as_ref()
            .and_then(|opt| opt.deprecated)
            .unwrap_or(false);

        Self {
            name: method.name().to_string(),
            call_type: method.into(),
            description,
            deprecated,
            input_type: method.input_type().to_string(),
            output_type: method.output_type().to_string(),
        }
    }
}

struct Service {
    name: String,
    description: String,
    deprecated: bool,
    methods: Vec<Method>,
}

#[derive(Template)]
#[template(path = "template.md")]
struct MarkdownTemplate {
    services: Vec<Service>,
}

fn get_location<'a>(info: &'a SourceCodeInfo, path: &[i32]) -> Option<&'a Location> {
    info.location.iter().find(|l| l.path == *path)
}

fn get_description(location: Option<&Location>) -> String {
    location.map_or_else(|| "".to_string(), |l| l.leading_comments().to_string())
}

impl Service {
    fn from(idx: usize, service: &ServiceDescriptorProto, info: &SourceCodeInfo) -> Self {
        let mut path = vec![6, idx as i32];

        let location = get_location(info, &path);

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
            .map(|(idx, method)| Method::from(method, &mut path, idx as i32, info))
            .collect::<Vec<_>>();

        path.pop();

        Self {
            name: service.name().to_string(),
            description: get_description(location),
            deprecated,
            methods,
        }
    }
}

fn format_proto(proto: &FileDescriptorProto) -> Result<String> {
    let info = proto
        .source_code_info
        .as_ref()
        .ok_or_else(|| anyhow!("no source code info"))?;

    let services = proto
        .service
        .iter()
        .enumerate()
        .map(|(idx, service)| Service::from(idx, service, info))
        .collect::<Vec<_>>();

    Ok(MarkdownTemplate { services }.render()?)
}

/// Retrieve descriptor proto `name` from `request.
fn get_proto<'a>(request: &'a CodeGeneratorRequest, name: &str) -> Result<&'a FileDescriptorProto> {
    request
        .proto_file
        .iter()
        .find(|p| p.name() == name)
        .ok_or_else(|| anyhow!("{name} not found"))
}

fn main() -> Result<()> {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf)?;

    let request = CodeGeneratorRequest::decode(&*buf)?;

    let file = match Mode::from(&request) {
        Mode::SinglePage(name) => {
            let mut content = String::new();

            for name in &request.file_to_generate {
                let proto = get_proto(&request, name)?;
                content.push_str(&format_proto(proto)?);
            }

            vec![File {
                name: Some(name),
                insertion_point: None,
                content: Some(content),
                generated_code_info: None,
            }]
        }
        Mode::MultiPage => request
            .file_to_generate
            .iter()
            .map(|name| {
                let proto = get_proto(&request, name)?;
                let content = Some(format_proto(proto)?);

                Ok(File {
                    name: Some(format!("{}.md", name.replace("/", "."))),
                    insertion_point: None,
                    content,
                    generated_code_info: None,
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
