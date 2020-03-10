use crate::types::Program;
use crate::types::*;
use pest::iterators::{Pair, Pairs};
use pest::Parser as PestParser;

#[derive(Parser)]
#[grammar = "../grammars/proto.pest"]
struct PestProtoParser;

pub trait Parser {
    fn parse<'a>(&self, input: &'a str) -> Result<Program, String>;
}

pub struct ParserImpl {}

impl ParserImpl {
    pub fn new() -> Self {
        ParserImpl {}
    }

    fn parse_pest<'a>(prog: &'a str) -> Result<Pairs<Rule>, pest::error::Error<Rule>> {
        PestProtoParser::parse(Rule::program, prog)
    }

    fn parse_enum<'a>(statement: Pair<'a, Rule>) -> Result<ProtoType, String> {
        Err("not implemented".to_string())
    }

    fn parse_message<'a>(statement: Pair<'a, Rule>) -> Result<ProtoType, String> {
        assert_eq!(statement.as_rule(), Rule::message_def);

        let mut message_def_parts = statement.into_inner();

        let message_name = message_def_parts.next().unwrap();
        let name = message_name.as_str().to_string();

        let mut result = Message::new(name);

        let message_body = message_def_parts.next().unwrap();

        let message_body_parts = message_body.into_inner();
        for message_body_part in message_body_parts {
            match message_body_part.as_rule() {
                Rule::message_field => {
                    let field = message_body_part;
                    let mut field_parts = field.into_inner();

                    let modifier = {
                        let next_pair = field_parts.peek().unwrap();

                        match next_pair.as_rule() {
                            Rule::message_field_modifier => {
                                match field_parts.next().unwrap().as_str() {
                                    "required" => Some(MessageFieldModifier::Required),
                                    "optional" => Some(MessageFieldModifier::Optional),
                                    modifier @ _ => {
                                        return Err(format!("Unkown modiifer {}", modifier));
                                    }
                                }
                            }
                            _ => None,
                        }
                    };

                    let field_type = field_parts.next().unwrap().as_str().to_string();
                    let name = field_parts.next().unwrap().as_str().to_string();
                    let position = field_parts.next().unwrap().as_str().parse::<u32>().unwrap();

                    let option = {
                        let maybe_next_pair = field_parts.peek();

                        match maybe_next_pair {
                            Some(next_pair) => match next_pair.as_rule() {
                                Rule::field_option => match Self::parse_field_option(next_pair) {
                                    Ok(option) => Some(option),
                                    Err(err) => return Err(err),
                                },
                                _ => None,
                            },
                            None => None,
                        }
                    };

                    result.fields.push(MessageField {
                        modifier,
                        name,
                        field_type,
                        option,
                        position,
                    })
                }
                err @ _ => {
                    return Err(format!(
                        "Unexpected rule {:?} when parsing message body",
                        err
                    ));
                }
            }
        }

        Ok(ProtoType::Message(result))
    }

    fn parse_field_option(option: Pair<Rule>) -> Result<ProtoOption, String> {
        assert_eq!(option.as_rule(), Rule::field_option);

        let option_body_pair = option.into_inner().next().unwrap();
        Self::parse_option(option_body_pair)
    }

    fn parse_option(option_body_pair: Pair<Rule>) -> Result<ProtoOption, String> {
        let mut option_body_inner = option_body_pair.into_inner();
        let mut option_identifier_pairs = option_body_inner.next().unwrap().into_inner();

        let name = option_identifier_pairs.next().unwrap().as_str().to_string();
        let field_path = match option_identifier_pairs.peek() {
            None => None,
            Some(_) => Some(
                option_identifier_pairs
                    .map(|pair| pair.as_str())
                    .collect::<Vec<&str>>()
                    .join("."),
            ),
        };

        let value = option_body_inner.next().unwrap().as_str().to_string();

        Ok(ProtoOption {
            name,
            field_path,
            value,
        })
    }
}

impl Parser for ParserImpl {
    fn parse<'a>(&self, input: &'a str) -> Result<Program, String> {
        match Self::parse_pest(input) {
            Err(err) => Err(format!("{}", err)),
            Ok(mut parse_root) => {
                let mut prog = Program::new();

                let top_level_stmts = parse_root.next().unwrap().into_inner();
                for stmt in top_level_stmts {
                    match stmt.as_rule() {
                        Rule::enum_def => match Self::parse_enum(stmt) {
                            Ok(e) => prog.types.push(e),
                            Err(err) => return Err(err),
                        },

                        Rule::message_def => match Self::parse_message(stmt) {
                            Ok(m) => prog.types.push(m),
                            Err(err) => return Err(err),
                        },
                        _ => return Err("Unexpected rule found at top level of file.".to_string()),
                    }
                }

                Ok(prog)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    macro_rules! parse_test_raw {
        ($test_path: expr) => {{
            ParserImpl::parse_pest(include_str!($test_path))
                .expect(&format!("failed to parse {}", $test_path))
        }};
    }

    macro_rules! parse_test {
        ($test_path: expr) => {{
            let parser = ParserImpl::new();
            parser
                .parse(include_str!($test_path))
                .expect(&format!("failed to parse {}", $test_path))
        }};
    }

    use super::*;

    #[test]
    fn test_message() {
        let program = parse_test!("../test_data/message.proto");

        assert_eq!(program.types.len(), 1);
        assert_eq!(
            program,
            Program {
                types: vec![ProtoType::Message(Message {
                    name: "Person".to_string(),
                    fields: vec![
                        MessageField {
                            field_type: "string".to_string(),
                            name: "first_name".to_string(),
                            modifier: None,
                            option: None,
                            position: 1
                        },
                        MessageField {
                            field_type: "string".to_string(),
                            name: "last_name".to_string(),
                            modifier: None,
                            option: None,
                            position: 2
                        },
                        MessageField {
                            field_type: "int64".to_string(),
                            name: "date_of_birth_unix_epoch".to_string(),
                            modifier: None,
                            option: None,
                            position: 3
                        }
                    ]
                })]
            }
        );
    }

    #[test]
    fn test_enum() {
        let program = parse_test!("../test_data/enum.proto");

        assert_eq!(program.types.len(), 1);
        assert_eq!(
            program,
            Program {
                types: vec![ProtoType::Enum(Enum {
                    name: "RelationshipType".to_string(),
                    values: vec![
                        EnumValue {
                            name: "PARENT".to_string(),
                            option: None,
                            position: 1 
                        },
                    ]
                })]
            }
        );
    }

    #[test]
    fn test_message_raw() {
        let root = parse_test_raw!("../test_data/message.proto");

        let program = root.peek().unwrap();
        assert_eq!(program.as_rule(), Rule::program);

        let message = program.into_inner().peek().unwrap();
        assert_eq!(message.as_rule(), Rule::message_def);

        let mut message_pairs = message.into_inner();

        let message_name = message_pairs.nth(0).unwrap();
        assert_eq!(message_name.as_str(), "Person");

        let message_body = message_pairs.nth(0).unwrap();
        let mut message_fields = message_body.into_inner();

        assert_message_field_raw(
            message_fields.nth(0).unwrap(),
            MessageField {
                modifier: None,
                field_type: "string".to_string(),
                name: "first_name".to_string(),
                option: None,
                position: 1,
            },
        );

        assert_message_field_raw(
            message_fields.nth(0).unwrap(),
            MessageField {
                modifier: None,
                field_type: "string".to_string(),
                name: "last_name".to_string(),
                option: None,
                position: 2,
            },
        );

        assert_message_field_raw(
            message_fields.nth(0).unwrap(),
            MessageField {
                modifier: None,
                field_type: "int64".to_string(),
                name: "date_of_birth_unix_epoch".to_string(),
                option: None,
                position: 3,
            },
        );
    }

    #[test]
    fn test_enum_raw() {
        let prog = parse_test_raw!("../test_data/enum.proto");

        let mut definitions = prog.peek().unwrap().into_inner().collect::<Vec<_>>();
        assert_eq!(definitions.len(), 1);

        let enum_def = definitions.remove(0);
        assert_eq!(enum_def.as_rule(), Rule::enum_def);

        let mut enum_def_parts = enum_def.into_inner().collect::<Vec<_>>();

        let enum_name = enum_def_parts.remove(0);
        assert_eq!(enum_name.as_rule(), Rule::enum_name);
        assert_eq!(enum_name.as_str(), "RelationshipType");

        let enum_body = enum_def_parts.remove(0);
        assert_eq!(enum_body.as_rule(), Rule::enum_body);

        let enum_body_parts = enum_body.into_inner().collect::<Vec<_>>();
    }

    fn assert_message_field_raw<'a>(field: Pair<'a, Rule>, assertion: MessageField) {
        let mut field_parts = field.into_inner();

        match &assertion.modifier {
            Some(modifier) => {
                let maybe_modifier = field_parts.nth(0).unwrap();

                assert_eq!(maybe_modifier.as_rule(), Rule::message_field_modifier);
                assert_eq!(
                    maybe_modifier.as_str(),
                    match modifier {
                        MessageFieldModifier::Optional => "optional",
                        MessageFieldModifier::Required => "required",
                    }
                );
            }
            None => assert!(field_parts.peek().unwrap().as_rule() == Rule::type_identifier),
        }

        let field_type = field_parts.nth(0).unwrap();
        let name = field_parts.nth(0).unwrap();
        let position = field_parts.nth(0).unwrap();

        assert_eq!(&field_type.as_rule(), &Rule::type_identifier);
        assert_eq!(&field_type.as_str(), &assertion.field_type);

        assert_eq!(&name.as_rule(), &Rule::identifier);
        assert_eq!(&name.as_str(), &assertion.name);

        assert_eq!(&position.as_rule(), &Rule::numeric);
        assert_eq!(
            position.as_str().parse::<u32>().unwrap(),
            assertion.position
        );
    }
}
