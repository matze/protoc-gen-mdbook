use anyhow::Result;
use askama::Template;
use prost::Message;
use prost_types::compiler::code_generator_response::{Feature, File};
use prost_types::compiler::{CodeGeneratorRequest, CodeGeneratorResponse};
use proto::{get_services, Service};
use std::io::{Read, Write};

mod proto;

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

#[derive(Template)]
#[template(path = "template.md")]
struct Page<'a> {
    services: Vec<Service<'a>>,
}

/// Generate single page named `name` containing all services from all proto files.
fn generate_single_page(request: &CodeGeneratorRequest, name: &str) -> Result<Vec<File>> {
    let mut content = String::new();

    for name in &request.file_to_generate {
        let services = get_services(request, name)?;
        content.push_str(&Page { services }.render()?);
    }

    Ok(vec![File {
        name: Some(name.to_string()),
        content: Some(content),
        ..Default::default()
    }])
}

/// Generate pages for each proto file containing all service documentations of that proto file.
fn generate_multiple_pages(request: &CodeGeneratorRequest) -> Result<Vec<File>> {
    request
        .file_to_generate
        .iter()
        .map(|name| {
            let services = get_services(request, name)?;
            let content = Some(Page { services }.render()?);

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

#[cfg(test)]
mod tests {
    use super::filters::render_multiline_comment;

    #[test]
    fn render_multiline_comments() {
        assert_eq!(
            render_multiline_comment("foo\nbar").unwrap(),
            "//foo\n//bar"
        );
    }
}
