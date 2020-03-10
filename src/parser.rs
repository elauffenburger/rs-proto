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
        let mut enum_def_parts = statement.into_inner();

        let enum_name = enum_def_parts.next().unwrap().as_str().to_string();
        let mut result = Enum::new(enum_name);

        let enum_body_parts = enum_def_parts.next().unwrap().into_inner();
        for pair in enum_body_parts {
            match pair.as_rule() {
                Rule::option => match Self::parse_option_body(pair.into_inner().next().unwrap()) {
                    Ok(option) => result.options.push(option),
                    Err(err) => return Err(err),
                },
                Rule::enum_value => {
                    let mut value_parts = pair.into_inner();
                    let name = value_parts.next().unwrap().as_str().to_string();
                    let position = value_parts.next().unwrap().as_str().parse::<u32>().unwrap();

                    let mut options = vec![];
                    for next in value_parts {
                        match next.as_rule() {
                            Rule::field_option => match Self::parse_field_option(next) {
                                Ok(option) => options.push(option),
                                Err(err) => return Err(err),
                            },
                            err @ _ => {
                                return Err(format!(
                                    "Unknown token encountered while parsing enum options: {:?}",
                                    err
                                ));
                            }
                        }
                    }

                    result.values.push(EnumValue {
                        name,
                        position,
                        options,
                    })
                }
                err @ _ => {
                    return Err(format!(
                        "Unexpected rule found when parsing enum body: {:?}",
                        err
                    ));
                }
            }
        }

        Ok(ProtoType::Enum(result))
    }

    fn parse_message<'a>(statement: Pair<'a, Rule>) -> Result<ProtoType, String> {
        let mut message_def_parts = statement.into_inner();

        let message_name = message_def_parts.next().unwrap();
        let name = message_name.as_str().to_string();

        let mut result = Message::new(name);

        let message_body = message_def_parts.next().unwrap();

        let message_body_parts = message_body.into_inner();
        for message_body_part in message_body_parts {
            match message_body_part.as_rule() {
                Rule::option => {
                    match Self::parse_option_body(message_body_part.into_inner().next().unwrap()) {
                        Ok(option) => result.options.push(option),
                        Err(err) => return Err(err),
                    }
                }
                Rule::message_def => match Self::parse_message(message_body_part) {
                    Ok(ProtoType::Message(message)) => result.messages.push(message),
                    Ok(other_type) => {
                        return Err(format!(
                            "Unexpected type returned when parsing inner message: {:?}",
                            other_type
                        ));
                    }
                    Err(err) => return Err(err),
                },
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

                    let mut options = vec![];
                    for next in field_parts {
                        match next.as_rule() {
                            Rule::field_option => match Self::parse_field_option(next) {
                                Ok(option) => options.push(option),
                                Err(err) => return Err(err),
                            },
                            _ => {}
                        }
                    }

                    result.fields.push(MessageField {
                        modifier,
                        name,
                        field_type,
                        options,
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
        Self::parse_option_body(option_body_pair)
    }

    fn parse_option_body(option_body_pair: Pair<Rule>) -> Result<ProtoOption, String> {
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

    fn parse_syntax(statement: Pair<Rule>) -> Result<ProtoSyntax, String> {
        match statement.into_inner().next().unwrap().as_str() {
            "proto2" => Ok(ProtoSyntax::Proto2),
            "proto3" => Ok(ProtoSyntax::Proto3),
            syntax @ _ => Err(format!("Unknown proto syntax '{}'", syntax)),
        }
    }

    fn parse_package(statement: Pair<Rule>) -> Result<String, String> {
        Ok(statement.into_inner().next().unwrap().as_str().to_string())
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
                        Rule::syntax => match Self::parse_syntax(stmt) {
                            Ok(s) => prog.syntax = Some(s),
                            Err(err) => return Err(err),
                        },

                        Rule::package => match Self::parse_package(stmt) {
                            Ok(p) => prog.package = Some(p),
                            Err(err) => return Err(err),
                        },

                        Rule::option => {
                            match Self::parse_option_body(stmt.into_inner().next().unwrap()) {
                                Ok(o) => prog.options.push(o),
                                Err(err) => return Err(err),
                            }
                        }

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
    fn test_top_level_concepts() {
        let program = parse_test!("../test_data/top_level_concepts.proto");

        assert_eq!(
            program,
            Program {
                syntax: Some(ProtoSyntax::Proto3),
                package: Some("foo.bar.baz".to_string()),
                options: vec![ProtoOption {
                    name: "java_package".to_string(),
                    field_path: None,
                    value: "\"com.rsproto.toplevelconcepts\"".to_string()
                }],
                types: vec![],
            }
        )
    }

    #[test]
    fn test_message() {
        let program = parse_test!("../test_data/message.proto");

        assert_eq!(program.types.len(), 1);
        assert_eq!(
            program,
            Program {
                syntax: None,
                package: None,
                options: vec![],
                types: vec![ProtoType::Message(Message {
                    name: "Person".to_string(),
                    options: vec![],
                    messages: vec![],
                    fields: vec![
                        MessageField {
                            field_type: "string".to_string(),
                            name: "first_name".to_string(),
                            modifier: None,
                            options: vec![],
                            position: 1
                        },
                        MessageField {
                            field_type: "string".to_string(),
                            name: "last_name".to_string(),
                            modifier: None,
                            options: vec![],
                            position: 2
                        },
                        MessageField {
                            field_type: "int64".to_string(),
                            name: "date_of_birth_unix_epoch".to_string(),
                            modifier: None,
                            options: vec![],
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
                syntax: None,
                package: None,
                options: vec![],
                types: vec![ProtoType::Enum(Enum {
                    name: "RelationshipType".to_string(),
                    options: vec![],
                    values: vec![
                        EnumValue {
                            name: "PARENT".to_string(),
                            options: vec![],
                            position: 1
                        },
                        EnumValue {
                            name: "SIBLING".to_string(),
                            options: vec![],
                            position: 2
                        },
                        EnumValue {
                            name: "CHILD".to_string(),
                            options: vec![],
                            position: 3
                        },
                        EnumValue {
                            name: "ANCESTOR".to_string(),
                            options: vec![],
                            position: 4
                        },
                        EnumValue {
                            name: "DESCENDANT".to_string(),
                            options: vec![],
                            position: 5
                        },
                    ]
                })]
            }
        );
    }
}
