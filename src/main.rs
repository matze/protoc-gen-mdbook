use anyhow::Result;
use askama::Template;
use prost::Message;
use prost_types::compiler::code_generator_response::{Feature, File};
use prost_types::compiler::{CodeGeneratorRequest, CodeGeneratorResponse};
use std::io::{Read, Write};

mod proto;
mod render;

#[derive(Default)]
pub struct Options {
    output: Option<String>,
    optimize_for_doxygen: bool,
}

impl Options {
    fn from_request(request: &CodeGeneratorRequest) -> Self {
        let re = regex::Regex::new(r"(output|optimize):([^,]+)").expect("constructing regex");

        request
            .parameter
            .as_ref()
            .map_or_else(Self::default, |opt| {
                let mut result = Self::default();

                for (_, [key, value]) in re.captures_iter(opt).map(|m| m.extract()) {
                    if key == "output" {
                        result.output = Some(value.to_string());
                    } else if key == "optimize" {
                        result.optimize_for_doxygen = value == "doxygen";
                    }
                }

                result
            })
    }
}

/// Generate single page named `name` containing all services from all proto files.
fn generate_single_page(request: &CodeGeneratorRequest, options: &Options) -> Result<Vec<File>> {
    let mut content = String::new();
    let types = proto::get_types(request);

    for name in &request.file_to_generate {
        let services = proto::get_services(request, name, &types)?;
        content.push_str(&render::Page::from(services, &types, options).render()?);
    }

    Ok(vec![File {
        name: options.output.clone(),
        content: Some(content),
        ..Default::default()
    }])
}

/// Generate pages for each proto file containing all service documentations of that proto file.
fn generate_multiple_pages(request: &CodeGeneratorRequest, options: &Options) -> Result<Vec<File>> {
    let types = proto::get_types(request);

    request
        .file_to_generate
        .iter()
        .map(|name| {
            let services = proto::get_services(request, name, &types)?;
            let content = Some(render::Page::from(services, &types, options).render()?);

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
    let options = Options::from_request(&request);

    let file = if options.output.is_some() {
        generate_single_page(&request, &options)?
    } else {
        generate_multiple_pages(&request, &options)?
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
    use super::*;

    #[test]
    fn parse_empty_options() {
        let request = CodeGeneratorRequest {
            parameter: None,
            ..Default::default()
        };

        let options = Options::from_request(&request);
        assert!(options.output.is_none());
        assert!(!options.optimize_for_doxygen);
    }

    #[test]
    fn parse_single_file_options() {
        let request = CodeGeneratorRequest {
            parameter: Some("output:foo.md".to_string()),
            ..Default::default()
        };

        let options = Options::from_request(&request);
        assert!(options.output.is_some());
        assert_eq!(options.output.unwrap(), "foo.md");
        assert!(!options.optimize_for_doxygen);
    }

    #[test]
    fn parse_optimize_for_doxygen() {
        let request = CodeGeneratorRequest {
            parameter: Some("optimize:doxygen".to_string()),
            ..Default::default()
        };

        let options = Options::from_request(&request);
        assert!(options.output.is_none());
        assert!(options.optimize_for_doxygen);
    }

    #[test]
    fn parse_both_options() {
        let request = CodeGeneratorRequest {
            parameter: Some("output:bar.md,optimize:doxygen".to_string()),
            ..Default::default()
        };

        let options = Options::from_request(&request);
        assert!(options.output.is_some());
        assert_eq!(options.output.unwrap(), "bar.md");
        assert!(options.optimize_for_doxygen);
    }
}
