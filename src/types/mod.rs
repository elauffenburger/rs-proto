#[derive(Debug, PartialEq)]
pub enum Type<'a> {
    Message(Message),
    Enum(Enum<'a>),
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

impl Message {
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
pub struct Enum<'a> {
    pub name: &'a str,
    pub values: Vec<EnumField<'a>>,
}

#[derive(Debug, PartialEq)]
pub struct EnumField<'a> {
    pub name: &'a str,
    pub option: Option<ProtoOption>,
    pub position: u32,
}

#[derive(Debug, PartialEq)]
pub struct Program<'a> {
    pub types: Vec<Type<'a>>,
}

impl<'a> Program<'a> {
    pub fn new() -> Program<'a> {
        Program { types: vec![] }
    }
}
