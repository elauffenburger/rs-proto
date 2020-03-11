#[derive(Debug, PartialEq)]
pub enum ProtoType {
    Message(Message),
    Enum(Enum),
}

#[derive(Debug, PartialEq)]
pub struct ProtoOption {
    pub name: String,
    pub field_path: Option<String>,
    pub value: ProtoConstant,
}

#[derive(Debug, PartialEq)]
pub enum ProtoConstant {
    Numeric(f32),
    Str(String),
    Boolean(bool)
}

#[derive(Debug, PartialEq)]
pub struct Message {
    pub name: String,
    pub options: Vec<ProtoOption>,
    pub types: Vec<ProtoType>,
    pub fields: Vec<MessageField>,
}

impl<'a> Message {
    pub fn new(name: String) -> Self {
        Message {
            name,
            options: vec![],
            types: vec![],
            fields: vec![],
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum MessageFieldModifier {
    Required,
    Optional,
    Repeated,
}

#[derive(Debug, PartialEq)]
pub struct MessageField {
    pub modifier: Option<MessageFieldModifier>,
    pub field_type: String,
    pub name: String,
    pub options: Vec<ProtoOption>,
    pub position: u32,
}

#[derive(Debug, PartialEq)]
pub struct Enum {
    pub name: String,
    pub options: Vec<ProtoOption>,
    pub values: Vec<EnumValue>,
}

impl Enum {
    pub fn new(name: String) -> Self {
        Enum {
            name,
            options: vec![],
            values: vec![],
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct EnumValue {
    pub name: String,
    pub options: Vec<ProtoOption>,
    pub position: u32,
}

#[derive(Debug, PartialEq)]
pub enum ProtoSyntax {
    Proto2,
    Proto3,
}

#[derive(Debug, PartialEq)]
pub enum ProtoImportModifier {
    Public
}

#[derive(Debug, PartialEq)]
pub struct ProtoImport {
    pub path: String,
    pub modifier: Option<ProtoImportModifier>,
}

#[derive(Debug, PartialEq)]
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
