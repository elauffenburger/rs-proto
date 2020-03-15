use super::CodeGenerator;
use crate::parser::*;
use crate::utils::{camel_case, CasedString};
use std::cell::RefCell;
use std::fmt;
use std::fmt::Debug;
use std::rc::Rc;

const BASE_ENUM_TYPE: &'static str = "ProtobufEnum";

pub struct DartCodeGenerator {
    parser: Box<Parser>,
}

type QueuedOpFn = FnMut(&mut GeneratorEnvironment) -> Result<String, String>;

pub enum QueuedOp {
    QueuedOp(Box<QueuedOpFn>),
}

impl Debug for QueuedOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<QueuedOp>")
    }
}

pub struct ProtoTypeHierarchy {
    // The head of this hierarchy.
    pub head: Rc<RefCell<ProtoTypeHierarchyNode>>,
}

impl Debug for ProtoTypeHierarchy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<ProtoTypeHierarchy>")
    }
}

impl ProtoTypeHierarchy {
    pub fn from_program(program: &Program) -> Self {
        let head = Rc::new(RefCell::new(ProtoTypeHierarchyNode::new_head()));

        for proto_type in &program.types {
            let child = ProtoTypeHierarchyNode::new(head.clone(), Rc::new(proto_type.clone()));

            head.borrow_mut().children.push(child);
        }

        ProtoTypeHierarchy { head: head }
    }

    pub fn find_type_node(
        &self,
        proto_type: &ProtoType,
    ) -> Option<Rc<RefCell<ProtoTypeHierarchyNode>>> {
        Self::find_type_node_rec(self.head.clone(), proto_type)
    }

    fn find_type_node_rec(
        node: Rc<RefCell<ProtoTypeHierarchyNode>>,
        proto_type: &ProtoType,
    ) -> Option<Rc<RefCell<ProtoTypeHierarchyNode>>> {
        if let Some(node_proto_type) = node.borrow().proto_type.clone() {
            if (*node_proto_type) == *proto_type {
                return Some(node.clone());
            }
        }

        for child in &node.borrow().children {
            match Self::find_type_node_rec(child.clone(), proto_type) {
                result @ Some(_) => return result,
                None => {}
            }
        }

        None
    }
}

pub struct ProtoTypeHierarchyNode {
    // The parent of this node (if this is not the root node).
    pub parent: Option<Rc<RefCell<ProtoTypeHierarchyNode>>>,

    // The type represented by this node (if present).
    pub proto_type: Option<Rc<ProtoType>>,

    // The fully qualified name of the type (if present).
    pub fully_qualified_identifier: Option<String>,

    // Children of this node.
    pub children: Vec<Rc<RefCell<ProtoTypeHierarchyNode>>>,
}

impl Debug for ProtoTypeHierarchyNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parent = match &self.parent {
            Some(parent) => match parent.borrow().proto_type.clone() {
                Some(proto_type) => format!("Some({:?})", proto_type),
                None => "None".to_string(),
            },
            None => "None".to_string(),
        };

        let proto_type = match self.proto_type.clone() {
            Some(proto_type) => format!("Some({:?})", proto_type),
            None => "None".to_string(),
        };

        let fully_qualified_identifier = self.fully_qualified_identifier.clone();

        let children = format!("{}", self.children.len());

        f.write_fmt(format_args!("ProtoTypeHierarchyNode{{ parent: {}, proto_type: {}, fully_qualified_identifier: {:?}, children: {} }}", parent, proto_type, fully_qualified_identifier, children))
    }
}

impl ProtoTypeHierarchyNode {
    pub fn new_head() -> Self {
        ProtoTypeHierarchyNode {
            parent: None,
            proto_type: None,
            fully_qualified_identifier: None,
            children: vec![],
        }
    }

    pub fn new(
        parent: Rc<RefCell<ProtoTypeHierarchyNode>>,
        proto_type: Rc<ProtoType>,
    ) -> Rc<RefCell<Self>> {
        let fully_qualified_identifier = Some(
            match parent.clone().borrow().fully_qualified_identifier.clone() {
                Some(parent_identifier) => {
                    format!("{}_{}", parent_identifier, &proto_type.clone().get_name())
                }
                None => proto_type.clone().get_name().to_string(),
            },
        );

        let result = Rc::new(RefCell::new(ProtoTypeHierarchyNode {
            parent: Some(parent),
            proto_type: Some(proto_type.clone()),
            fully_qualified_identifier: fully_qualified_identifier,
            children: vec![],
        }));

        result.borrow_mut().children = match &*proto_type {
            ProtoType::Message(message) => message
                .types
                .iter()
                .map(|nested_type| {
                    ProtoTypeHierarchyNode::new(result.clone(), Rc::new(nested_type.clone()))
                })
                .collect(),
            ProtoType::Enum(_) => vec![],
        };

        result
    }
}

#[derive(Debug)]
pub struct GeneratorEnvironment {
    // Hierarchy of known proto types.
    type_hierarchy: Rc<ProtoTypeHierarchy>,

    // The type we're evaluating operations in the context of.
    type_context: Rc<RefCell<ProtoTypeHierarchyNode>>,

    // Operations that should be performed when an environment fully unwinds.
    queued_ops: Vec<QueuedOp>,

    // Children of this environment.
    children: Vec<Rc<RefCell<GeneratorEnvironment>>>,
}

impl GeneratorEnvironment {
    pub fn new(type_hierarchy: Rc<ProtoTypeHierarchy>) -> Self {
        let type_context = type_hierarchy.head.clone();

        GeneratorEnvironment {
            type_hierarchy,
            type_context,
            queued_ops: vec![],
            children: vec![],
        }
    }

    pub fn new_child(&mut self, proto_type: &ProtoType) -> Rc<RefCell<Self>> {
        let type_hierarchy = self.type_hierarchy.clone();
        let type_context = match type_hierarchy.find_type_node(proto_type) {
            Some(type_context) => type_context,
            None => panic!(
                "Failed to find type '{:?}' in hierarchy: {:?}",
                proto_type, type_hierarchy
            ),
        };

        let child = Rc::new(RefCell::new(GeneratorEnvironment {
            type_hierarchy,
            type_context,
            queued_ops: vec![],
            children: vec![],
        }));

        self.children.push(child.clone());

        child
    }

    pub fn get_fully_qualified_identifier(&self) -> Option<String> {
        self.type_context
            .borrow()
            .fully_qualified_identifier
            .clone()
    }

    pub fn resolve_proto_type(
        &self,
        identifier: &str,
    ) -> Option<Rc<RefCell<ProtoTypeHierarchyNode>>> {
        // TODO: this *should* work for types like "Foo" or "Bar", but nested expressions like
        // "Foo.Bar" will fail. We need to break up the identifier into ["Foo", "Bar"] and then
        // iteratively resolve Foo, then Bar from Foo, and then finally return Bar.

        let mut curr = Some(self.type_context.clone());

        loop {
            match curr.clone() {
                Some(node) => match node.borrow().proto_type.clone() {
                    Some(ref proto_type) if proto_type.get_name() == identifier => return curr,
                    _ => {
                        for child in &node.borrow().children {
                            if let Some(proto_type) = child.borrow().proto_type.clone() {
                                if proto_type.get_name() == identifier {
                                    return Some(child.clone());
                                }
                            }
                        }

                        curr = node.borrow().parent.clone();
                    }
                },
                None => return None,
            }
        }
    }

    pub fn resolve_identifier(&self, identifier: &str) -> String {
        let resolved_type = match self.resolve_proto_type(identifier) {
            Some(resolved_type) => resolved_type,
            _ => panic!(
                "Failed to find identifier '{}' relative to {:?}",
                identifier, self
            ),
        };

        let identifier = resolved_type
            .borrow()
            .fully_qualified_identifier
            .clone()
            .expect("expected fully qualified identifier on non-root node");

        identifier.to_string()
    }

    pub fn queue_op(&mut self, op: QueuedOp) {
        self.queued_ops.push(op);
    }

    pub fn flush_queued_ops(&mut self) -> Result<Vec<String>, String> {
        let mut results = vec![];

        while let Some(op) = self.queued_ops.pop() {
            match op {
                QueuedOp::QueuedOp(mut op) => results.push(op(self)?),
            }
        }

        Ok(results)
    }

    pub fn flush_queued_ops_deep(&mut self) -> Result<Vec<String>, String> {
        let mut results = self.flush_queued_ops()?;

        for child in &self.children {
            results.extend(child.borrow_mut().flush_queued_ops_deep()?);
        }

        Ok(results)
    }
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
            ProtoFieldType::Identifier(identifier) => Ok(env.resolve_identifier(identifier)),
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

        let type_hierarchy = ProtoTypeHierarchy::from_program(&prog);
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
