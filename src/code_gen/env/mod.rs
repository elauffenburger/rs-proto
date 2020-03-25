use crate::parser::{Program, ProtoIdentifierPath, ProtoType};
use std::cell::RefCell;
use std::rc::Rc;

mod identifier_qualifier;
mod proto_type_hierarchy;
mod proto_type_hierarchy_node;

pub use identifier_qualifier::*;
pub use proto_type_hierarchy::*;
pub use proto_type_hierarchy_node::*;

#[derive(Debug)]
pub struct GeneratorEnvironment<'a> {
    // The Program this hierarchy is based off of.
    program: &'a Program<'a>,

    // Hierarchy of known proto types.
    type_hierarchy: Rc<ProtoTypeHierarchy<'a>>,

    // The type we're evaluating operations in the context of.
    type_context: Rc<RefCell<ProtoTypeHierarchyNode<'a>>>,

    // Outputs that should be appended when an environment fully unwinds.
    queued_outputs: Vec<String>,

    // Children of this environment.
    children: Vec<Rc<RefCell<GeneratorEnvironment<'a>>>>,
}

impl<'a> GeneratorEnvironment<'a> {
    pub fn new(program: &'a Program, type_hierarchy: Rc<ProtoTypeHierarchy<'a>>) -> Self {
        let type_context = type_hierarchy.clone().head.clone();

        GeneratorEnvironment {
            program,
            type_hierarchy,
            type_context,
            queued_outputs: vec![],
            children: vec![],
        }
    }

    pub fn new_child(&mut self, proto_type: &ProtoType) -> Rc<RefCell<Self>> {
        let type_hierarchy = self.type_hierarchy.clone();
        let type_context = match type_hierarchy.find_type_node(proto_type) {
            Some(type_context) => type_context,
            None => panic!(
                "Failed to find type '{:?}' in hierarchy: {:?}",
                proto_type, self.type_hierarchy
            ),
        };

        let child = Rc::new(RefCell::new(GeneratorEnvironment {
            program: self.program,
            type_hierarchy,
            type_context,
            queued_outputs: vec![],
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
        path: &ProtoIdentifierPath,
    ) -> Option<Rc<RefCell<ProtoTypeHierarchyNode<'a>>>> {
        path.get_path_parts()
            .iter()
            .fold(None, |acc, identifier| match acc {
                None => {
                    let starting_context = &self.type_context;
                    let derived_context =
                        Self::resolve_proto_type_relative_to_context(identifier, starting_context);

                    Some(derived_context)
                }
                Some(result) => match result {
                    None => None,
                    Some(context) => {
                        let derived_context =
                            Self::resolve_proto_type_relative_to_context(identifier, &context);

                        Some(derived_context)
                    }
                },
            })
            .unwrap()
    }

    fn resolve_proto_type_relative_to_context(
        identifier: &str,
        type_context: &Rc<RefCell<ProtoTypeHierarchyNode<'a>>>,
    ) -> Option<Rc<RefCell<ProtoTypeHierarchyNode<'a>>>> {
        let mut curr = Some(type_context.clone());

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

    pub fn resolve_identifier_path(&self, path: &ProtoIdentifierPath) -> String {
        let resolved_type = match self.resolve_proto_type(path) {
            Some(resolved_type) => resolved_type,
            _ => panic!(
                "Failed to find identifier '{:?}' relative to {:?}",
                path, self
            ),
        };

        let identifier = resolved_type
            .borrow()
            .fully_qualified_identifier
            .clone()
            .expect("expected fully qualified identifier on non-root node");

        identifier.to_string()
    }

    pub fn queue_output(&mut self, output: String) {
        self.queued_outputs.push(output);
    }

    pub fn flush_queued_outputs(&mut self) -> Vec<String> {
        self.queued_outputs.drain(0..).collect()
    }

    pub fn flush_queued_outputs_deep(&mut self) -> Vec<String> {
        self.flush_queued_outputs()
            .into_iter()
            .chain(
                self.children
                    .iter()
                    .flat_map(|child| child.borrow_mut().flush_queued_outputs_deep()),
            )
            .collect()
    }
}
