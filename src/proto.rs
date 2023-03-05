//! Higher level wrapper types for the *Proto types from proto-types.

use crate::render::filters;
use anyhow::{anyhow, Result};
use askama::Template;
use prost_types::compiler::CodeGeneratorRequest;
use prost_types::field_descriptor_proto as fdp;
use prost_types::{
    DescriptorProto, EnumDescriptorProto, EnumValueDescriptorProto, FieldDescriptorProto,
    FileDescriptorProto, MethodDescriptorProto, ServiceDescriptorProto, SourceCodeInfo,
};
use std::collections::HashMap;

/// Wrap message and enum types for generic rendering.
#[derive(PartialEq)]
pub enum Types<'a> {
    Message(MessageType<'a>),
    Enum(EnumType<'a>),
}

impl<'a> Types<'a> {
    /// Return `true` if `name` matches.
    pub fn has_name(&self, name: &str) -> bool {
        match self {
            Types::Message(ty) => ty.name == name,
            Types::Enum(ty) => ty.name == name,
        }
    }
}

/// Maps from package name to all included enum and message types.
pub type AllTypes<'a> = HashMap<String, Vec<Types<'a>>>;

/// A fully qualified type name including package path and leading dot.
#[derive(PartialEq)]
pub struct FullyQualifiedTypeName<'a> {
    #[allow(dead_code)]
    pub original: &'a str,
    /// Package path without the leading dot
    pub package: &'a str,
    /// Just the type name without the package path
    pub name: &'a str,
}

impl<'a> From<&'a str> for FullyQualifiedTypeName<'a> {
    fn from(original: &'a str) -> Self {
        let start = original.find('.').unwrap();
        let end = original.rfind('.').unwrap();

        Self {
            original,
            package: &original[start + 1..end],
            name: &original[end + 1..],
        }
    }
}

impl<'a> From<&'a FieldDescriptorProto> for FullyQualifiedTypeName<'a> {
    fn from(value: &'a FieldDescriptorProto) -> Self {
        Self::from(value.type_name())
    }
}

/// Custom message type consisting of a fully qualified name.
#[derive(PartialEq)]
pub struct CustomType<'a> {
    pub name: FullyQualifiedTypeName<'a>,
}

/// Field type which is either a well-known proto type or a custom message type.
#[derive(PartialEq)]
pub enum FieldType<'a> {
    WellKnown(fdp::Type),
    Custom(CustomType<'a>),
}

impl<'a> FieldType<'a> {
    pub fn name(&self) -> &str {
        match self {
            Self::WellKnown(ty) => scalar_type_name(*ty),
            Self::Custom(ty) => ty.name.name,
        }
    }
}

impl<'a> From<&'a FieldDescriptorProto> for FieldType<'a> {
    fn from(field: &'a FieldDescriptorProto) -> Self {
        if field.type_name.is_some() {
            // unsafe: we do not yet guarantee that the field contains a leading dot.
            FieldType::Custom(CustomType {
                name: FullyQualifiedTypeName::from(field),
            })
        } else {
            FieldType::WellKnown(field.r#type())
        }
    }
}

/// Field type found in messages.
#[derive(PartialEq)]
pub struct Field<'a> {
    pub name: &'a str,
    pub ty: FieldType<'a>,
    pub number: i32,
    pub optional: bool,
    pub repeated: bool,
    pub leading_comments: &'a str,
    pub trailing_comments: &'a str,
}

/// Message types referenced as inputs and outputs in methods.
#[derive(PartialEq, Template)]
#[template(path = "message_type.md", escape = "none")]
pub struct MessageType<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub fields: Vec<Field<'a>>,
    pub nested: Vec<MessageType<'a>>,
    pub depth: usize,
}

/// Enum value types.
#[derive(PartialEq)]
pub struct EnumValue<'a> {
    pub name: &'a str,
    pub number: i32,
    pub leading_comments: &'a str,
    pub trailing_comments: &'a str,
}

/// Enum types.
#[derive(PartialEq)]
pub struct EnumType<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub values: Vec<EnumValue<'a>>,
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
    pub input_type: &'a Types<'a>,
    pub output_type: &'a Types<'a>,
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
fn scalar_type_name(ty: fdp::Type) -> &'static str {
    match ty {
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
pub fn get_types(request: &CodeGeneratorRequest) -> AllTypes {
    let mut result: HashMap<String, Vec<Types>> = HashMap::new();

    for proto in &request.proto_file {
        let package = proto.package();
        let info = proto.source_code_info.as_ref().unwrap();

        let mut message_types = proto
            .message_type
            .iter()
            .enumerate()
            .map(|(idx, ty)| Types::Message(MessageType::from(ty, as_i32(idx), info, 0)))
            .collect::<Vec<Types>>();

        result
            .entry(package.to_string())
            .or_default()
            .append(&mut message_types);

        let mut enum_types = proto
            .enum_type
            .iter()
            .enumerate()
            .map(|(idx, ty)| Types::Enum(EnumType::from(ty, as_i32(idx), info)))
            .collect::<Vec<Types>>();

        result
            .entry(package.to_string())
            .or_default()
            .append(&mut enum_types);
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
        .map_or("", |l| l.leading_comments())
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
    fn from(field: &'a FieldDescriptorProto, info: &'a SourceCodeInfo, path: &[i32]) -> Self {
        let ty = FieldType::from(field);
        let location = info.location.iter().find(|l| l.path == *path);
        let leading_comments = location.map_or("", |l| l.leading_comments());
        let trailing_comments = location.map_or("", |l| l.trailing_comments());
        let repeated = field
            .label
            .map_or(false, |l| l == fdp::Label::Repeated.into());

        Self {
            name: field.name(),
            ty,
            number: field.number(),
            optional: field.proto3_optional(),
            repeated,
            leading_comments,
            trailing_comments,
        }
    }
}

impl<'a> MessageType<'a> {
    /// Construct message type.
    fn from(
        message_type: &'a DescriptorProto,
        idx: i32,
        info: &'a SourceCodeInfo,
        depth: usize,
    ) -> Self {
        let description = get_description(info, &[4, idx]);

        let mut fields = message_type
            .field
            .iter()
            .enumerate()
            .map(|(i, f)| Field::from(f, info, &[4, idx, 2, as_i32(i)]))
            .collect::<Vec<_>>();

        fields.sort_by(|a, b| a.number.cmp(&b.number));

        let nested = message_type
            .nested_type
            .iter()
            .map(|d| MessageType::from(d, idx, info, depth + 1))
            .collect();

        Self {
            name: message_type.name(),
            description,
            fields,
            nested,
            depth,
        }
    }
}

impl<'a> EnumValue<'a> {
    /// Construct field.
    fn from(value: &'a EnumValueDescriptorProto, info: &'a SourceCodeInfo, path: &[i32]) -> Self {
        let location = info.location.iter().find(|l| l.path == *path);
        let leading_comments = location.map_or("", |l| l.leading_comments());
        let trailing_comments = location.map_or("", |l| l.trailing_comments().trim_end());

        Self {
            name: value.name(),
            number: value.number(),
            leading_comments,
            trailing_comments,
        }
    }
}

impl<'a> EnumType<'a> {
    /// Construct enum type.
    fn from(enum_type: &'a EnumDescriptorProto, idx: i32, info: &'a SourceCodeInfo) -> Self {
        let description = get_description(info, &[5, idx]);

        let mut values = enum_type
            .value
            .iter()
            .enumerate()
            .map(|(i, v)| EnumValue::from(v, info, &[5, idx, 2, as_i32(i)]))
            .collect::<Vec<_>>();

        values.sort_by(|a, b| a.number.cmp(&b.number));

        Self {
            name: enum_type.name(),
            description,
            values,
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

        let name = FullyQualifiedTypeName::from(method.input_type());
        let types = types.get(name.package).unwrap();
        let input_type = types.iter().find(|ty| ty.has_name(name.name)).unwrap();

        let name = FullyQualifiedTypeName::from(method.output_type());
        let output_type = types.iter().find(|ty| ty.has_name(name.name)).unwrap();

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
    use super::FullyQualifiedTypeName;

    #[test]
    fn fully_qualified_type_name_processing() {
        let name = FullyQualifiedTypeName::from(".foo.bar.Baz");
        assert_eq!(name.package, "foo.bar");
        assert_eq!(name.name, "Baz");
    }
}
