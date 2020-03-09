pub enum Type<'a> {
    Message(Message<'a>),
    Enum(Enum<'a>),
}

pub struct Message<'a> {
    pub name: String,
    pub fields: Vec<MessageField<'a>>,
}

impl<'a> Message<'a> {
    pub fn new() -> Self {
        Message { name: "".to_string(), fields: vec![] }
    }
}

pub enum MessageFieldModifier {
    Required,
    Optional
}

pub struct MessageField<'a> {
    pub modifier: Option<MessageFieldModifier>,
    pub field_type: &'a str,
    pub name: &'a str,
    pub position: u32,
}

pub struct Enum<'a> {
    pub name: &'a str,
    pub values: Vec<EnumField<'a>>,
}

pub struct EnumField<'a> {
    pub name: &'a str,
    pub position: u32,
}

pub struct Program<'a> {
    pub types: Vec<Type<'a>>,
}

impl<'a> Program<'a> {
    pub fn new() -> Program<'a> {
        Program { types: vec![] }
    }
}
