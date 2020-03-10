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

    fn do_parse(parse_root: &mut Pairs<Rule>) -> Result<Program, String> {
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

                Rule::import => match Self::parse_import(stmt) {
                    Ok(i) => prog.imports.push(i),
                    Err(err) => return Err(err),
                },

                Rule::option => match Self::parse_option(stmt) {
                    Ok(o) => prog.options.push(o),
                    Err(err) => return Err(err),
                },

                Rule::enum_def => match Self::parse_enum(stmt) {
                    Ok(e) => prog.types.push(e),
                    Err(err) => return Err(err),
                },

                Rule::message_def => match Self::parse_message(stmt) {
                    Ok(m) => prog.types.push(m),
                    Err(err) => return Err(err),
                },

                err @ _ => {
                    return Err(format!(
                        "Unexpected rule '{:?}' found at top level of file.",
                        err
                    ));
                }
            }
        }

        Ok(prog)
    }

    fn parse_enum<'a>(statement: Pair<'a, Rule>) -> Result<ProtoType, String> {
        let mut enum_def_parts = statement.into_inner();

        let name = enum_def_parts.next().unwrap().as_str().to_string();
        let mut result = Enum::new(name);

        let body_parts = enum_def_parts.next().unwrap().into_inner();
        for part in body_parts {
            match part.as_rule() {
                Rule::option => match Self::parse_option(part) {
                    Ok(option) => result.options.push(option),
                    Err(err) => return Err(err),
                },
                Rule::enum_value => {
                    let mut value_parts = part.into_inner();
                    let name = value_parts.next().unwrap().as_str().to_string();
                    let position = value_parts.next().unwrap().as_str().parse::<u32>().unwrap();

                    let options = match Self::parse_field_options(&mut value_parts) {
                        Ok(opts) => opts,
                        Err(err) => return Err(err),
                    };

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

        let name = message_def_parts.next().unwrap().as_str().to_string();
        let mut result = Message::new(name);

        let body = message_def_parts.next().unwrap();

        let body_parts = body.into_inner();
        for part in body_parts {
            match part.as_rule() {
                Rule::option => match Self::parse_option_body(part.into_inner().next().unwrap()) {
                    Ok(option) => result.options.push(option),
                    Err(err) => return Err(err),
                },
                Rule::message_def => match Self::parse_message(part) {
                    Ok(t) => result.types.push(t),
                    Err(err) => return Err(err),
                },
                Rule::message_field => {
                    let mut field_parts = part.into_inner();

                    let modifier = match field_parts.peek().unwrap().as_rule() {
                        Rule::message_field_modifier => {
                            match field_parts.next().unwrap().as_str() {
                                "required" => Some(MessageFieldModifier::Required),
                                "optional" => Some(MessageFieldModifier::Optional),
                                "repeated" => Some(MessageFieldModifier::Repeated),
                                modifier @ _ => return Err(format!("Unkown modifier {}", modifier)),
                            }
                        }
                        _ => None,
                    };

                    let field_type = field_parts.next().unwrap().as_str().to_string();
                    let name = field_parts.next().unwrap().as_str().to_string();
                    let position = field_parts.next().unwrap().as_str().parse::<u32>().unwrap();

                    let options = match Self::parse_field_options(&mut field_parts) {
                        Ok(opts) => opts,
                        Err(err) => return Err(err),
                    };

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

    fn parse_option(option: Pair<Rule>) -> Result<ProtoOption, String> {
        let option_body_pair = option.into_inner().next().unwrap();
        Self::parse_option_body(option_body_pair)
    }

    fn parse_field_options(next_pairs: &mut Pairs<Rule>) -> Result<Vec<ProtoOption>, String> {
        let mut options = vec![];
        for next in next_pairs {
            match next.as_rule() {
                Rule::field_option => match Self::parse_field_option(next) {
                    Ok(option) => options.push(option),
                    Err(err) => return Err(err),
                },
                err @ _ => {
                    return Err(format!(
                        "Unknown token encountered while parsing field options: {:?}",
                        err
                    ));
                }
            }
        }

        Ok(options)
    }

    fn parse_field_option(option: Pair<Rule>) -> Result<ProtoOption, String> {
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

    fn parse_import(statement: Pair<Rule>) -> Result<ProtoImport, String> {
        let mut import_parts = statement.into_inner();

        let modifier = match import_parts.peek().unwrap().as_rule() {
            Rule::import_modifier => match import_parts.next().unwrap().as_str() {
                "public" => Some(ProtoImportModifier::Public),
                err @ _ => return Err(format!("Unknown import modifier '{}'", err)),
            },
            _ => None,
        };

        let path = import_parts.next().unwrap().as_str().to_string();

        Ok(ProtoImport { modifier, path })
    }
}

impl Parser for ParserImpl {
    fn parse<'a>(&self, input: &'a str) -> Result<Program, String> {
        match Self::parse_pest(input) {
            Err(err) => Err(format!("{}", err)),
            Ok(mut parse_root) => Self::do_parse(&mut parse_root),
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
    fn test_reference_example() {
        let program = parse_test!("../test_data/reference_example.proto");

        assert_eq!(
            program,
            Program {
                syntax: Some(ProtoSyntax::Proto3),
                imports: vec![ProtoImport {
                    path: "other.proto".to_string(),
                    modifier: Some(ProtoImportModifier::Public)
                }],
                package: None,
                options: vec![ProtoOption {
                    name: "java_package".to_string(),
                    field_path: None,
                    value: "\"com.example.foo\"".to_string()
                }],
                types: vec![
                    ProtoType::Enum(Enum {
                        name: "EnumAllowingAlias".to_string(),
                        options: vec![ProtoOption {
                            name: "allow_alias".to_string(),
                            field_path: None,
                            value: "true".to_string()
                        }],
                        values: vec![
                            EnumValue {
                                name: "UNKNOWN".to_string(),
                                options: vec![],
                                position: 0
                            },
                            EnumValue {
                                name: "STARTED".to_string(),
                                options: vec![],
                                position: 1
                            },
                            EnumValue {
                                name: "RUNNING".to_string(),
                                options: vec![ProtoOption {
                                    name: "custom_option".to_string(),
                                    field_path: None,
                                    value: "\"hello world\"".to_string()
                                }],
                                position: 2
                            },
                        ]
                    }),
                    ProtoType::Message(Message {
                        name: "outer".to_string(),
                        options: vec![ProtoOption {
                            name: "my_option".to_string(),
                            field_path: Some("a".to_string()),
                            value: "true".to_string()
                        }],
                        types: vec![ProtoType::Message(Message {
                            name: "inner".to_string(),
                            options: vec![],
                            types: vec![],
                            fields: vec![MessageField {
                                name: "ival".to_string(),
                                modifier: None,
                                field_type: "int64".to_string(),
                                options: vec![],
                                position: 1
                            }]
                        })],
                        fields: vec![
                            MessageField {
                                name: "inner_message".to_string(),
                                field_type: "inner".to_string(),
                                modifier: Some(MessageFieldModifier::Repeated),
                                options: vec![],
                                position: 2
                            },
                            MessageField {
                                name: "enum_field".to_string(),
                                field_type: "EnumAllowingAlias".to_string(),
                                modifier: None,
                                options: vec![],
                                position: 3
                            },
                            MessageField {
                                name: "my_map".to_string(),
                                field_type: "map<int32, string>".to_string(),
                                modifier: None,
                                options: vec![],
                                position: 4
                            },
                        ]
                    })
                ],
            }
        )
    }

    #[test]
    fn test_top_level_concepts() {
        let program = parse_test!("../test_data/top_level_concepts.proto");

        assert_eq!(
            program,
            Program {
                syntax: Some(ProtoSyntax::Proto3),
                package: Some("foo.bar.baz".to_string()),
                imports: vec![],
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
                imports: vec![],
                options: vec![],
                types: vec![ProtoType::Message(Message {
                    name: "Person".to_string(),
                    options: vec![],
                    types: vec![],
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
                imports: vec![],
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
