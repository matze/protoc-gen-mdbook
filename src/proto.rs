//! Higher level wrapper types for the *Proto types from proto-types.

use anyhow::{anyhow, Result};
use prost_types::compiler::CodeGeneratorRequest;
use prost_types::field_descriptor_proto as fdp;
use prost_types::{
    DescriptorProto, FieldDescriptorProto, FileDescriptorProto, MethodDescriptorProto,
    ServiceDescriptorProto, SourceCodeInfo,
};
use std::collections::HashMap;

/// Maps from package name to all included message types.
pub type AllTypes<'a> = HashMap<String, Vec<MessageType<'a>>>;

/// Field type found in messages.
#[derive(Debug)]
pub struct Field<'a> {
    pub name: &'a str,
    pub type_name: &'a str,
    pub number: i32,
    pub optional: bool,
    pub leading_comments: &'a str,
    pub trailing_comments: &'a str,
}

/// Message types referenced as inputs and outputs in methods.
#[derive(Debug)]
pub struct MessageType<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub fields: Vec<Field<'a>>,
}

/// Streaming call type of a method.
pub enum CallType {
    Unary,
    ServerStreaming,
    ClientStreaming,
    BidiStreaming,
}

/// Service method type.
pub struct Method<'a> {
    pub name: &'a str,
    pub call_type: CallType,
    pub description: &'a str,
    pub deprecated: bool,
    pub input_type: &'a MessageType<'a>,
    pub output_type: &'a MessageType<'a>,
}

/// gRPC service type.
pub struct Service<'a> {
    pub name: &'a str,
    pub package: &'a str,
    pub description: &'a str,
    pub deprecated: bool,
    pub methods: Vec<Method<'a>>,
}

/// Get proto type name as found in .proto files.
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

/// Return all message types for all compiled protos mapped from their package tree.
pub fn get_message_types(request: &CodeGeneratorRequest) -> AllTypes {
    let mut result: HashMap<String, Vec<MessageType>> = HashMap::new();

    for proto in &request.proto_file {
        let package = proto.package();
        let info = proto.source_code_info.as_ref().unwrap();

        let types = proto
            .message_type
            .iter()
            .enumerate()
            .map(|(idx, mt)| MessageType::from(package, mt, as_i32(idx), info))
            .collect::<Vec<MessageType>>();

        result.insert(package.to_string(), types);
    }

    result
}

/// Construct all `Service`s of file descriptor `name` in `request`.
pub fn get_services<'a>(
    request: &'a CodeGeneratorRequest,
    name: &str,
    types: &'a AllTypes,
) -> Result<Vec<Service<'a>>> {
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
        .map(|(idx, service)| Service::from(proto, service, types, as_i32(idx), info))
        .collect::<Vec<_>>();

    Ok(services)
}

/// Get leading comments for the given `path` or empty string if not found matching.
fn get_description<'a>(info: &'a SourceCodeInfo, path: &[i32]) -> &'a str {
    info.location
        .iter()
        .find(|l| l.path == *path)
        .map_or_else(|| "", |l| l.leading_comments())
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

/// Return package name for fully qualified typename without the leading dot, i.e. `foo.bar` for
/// `.foo.bar.Baz`.
fn extract_package_name(type_name: &str) -> &str {
    let start = type_name.find('.').unwrap_or(0);
    let end = type_name.rfind('.').unwrap_or(type_name.len() - 1);
    &type_name[start + 1..end]
}

/// Helper function to cast from guaranteed 31 bit usize to i32
#[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
fn as_i32(idx: usize) -> i32 {
    idx as i32
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

impl std::fmt::Display for CallType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            CallType::Unary => write!(f, "unary"),
            CallType::ServerStreaming => write!(f, "server streaming"),
            CallType::ClientStreaming => write!(f, "client streaming"),
            CallType::BidiStreaming => write!(f, "bidi streaming"),
        }
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

impl<'a> MessageType<'a> {
    /// Construct message type matching `name` or a sensible default if it cannot be found.
    fn from(
        package: &str,
        message_type: &'a DescriptorProto,
        idx: i32,
        info: &'a SourceCodeInfo,
    ) -> Self {
        let description = get_description(info, &[4, idx]);

        let mut fields = message_type
            .field
            .iter()
            .enumerate()
            .map(|(i, f)| Field::from(f, package, info, &[4, idx, 2, as_i32(i)]))
            .collect::<Vec<_>>();

        fields.sort_by(|a, b| a.number.cmp(&b.number));

        Self {
            name: message_type.name(),
            description,
            fields,
        }
    }
}

impl<'a> Method<'a> {
    fn from(
        method: &'a MethodDescriptorProto,
        types: &'a AllTypes,
        path: &mut Vec<i32>,
        idx: i32,
        info: &'a SourceCodeInfo,
    ) -> Self {
        path.push(idx);
        let description = get_description(info, path);
        path.pop();

        let package = extract_package_name(method.input_type());
        let types = types.get(package).unwrap();

        let input_type_name = strip_qualified_package_name(method.input_type(), package);
        let input_type = types.iter().find(|ty| ty.name == input_type_name).unwrap();

        let output_type_name = strip_qualified_package_name(method.output_type(), package);
        let output_type = types.iter().find(|ty| ty.name == output_type_name).unwrap();

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

impl<'a> Service<'a> {
    fn from(
        proto: &'a FileDescriptorProto,
        service: &'a ServiceDescriptorProto,
        types: &'a AllTypes,
        idx: i32,
        info: &'a SourceCodeInfo,
    ) -> Self {
        let mut path = vec![6, idx];

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
            .map(|(idx, method)| Method::from(method, types, &mut path, as_i32(idx), info))
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

#[cfg(test)]
mod tests {
    use super::extract_package_name as extract;
    use super::strip_qualified_package_name as strip;

    #[test]
    fn strip_package_name() {
        assert_eq!(strip(".foo.bar.Baz", "foo.bar"), "Baz");
        assert_eq!(strip(".foo.qux.Baz", "foo.bar"), ".foo.qux.Baz");
        assert_eq!(strip("Baz", "foo.bar"), "Baz");
    }

    #[test]
    fn extract_package_name() {
        assert_eq!(extract(".foo.bar.Baz"), "foo.bar");
    }
}
