#[derive(Debug, PartialEq, Clone)]
pub enum ProtoType {
    Message(ProtoMessage),
    Enum(ProtoEnum),
}

#[derive(Debug, PartialEq, Clone)]
pub enum ProtoFieldType {
    Primitive(ProtoPrimitiveType),
    Identifier(String)
}

#[derive(Debug, PartialEq, Clone)]
pub enum ProtoPrimitiveType {
    Int32,
    Int64,
    Str,
    Boolean,
    Map(Box<ProtoFieldType>, Box<ProtoFieldType>)
}

#[derive(Debug, PartialEq, Clone)]
pub struct ProtoOption {
    pub name: String,
    pub field_path: Option<String>,
    pub value: ProtoConstant,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ProtoConstant {
    Numeric(f32),
    Str(String),
    Boolean(bool)
}

#[derive(Debug, PartialEq, Clone)]
pub struct ProtoMessage {
    pub name: String,
    pub options: Vec<ProtoOption>,
    pub types: Vec<ProtoType>,
    pub fields: Vec<ProtoMessageField>,
}

impl<'a> ProtoMessage {
    pub fn new(name: String) -> Self {
        ProtoMessage {
            name,
            options: vec![],
            types: vec![],
            fields: vec![],
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ProtoMessageFieldModifier {
    Required,
    Optional,
    Repeated,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ProtoMessageField {
    pub modifier: Option<ProtoMessageFieldModifier>,
    pub field_type: ProtoFieldType,
    pub name: String,
    pub options: Vec<ProtoOption>,
    pub position: u32,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ProtoEnum {
    pub name: String,
    pub options: Vec<ProtoOption>,
    pub values: Vec<ProtoEnumValue>,
}

impl ProtoEnum {
    pub fn new(name: String) -> Self {
        ProtoEnum {
            name,
            options: vec![],
            values: vec![],
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ProtoEnumValue {
    pub name: String,
    pub options: Vec<ProtoOption>,
    pub position: u32,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ProtoSyntax {
    Proto2,
    Proto3,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ProtoImportModifier {
    Public
}

#[derive(Debug, PartialEq, Clone)]
pub struct ProtoImport {
    pub path: String,
    pub modifier: Option<ProtoImportModifier>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Program {
    pub syntax: Option<ProtoSyntax>,
    pub package: Option<String>,
    pub imports: Vec<ProtoImport>,
    pub options: Vec<ProtoOption>,
    pub types: Vec<ProtoType>,
}

impl Program {
    pub fn new() -> Program {
        Program {
            syntax: None,
            package: None,
            imports: vec![],
            options: vec![],
            types: vec![],
        }
    }
}
