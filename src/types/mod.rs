#[derive(Debug, PartialEq)]
pub enum ProtoType {
    Message(Message),
    Enum(Enum),
}

#[derive(Debug, PartialEq)]
pub struct ProtoOption {
    pub name: String,
    pub field_path: Option<String>,
    pub value: String,
}

#[derive(Debug, PartialEq)]
pub struct Message {
    pub name: String,
    pub fields: Vec<MessageField>,
}

impl<'a> Message {
    pub fn new(name: String) -> Self {
        Message { name, fields: vec![] }
    }
}

#[derive(Debug, PartialEq)]
pub enum MessageFieldModifier {
    Required,
    Optional
}

#[derive(Debug, PartialEq)]
pub struct MessageField {
    pub modifier: Option<MessageFieldModifier>,
    pub field_type: String,
    pub name: String,
    pub option: Option<ProtoOption>,
    pub position: u32,
}

#[derive(Debug, PartialEq)]
pub struct Enum {
    pub name: String,
    pub values: Vec<EnumValue>,
}

#[derive(Debug, PartialEq)]
pub struct EnumValue {
    pub name: String,
    pub option: Option<ProtoOption>,
    pub position: u32,
}

#[derive(Debug, PartialEq)]
pub struct Program {
    pub types: Vec<ProtoType>,
}

impl Program {
    pub fn new() -> Program {
        Program { types: vec![] }
    }
}
