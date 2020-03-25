use crate::parser::{Program, ProtoIdentifierPath, ProtoType};
use std::cell::RefCell;
use std::fmt;
use std::fmt::Debug;
use std::rc::Rc;

type IdentifierQualfifierFn = Fn(&ProtoType, Rc<RefCell<ProtoTypeHierarchyNode>>) -> String;

pub enum IdentifierQualifier {
    IdentifierQualifier(Box<IdentifierQualfifierFn>),
}

impl IdentifierQualifier {
    pub fn new(qualifier_fn: Box<IdentifierQualfifierFn>) -> Self {
        IdentifierQualifier::IdentifierQualifier(qualifier_fn)
    }

    pub fn invoke(
        &self,
        proto_type: &ProtoType,
        parent: Rc<RefCell<ProtoTypeHierarchyNode>>,
    ) -> String {
        match self {
            IdentifierQualifier::IdentifierQualifier(qualifier_fn) => {
                qualifier_fn(proto_type, parent)
            }
        }
    }
}

pub struct ProtoTypeHierarchy<'a> {
    // The head of this hierarchy.
    pub head: Rc<RefCell<ProtoTypeHierarchyNode<'a>>>,
}

impl<'a> Debug for ProtoTypeHierarchy<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<ProtoTypeHierarchy>")
    }
}

impl<'a> ProtoTypeHierarchy<'a> {
    pub fn from_program(program: &'a Program, identifier_qualifier: IdentifierQualifier) -> Self {
        let head = Rc::new(RefCell::new(ProtoTypeHierarchyNode::new_head()));

        for proto_type in &program.types {
            let child = ProtoTypeHierarchyNode::new(
                head.clone(),
                Rc::new(proto_type.clone()),
                &identifier_qualifier,
            );

            head.borrow_mut().children.push(child);
        }

        ProtoTypeHierarchy { head: head }
    }

    pub fn find_type_node(
        &self,
        proto_type: &ProtoType,
    ) -> Option<Rc<RefCell<ProtoTypeHierarchyNode<'a>>>> {
        Self::find_type_node_rec(self.head.clone(), proto_type)
    }

    fn find_type_node_rec(
        node: Rc<RefCell<ProtoTypeHierarchyNode<'a>>>,
        proto_type: &ProtoType,
    ) -> Option<Rc<RefCell<ProtoTypeHierarchyNode<'a>>>> {
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

pub struct ProtoTypeHierarchyNode<'a> {
    // The parent of this node (if this is not the root node).
    pub parent: Option<Rc<RefCell<ProtoTypeHierarchyNode<'a>>>>,

    // The type represented by this node (if present).
    pub proto_type: Option<Rc<ProtoType<'a>>>,

    // The fully qualified name of the type (if present).
    pub fully_qualified_identifier: Option<String>,

    // Children of this node.
    pub children: Vec<Rc<RefCell<ProtoTypeHierarchyNode<'a>>>>,
}

impl<'a> Debug for ProtoTypeHierarchyNode<'a> {
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

impl<'a> ProtoTypeHierarchyNode<'a> {
    pub fn new_head() -> Self {
        ProtoTypeHierarchyNode {
            parent: None,
            proto_type: None,
            fully_qualified_identifier: None,
            children: vec![],
        }
    }

    pub fn new(
        parent: Rc<RefCell<ProtoTypeHierarchyNode<'a>>>,
        proto_type: Rc<ProtoType<'a>>,
        identifier_qualifier: &IdentifierQualifier,
    ) -> Rc<RefCell<Self>> {
        let fully_qualified_identifier = identifier_qualifier.invoke(&proto_type, parent.clone());

        let result = Rc::new(RefCell::new(ProtoTypeHierarchyNode {
            parent: Some(parent),
            proto_type: Some(proto_type.clone()),
            fully_qualified_identifier: Some(fully_qualified_identifier),
            children: vec![],
        }));

        result.borrow_mut().children = match &*proto_type {
            ProtoType::Message(message) => message
                .types
                .iter()
                .map(|nested_type| {
                    ProtoTypeHierarchyNode::new(
                        result.clone(),
                        Rc::new(nested_type.clone()),
                        identifier_qualifier,
                    )
                })
                .collect(),
            ProtoType::Enum(_) => vec![],
        };

        result
    }
}

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
