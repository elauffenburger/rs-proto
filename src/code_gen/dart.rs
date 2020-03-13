use super::CodeGenerator;
use crate::parser::*;
use crate::utils::{camel_case, CasedString};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const BASE_ENUM_TYPE: &'static str = "ProtobufEnum";

pub struct DartCodeGenerator {
    parser: Box<Parser>,
}

type QueuedOp = FnOnce(Rc<RefCell<GeneratorEnvironment>>) -> Result<String, String>;

pub struct GeneratorEnvironment {
    // A lookup for a fully-qualified version of an identifier.
    identifier_lookup: HashMap<String, String>,

    // Operations that should be performed when an environment fully unwinds.
    queued_ops: Vec<Box<QueuedOp>>,

    // The name of the scope for this GeneratorEnvironment.
    scope_name: Option<String>,

    // Children of this environment.
    children: Vec<Rc<RefCell<GeneratorEnvironment>>>,
}

impl GeneratorEnvironment {
    pub fn new() -> Self {
        GeneratorEnvironment {
            identifier_lookup: HashMap::new(),
            queued_ops: vec![],
            scope_name: None,
            children: vec![],
        }
    }

    pub fn new_child(&mut self, scope_name: Option<String>) -> Rc<RefCell<Self>> {
        let child = Rc::new(RefCell::new(GeneratorEnvironment {
            identifier_lookup: HashMap::new(),
            queued_ops: vec![],
            children: vec![],
            scope_name: match &self.scope_name {
                Some(parent_name) => match scope_name {
                    Some(name) => Some(format!("{}_{}", parent_name, name)),
                    None => Some(parent_name.to_string()),
                },
                None => match scope_name {
                    Some(name) => Some(name.to_string()),
                    None => None,
                },
            },
        }));

        self.children.push(child.clone());

        child
    }

    pub fn get_qualified_type_name<'a>(&self, name: &'a str) -> String {
        match &self.scope_name {
            Some(parent_scope_name) => format!("{}_{}", parent_scope_name, name),
            None => name.to_string(),
        }
    }

    pub fn resolve_identifier<'a>(&self, identifier: &'a str) -> String {
        match self.identifier_lookup.get(identifier) {
            Some(identifier) => identifier.to_string(),
            None => identifier.to_string(),
        }
    }

    pub fn queue_op(&mut self, op: Box<QueuedOp>) {
        self.queued_ops.push(op);
    }
}

impl DartCodeGenerator {
    pub fn new(parser: Box<Parser>) -> Self {
        DartCodeGenerator { parser }
    }

    fn gen_type(
        proto_type: &ProtoType,
        env: Rc<RefCell<GeneratorEnvironment>>,
    ) -> Result<String, String> {
        match proto_type {
            ProtoType::Enum(enumeration) => Self::gen_enum(&enumeration, env, 0),
            ProtoType::Message(message) => Self::gen_message(&message, env, 0),
            err @ _ => Err(format!("Unknown proto type '{:?}'", err)),
        }
    }

    fn gen_message<'a>(
        message: &ProtoMessage,
        env: Rc<RefCell<GeneratorEnvironment>>,
        indent: usize,
    ) -> Result<String, String> {
        let mut result = vec![];

        let indentation = "\t".repeat(indent);
        let inner_indentation = "\t".repeat(indent);

        result.push(format!(
            "{}class {} {{\n",
            indentation,
            env.borrow().get_qualified_type_name(&message.name)
        ));

        for field in &message.fields {
            result.push(format!(
                "{}{}\n",
                &inner_indentation,
                Self::gen_message_field(field, env.clone(), indent + 1)?
            ));
        }

        result.push(format!("{}}}", indentation));

        // Queue up message ops to be written after we finish unrolling the environment.
        for proto_type in &message.types {
            let proto_type = proto_type.clone();

            env.borrow_mut()
                .queue_op(Box::new(move |env| Self::gen_type(&proto_type, env)));
        }

        Ok(result.join(""))
    }

    fn gen_message_field(
        field: &ProtoMessageField,
        env: Rc<RefCell<GeneratorEnvironment>>,
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
        env: Rc<RefCell<GeneratorEnvironment>>,
    ) -> Result<String, String> {
        match field_type {
            ProtoFieldType::Identifier(identifier) => {
                Ok(env.borrow().resolve_identifier(identifier))
            }
            ProtoFieldType::Primitive(primitive) => match primitive {
                ProtoPrimitiveType::Int32 | ProtoPrimitiveType::Int64 => Ok("int".to_string()),
                ProtoPrimitiveType::Boolean => Ok("bool".to_string()),
                ProtoPrimitiveType::Str => Ok("String".to_string()),
                ProtoPrimitiveType::Map(key, value) => Ok(format!(
                    "Map<{}, {}>",
                    Self::get_dart_type(key, env.clone())?,
                    Self::get_dart_type(value, env.clone())?
                )),
            },
        }
    }

    fn gen_enum(
        enumeration: &ProtoEnum,
        env: Rc<RefCell<GeneratorEnvironment>>,
        indent: usize,
    ) -> Result<String, String> {
        let mut result = vec![];

        let indentation = "\t".repeat(indent as usize);
        let enum_name = env.borrow().get_qualified_type_name(&enumeration.name);

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

        let env = Rc::new(RefCell::new(GeneratorEnvironment::new()));
        for proto_type in &prog.types {
            result.push(Self::gen_type(proto_type, env.clone())?);
        }

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

        assert_eq!(result,
        "class Foo {
}

class Foo_Bar {
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
}

class Foo_Baz_Bar extends ProtobufEnum {
\tstatic List<Foo_Baz_Bar> values = [
\t];
\tFoo_Baz_Bar._(int position, String name) {
\t\tthis.position = position;
\t\tthis.name = name;
\t}
}
            ");
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
