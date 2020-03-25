#[derive(Debug, PartialEq, Clone)]
pub enum ProtoType<'a> {
    Message(ProtoMessage<'a>),
    Enum(ProtoEnum<'a>),
}

impl<'a> ProtoType<'a> {
    pub fn get_name(&self) -> &str {
        match self {
            ProtoType::Message(message) => &message.name,
            ProtoType::Enum(enumeration) => &enumeration.name,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ProtoFieldType<'a> {
    Primitive(ProtoPrimitiveType<'a>),
    IdentifierPath(ProtoIdentifierPath<'a>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum ProtoIdentifierPath<'a> {
    Path(&'a str),
}

impl<'a> ProtoIdentifierPath<'a> {
    pub fn get_path_parts(&self) -> Vec<&str> {
        match self {
            ProtoIdentifierPath::Path(path) => path.split('.').collect(),
        }
    }
}

impl<'a> From<&'a str> for ProtoIdentifierPath<'a> {
    fn from(string: &'a str) -> Self {
        ProtoIdentifierPath::Path(string)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ProtoPrimitiveType<'a> {
    Int32,
    Int64,
    Str,
    Boolean,
    Map(Box<ProtoFieldType<'a>>, Box<ProtoFieldType<'a>>),
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
    Boolean(bool),
}

#[derive(Debug, PartialEq, Clone)]
pub struct ProtoMessage<'a> {
    pub name: &'a str,
    pub options: Vec<ProtoOption>,
    pub types: Vec<ProtoType<'a>>,
    pub fields: Vec<ProtoMessageField<'a>>,
}

impl<'a> ProtoMessage<'a> {
    pub fn new(name: &'a str) -> Self {
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
pub struct ProtoMessageField<'a> {
    pub modifier: Option<ProtoMessageFieldModifier>,
    pub field_type: ProtoFieldType<'a>,
    pub name: &'a str,
    pub options: Vec<ProtoOption>,
    pub position: u32,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ProtoEnum<'a> {
    pub name: &'a str,
    pub options: Vec<ProtoOption>,
    pub values: Vec<ProtoEnumValue>,
}

impl<'a> ProtoEnum<'a> {
    pub fn new(name: &'a str) -> Self {
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
    Public,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ProtoImport {
    pub path: String,
    pub modifier: Option<ProtoImportModifier>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Program<'a> {
    pub src: &'a str,
    pub syntax: Option<ProtoSyntax>,
    pub package: Option<&'a str>,
    pub imports: Vec<ProtoImport>,
    pub options: Vec<ProtoOption>,
    pub types: Vec<ProtoType<'a>>,
}

impl<'a> Program<'a> {
    pub fn new(src: &'a str) -> Program {
        Program {
            src,
            syntax: None,
            package: None,
            imports: vec![],
            options: vec![],
            types: vec![],
        }
    }
}
