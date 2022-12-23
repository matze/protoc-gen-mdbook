use anyhow::Result;
use askama::Template;
use prost::Message;
use prost_types::compiler::code_generator_response::{Feature, File};
use prost_types::compiler::{CodeGeneratorRequest, CodeGeneratorResponse};
use std::io::{Read, Write};

mod proto;
mod render;

/// Generate single page named `name` containing all services from all proto files.
fn generate_single_page(request: &CodeGeneratorRequest, name: &str) -> Result<Vec<File>> {
    let mut content = String::new();
    let types = proto::get_message_types(request);

    for name in &request.file_to_generate {
        let services = proto::get_services(request, name, &types)?;
        content.push_str(&render::Page::from(services, &types).render()?);
    }

    Ok(vec![File {
        name: Some(name.to_string()),
        content: Some(content),
        ..Default::default()
    }])
}

/// Generate pages for each proto file containing all service documentations of that proto file.
fn generate_multiple_pages(request: &CodeGeneratorRequest) -> Result<Vec<File>> {
    let types = proto::get_message_types(request);

    request
        .file_to_generate
        .iter()
        .map(|name| {
            let services = proto::get_services(request, name, &types)?;
            let content = Some(render::Page::from(services, &types).render()?);

            Ok(File {
                name: Some(format!("{}.md", name.replace('/', "."))),
                content,
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, anyhow::Error>>()
}

fn main() -> Result<()> {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf)?;

    let request = CodeGeneratorRequest::decode(&*buf)?;

    let file = if let Some(name) = &request.parameter {
        generate_single_page(&request, name)?
    } else {
        generate_multiple_pages(&request)?
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
