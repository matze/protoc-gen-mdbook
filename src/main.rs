use anyhow::{anyhow, Result};
use prost::Message;
use prost_types::compiler::code_generator_response::File;
use prost_types::compiler::{CodeGeneratorRequest, CodeGeneratorResponse};
use prost_types::source_code_info::Location;
use prost_types::{
    DescriptorProto, FileDescriptorProto, MethodDescriptorProto, ServiceDescriptorProto,
    SourceCodeInfo,
};
use std::io::{Read, Write};

fn get_location<'a>(info: &'a SourceCodeInfo, path: &Vec<i32>) -> Option<&'a Location> {
    info.location.iter().find(|l| l.path == *path)
}

fn format_comments(content: &mut String, path: &mut Vec<i32>, info: &SourceCodeInfo) {
    if let Some(location) = get_location(info, path) {
        if let Some(s) = &location.leading_comments {
            content.push_str(s.trim());
            content.push_str("\n\n");
        }

        if let Some(s) = &location.trailing_comments {
            content.push_str(s.trim());
            content.push_str("\n\n");
        }
    }
}

fn format_method(
    method: &MethodDescriptorProto,
    path: &mut Vec<i32>,
    info: &SourceCodeInfo,
) -> String {
    let mut content = String::new();

    content.push_str(&format!("### `{}()`\n\n", method.name()));

    let call_type = match (method.server_streaming(), method.client_streaming()) {
        (true, true) => "bidi streaming",
        (true, false) => "server streaming",
        (false, true) => "client streaming",
        (false, false) => "unary",
    };

    content.push_str(&format!("<kbd>{call_type}</kbd>"));

    if method
        .options
        .as_ref()
        .and_then(|opt| opt.deprecated)
        .unwrap_or(false)
    {
        content.push_str("&nbsp;<kbd>deprecated</kbd>");
    }

    content.push_str("\n\n");

    format_comments(&mut content, path, info);

    content.push_str(&format!("**Input: `{}`**\n\n", method.input_type()));
    content.push_str(&format!("**Output: `{}`**\n\n", method.output_type()));

    content
}

fn format_service(
    service: &ServiceDescriptorProto,
    path: &mut Vec<i32>,
    info: &SourceCodeInfo,
) -> Result<String> {
    let mut content = String::new();

    content.push_str(&format!("## {}\n\n", service.name()));

    if service
        .options
        .as_ref()
        .and_then(|opt| opt.deprecated)
        .unwrap_or(false)
    {
        content.push_str("<kbd>deprecated</kbd>");
    }

    content.push_str("\n\n");

    format_comments(&mut content, path, info);

    path.push(2);

    for (idx, method) in service.method.iter().enumerate() {
        path.push(idx.try_into()?);
        content.push_str(&format_method(method, path, info));
        path.pop();
    }

    path.pop();

    Ok(content)
}

fn format_message(
    descriptor: &DescriptorProto,
    path: &mut Vec<i32>,
    info: &SourceCodeInfo,
) -> String {
    let mut content = String::new();

    content.push_str(&format!("## {}\n\n", descriptor.name()));

    format_comments(&mut content, path, info);

    content
}

fn format_proto(proto: &FileDescriptorProto) -> Result<String> {
    let mut content = String::new();

    let info = proto.source_code_info.as_ref().unwrap();

    // TODO: add some enum for these ...
    let mut path = vec![6];

    for (idx, service) in proto.service.iter().enumerate() {
        path.push(idx.try_into()?);
        content.push_str(&format_service(service, &mut path, info)?);
        path.pop();
    }

    path.pop();
    path.push(4);

    for (idx, message) in proto.message_type.iter().enumerate() {
        path.push(idx.try_into()?);
        content.push_str(&format_message(message, &mut path, info));
        path.pop();
    }

    Ok(content)
}

fn main() -> Result<()> {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf)?;

    let request = CodeGeneratorRequest::decode(&*buf)?;
    let mut content = String::new();

    for name in &request.file_to_generate {
        let proto = request
            .proto_file
            .iter()
            .find(|p| p.name() == name)
            .ok_or_else(|| anyhow!("{name} not found"))?;

        content.push_str(&format_proto(proto)?);
    }

    let file = File {
        name: Some("proto.md".to_string()),
        insertion_point: None,
        content: Some(content),
        generated_code_info: None,
    };

    let response = CodeGeneratorResponse {
        error: None,
        supported_features: None,
        file: vec![file],
    };

    buf.clear();
    response.encode(&mut buf)?;
    std::io::stdout().write_all(&buf)?;

    Ok(())
}
