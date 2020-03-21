use super::CodeGenerator;
use crate::code_gen::env::*;
use crate::parser::*;
use crate::utils::{camel_case, CasedString};
use std::cell::RefCell;
use std::rc::Rc;

const BASE_ENUM_TYPE: &'static str = "ProtobufEnum";

pub struct DartCodeGenerator {
    parser: Box<Parser>,
}

impl DartCodeGenerator {
    pub fn new(parser: Box<Parser>) -> Self {
        DartCodeGenerator { parser }
    }

    fn gen_type<'a>(
        proto_type: &ProtoType,
        env: &'a mut GeneratorEnvironment,
    ) -> Result<String, String> {
        match proto_type {
            ProtoType::Enum(enumeration) => Self::gen_enum(&enumeration, env, 0),
            ProtoType::Message(message) => Self::gen_message(&message, env, 0),
            err @ _ => Err(format!("Unknown proto type '{:?}'", err)),
        }
    }

    fn gen_message(
        message: &ProtoMessage,
        env: &mut GeneratorEnvironment,
        indent: usize,
    ) -> Result<String, String> {
        let mut result = vec![];

        let indentation = "\t".repeat(indent);
        let inner_indentation = "\t".repeat(indent);

        let message_name = env
            .get_fully_qualified_identifier()
            .expect("expect to generate message in the context of a proto type");

        result.push(format!("{}class {} {{\n", indentation, &message_name));

        for field in &message.fields {
            result.push(format!(
                "{}{}\n",
                &inner_indentation,
                Self::gen_message_field(field, env, indent + 1)?
            ));
        }

        result.push(format!("{}}}", indentation));

        // Queue up message ops to be written after we finish unrolling the environment.
        for proto_type in &message.types {
            let child_env = env.new_child(proto_type);
            let proto_type = proto_type.clone();

            child_env
                .borrow_mut()
                .queue_op(QueuedOp::QueuedOp(Box::new(move |env| {
                    Ok(format!("\n\n{}", Self::gen_type(&proto_type, env)?))
                })));
        }

        Ok(result.join(""))
    }

    fn gen_message_field<'a>(
        field: &ProtoMessageField,
        env: &'a mut GeneratorEnvironment,
        indent: usize,
    ) -> Result<String, String> {
        let mut result = vec![];

        let indentation = "\t".repeat(indent);

        result.push(format!(
            "{}{} {};",
            indentation,
            Self::get_dart_type(&field.field_type, env)?,
            camel_case(CasedString::SnakeCase(&field.name))
        ));

        Ok(result.join(""))
    }

    fn get_dart_type(
        field_type: &ProtoFieldType,
        env: &mut GeneratorEnvironment,
    ) -> Result<String, String> {
        match field_type {
            ProtoFieldType::IdentifierPath(identifier) => Ok(env.resolve_identifier_path(identifier)),
            ProtoFieldType::Primitive(primitive) => match primitive {
                ProtoPrimitiveType::Int32 | ProtoPrimitiveType::Int64 => Ok("int".to_string()),
                ProtoPrimitiveType::Boolean => Ok("bool".to_string()),
                ProtoPrimitiveType::Str => Ok("String".to_string()),
                ProtoPrimitiveType::Map(key, value) => Ok(format!(
                    "Map<{}, {}>",
                    Self::get_dart_type(key, env)?,
                    Self::get_dart_type(value, env)?
                )),
            },
        }
    }

    fn gen_enum(
        enumeration: &ProtoEnum,
        env: &mut GeneratorEnvironment,
        indent: usize,
    ) -> Result<String, String> {
        let mut result = vec![];

        let indentation = "\t".repeat(indent as usize);

        let enum_name = env
            .get_fully_qualified_identifier()
            .expect("expect to generate message in the context of a proto type");

        result.push(format!(
            "{}class {} extends {} {{\n",
            indentation, enum_name, BASE_ENUM_TYPE
        ));

        result.push(Self::gen_enum_body(
            &enum_name,
            &enumeration.values,
            indent + 1,
        )?);

        result.push(format!("\n{}}}", indentation));

        Ok(result.join(""))
    }

    fn gen_enum_body<'a>(
        enum_name: &'a str,
        enum_values: &Vec<ProtoEnumValue>,
        indent: usize,
    ) -> Result<String, String> {
        let mut result = vec![];

        for value in enum_values.iter() {
            result.push(format!(
                "{}\n",
                Self::gen_enum_value(enum_name, &value, indent)?
            ));
        }

        result.push(format!(
            "\n{}",
            Self::gen_all_enum_values_list(enum_name, enum_values, indent)?
        ));

        result.push(format!("\n\n{}", Self::gen_enum_ctor(enum_name, indent)?));

        Ok(result.join(""))
    }

    fn gen_enum_value<'a, 'b>(
        enum_name: &'a str,
        value: &ProtoEnumValue,
        indent: usize,
    ) -> Result<String, String> {
        let indentation = "\t".repeat(indent as usize);

        Ok(format!(
            "{}static {} {} = {}._({}, \"{}\");",
            indentation,
            enum_name,
            camel_case(CasedString::ScreamingSnakeCase(&value.name)),
            enum_name,
            value.position,
            value.name,
        ))
    }

    fn gen_all_enum_values_list<'a, 'b>(
        enum_name: &'a str,
        enum_values: &Vec<ProtoEnumValue>,
        indent: usize,
    ) -> Result<String, String> {
        let indentation = "\t".repeat(indent as usize);
        let value_indentation = "\t".repeat(indent + 1 as usize);

        let all_values = enum_values
            .iter()
            .map(|value| {
                format!(
                    "{}{}",
                    value_indentation,
                    camel_case(CasedString::ScreamingSnakeCase(&value.name))
                )
            })
            .collect::<Vec<String>>()
            .join(",\n");

        Ok(format!(
            "{}static List<{}> values = [\n{}\n{}];",
            indentation, enum_name, all_values, indentation
        ))
    }

    fn gen_enum_ctor<'a, 'b>(enum_name: &'a str, indent: usize) -> Result<String, String> {
        let indentation = "\t".repeat(indent as usize);
        let inner_indentation = "\t".repeat(indent + 1 as usize);

        let mut result = vec![];

        result.push(format!(
            "{}{}._(int position, String name) {{\n",
            indentation, enum_name
        ));

        result.push(format!("{}this.position = position;\n", inner_indentation));
        result.push(format!("{}this.name = name;\n", inner_indentation));

        result.push(format!("{}}}", indentation));

        Ok(result.join(""))
    }
}

impl CodeGenerator for DartCodeGenerator {
    fn gen_code<'a>(&self, src: &'a str) -> Result<String, String> {
        let mut result = vec![];

        let prog = self.parser.parse(src)?;

        let type_hierarchy = ProtoTypeHierarchy::from_program(
            &prog,
            IdentifierQualifier::new(Box::new(|proto_type, parent| {
                match parent.clone().borrow().fully_qualified_identifier.clone() {
                    Some(parent_identifier) => {
                        format!("{}_{}", parent_identifier, &proto_type.get_name())
                    }
                    None => proto_type.get_name().to_string(),
                }
            })),
        );
        let env = Rc::new(RefCell::new(GeneratorEnvironment::new(Rc::new(
            type_hierarchy,
        ))));

        // Generate all the top-level types.
        for proto_type in &prog.types {
            result.push(Self::gen_type(
                proto_type,
                &mut env.borrow_mut().new_child(proto_type).borrow_mut(),
            )?);
        }

        // Generate any types that were queued up while generating top-level types.
        result.extend(env.borrow_mut().flush_queued_ops_deep()?);

        Ok(result.join(""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ParserImpl;

    macro_rules! gen_code_for_test {
        ($test_path: expr) => {{
            let parser = ParserImpl::new();
            let generator = DartCodeGenerator::new(Box::new(parser));

            generator
                .gen_code(include_str!($test_path))
                .expect("unsuccessful codegen")
        }};
    }

    #[test]
    fn test_nested() {
        let result = gen_code_for_test!("../../test_data/nested.proto");

        assert_eq!(
            result,
            "class Foo {\n}

class Foo_Bar {
\tFoo_Bar bar;
}

class Foo_Bar_Baz extends ProtobufEnum {

\tstatic List<Foo_Bar_Baz> values = [

\t];

\tFoo_Bar_Baz._(int position, String name) {
\t\tthis.position = position;
\t\tthis.name = name;
\t}
}

class Foo_Baz {
\tFoo_Baz_Bar bar;
\tFoo_Baz_Bar bar2;
\tFoo_Bar_Baz baz;
}

class Foo_Baz_Bar extends ProtobufEnum {

\tstatic List<Foo_Baz_Bar> values = [

\t];

\tFoo_Baz_Bar._(int position, String name) {
\t\tthis.position = position;
\t\tthis.name = name;
\t}
}"
        );
    }

    #[test]
    fn test_message() {
        let result = gen_code_for_test!("../../test_data/message.proto");

        assert_eq!(
            result,
            "class Person {
\tString firstName;
\tString lastName;
\tint dateOfBirthUnixEpoch;
}"
        );
    }

    #[test]
    fn test_enum() {
        let result = gen_code_for_test!("../../test_data/enum.proto");

        assert_eq!(
            result,
            "class RelationshipType extends ProtobufEnum {
\tstatic RelationshipType unknownValue = RelationshipType._(0, \"UNKNOWN_VALUE\");
\tstatic RelationshipType parent = RelationshipType._(1, \"PARENT\");
\tstatic RelationshipType sibling = RelationshipType._(2, \"SIBLING\");
\tstatic RelationshipType child = RelationshipType._(3, \"CHILD\");
\tstatic RelationshipType ancestor = RelationshipType._(4, \"ANCESTOR\");
\tstatic RelationshipType descendant = RelationshipType._(5, \"DESCENDANT\");

\tstatic List<RelationshipType> values = [
\t\tunknownValue,
\t\tparent,
\t\tsibling,
\t\tchild,
\t\tancestor,
\t\tdescendant
\t];

\tRelationshipType._(int position, String name) {
\t\tthis.position = position;
\t\tthis.name = name;
\t}
}"
        );
    }
}
