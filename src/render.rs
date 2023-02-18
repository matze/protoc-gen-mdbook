use crate::proto;
use askama::Template;

struct Method<'a> {
    name: &'a str,
    call_type: proto::CallType,
    description: &'a str,
    deprecated: bool,
    input_types: Vec<&'a proto::Types<'a>>,
    output_types: Vec<&'a proto::Types<'a>>,
}

struct Service<'a> {
    name: &'a str,
    package: &'a str,
    description: &'a str,
    deprecated: bool,
    methods: Vec<Method<'a>>,
    deprecated_methods: Vec<Method<'a>>,
}

#[derive(Template)]
#[template(path = "template.md")]
pub struct Page<'a> {
    services: Vec<Service<'a>>,
}

/// Descend field message types starting from `ty` recursively and return them.
#[must_use]
fn gather_types<'a>(ty: &'a proto::Types, types: &'a proto::AllTypes) -> Vec<&'a proto::Types<'a>> {
    let mut result = vec![];

    if let proto::Types::Message(ty) = ty {
        for field in &ty.fields {
            if let proto::FieldType::Custom(custom) = &field.ty {
                // We should be able to unwrap here but we get false package names for nested message
                // types, so work around for now.
                if let Some(custom_types) = types.get(custom.name.package) {
                    for custom_type in custom_types {
                        if custom_type.has_name(field.ty.name()) && !result.contains(&custom_type) {
                            result.push(custom_type);
                            result.append(&mut gather_types(custom_type, types));
                        }
                    }
                }
            }
        }
    }

    result
}

impl<'a> Method<'a> {
    fn from(value: proto::Method<'a>, types: &'a proto::AllTypes) -> Self {
        let mut additional = gather_types(value.input_type, types);
        let mut input_types = vec![value.input_type];
        input_types.append(&mut additional);

        let mut additional = gather_types(value.output_type, types);
        let mut output_types = vec![value.output_type];
        output_types.append(&mut additional);

        Self {
            name: value.name,
            call_type: value.call_type,
            deprecated: value.deprecated,
            description: value.description,
            input_types,
            output_types,
        }
    }
}

impl<'a> Service<'a> {
    fn from(value: proto::Service<'a>, types: &'a proto::AllTypes) -> Self {
        let (deprecated_methods, methods) = value
            .methods
            .into_iter()
            .map(|m| Method::from(m, types))
            .partition(|m| m.deprecated);

        Self {
            name: value.name,
            package: value.package,
            description: value.description,
            deprecated: value.deprecated,
            methods,
            deprecated_methods,
        }
    }
}

impl<'a> Page<'a> {
    pub fn from(services: Vec<proto::Service<'a>>, types: &'a proto::AllTypes) -> Self {
        let services = services
            .into_iter()
            .map(|s| Service::from(s, types))
            .collect();

        Self { services }
    }
}

mod filters {
    /// Split lines in `s` and prepend each line with `//` and join back.
    #[allow(clippy::unnecessary_wraps)]
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
