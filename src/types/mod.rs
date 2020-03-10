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
    pub options: Vec<ProtoOption>,
    pub messages: Vec<Message>,
    pub fields: Vec<MessageField>,
}

impl<'a> Message {
    pub fn new(name: String) -> Self {
        Message {
            name,
            options: vec![],
            messages: vec![],
            fields: vec![],
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum MessageFieldModifier {
    Required,
    Optional,
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
pub struct Program {
    pub types: Vec<ProtoType>,
}

impl Program {
    pub fn new() -> Program {
        Program { types: vec![] }
    }
}
