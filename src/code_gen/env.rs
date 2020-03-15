use crate::parser::*;
use std::cell::RefCell;
use std::fmt;
use std::fmt::Debug;
use std::rc::Rc;

type QueuedOpFn = FnMut(&mut GeneratorEnvironment) -> Result<String, String>;

pub enum QueuedOp {
    QueuedOp(Box<QueuedOpFn>),
}

type IdentifierQualfifierFn = Fn(&ProtoType, &ProtoTypeHierarchyNode) -> String;

pub enum IdentifierQualifier {
    IdentifierQualifier(Box<IdentifierQualfifierFn>),
}

impl IdentifierQualifier {
    pub fn new(qualifier_fn: Box<IdentifierQualfifierFn>) -> Self {
        IdentifierQualifier::IdentifierQualifier(qualifier_fn)
    }

    pub fn invoke(&self, proto_type: &ProtoType, parent: &ProtoTypeHierarchyNode) -> String {
        match self {
            IdentifierQualifier::IdentifierQualifier(qualifierFn) => {
                qualifierFn(proto_type, parent)
            }
        }
    }
}

impl Debug for QueuedOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<QueuedOp>")
    }
}

pub struct ProtoTypeHierarchy<'proto_type> {
    // The head of this hierarchy.
    pub head: Rc<ProtoTypeHierarchyNode<'proto_type>>,
}

impl<'pt> Debug for ProtoTypeHierarchy<'pt> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<ProtoTypeHierarchy>")
    }
}

impl<'pt> ProtoTypeHierarchy<'pt> {
    pub fn from_program(program: &Program, identifier_qualifier: IdentifierQualifier) -> Self {
        let mut head = Rc::new(ProtoTypeHierarchyNode::new_head());

        let mut children = vec![];
        for proto_type in &program.types {
            let child =
                ProtoTypeHierarchyNode::new(head.clone(), proto_type, &identifier_qualifier);

            children.push(child);
        }

        unsafe {
            let raw_head = Rc::into_raw(head);
            (*(raw_head as *mut ProtoTypeHierarchyNode)).children = children;

            head = Rc::from_raw(raw_head);
        }

        ProtoTypeHierarchy { head: head }
    }

    pub fn find_type_node(
        &self,
        proto_type: &ProtoType,
    ) -> Option<Rc<ProtoTypeHierarchyNode<'pt>>> {
        Self::find_type_node_rec(self.head.clone(), proto_type)
    }

    fn find_type_node_rec(
        node: Rc<ProtoTypeHierarchyNode<'pt>>,
        proto_type: &ProtoType,
    ) -> Option<Rc<ProtoTypeHierarchyNode<'pt>>> {
        let node_proto_type_opt = node.proto_type.clone();

        if let Some(node_proto_type) = node_proto_type_opt {
            if (*node_proto_type) == *proto_type {
                return Some(node);
            }
        }

        for child in &node.children {
            match Self::find_type_node_rec(child.clone(), proto_type) {
                result @ Some(_) => return result,
                None => {}
            }
        }

        None
    }
}

pub struct ProtoTypeHierarchyNode<'proto_type> {
    // The parent of this node (if this is not the root node).
    pub parent: Option<Rc<ProtoTypeHierarchyNode<'proto_type>>>,

    // The type represented by this node (if present).
    pub proto_type: Option<&'proto_type ProtoType>,

    // The fully qualified name of the type (if present).
    pub fully_qualified_identifier: Option<String>,

    // Children of this node.
    pub children: Vec<Rc<ProtoTypeHierarchyNode<'proto_type>>>,
}

impl<'pt> Debug for ProtoTypeHierarchyNode<'pt> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parent = match &self.parent {
            Some(parent) => match &parent.proto_type {
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

impl<'pt> ProtoTypeHierarchyNode<'pt> {
    pub fn new_head() -> Self {
        ProtoTypeHierarchyNode {
            parent: None,
            proto_type: None,
            fully_qualified_identifier: None,
            children: vec![],
        }
    }

    pub fn new(
        parent: Rc<ProtoTypeHierarchyNode<'pt>>,
        proto_type: &'pt ProtoType,
        identifier_qualifier: &IdentifierQualifier,
    ) -> Rc<ProtoTypeHierarchyNode<'pt>> {
        let fully_qualified_identifier = identifier_qualifier.invoke(&proto_type, &parent);

        let mut result = Rc::new(ProtoTypeHierarchyNode {
            parent: Some(parent),
            proto_type: Some(proto_type),
            fully_qualified_identifier: Some(fully_qualified_identifier),
            children: vec![],
        });

        let children = match &*proto_type {
            ProtoType::Message(message) => message
                .types
                .iter()
                .map(|nested_type| {
                    ProtoTypeHierarchyNode::new(result.clone(), nested_type, identifier_qualifier)
                })
                .collect(),
            ProtoType::Enum(_) => vec![],
        };

        unsafe {
            let raw_result = Rc::into_raw(result);
            (*(raw_result as *mut Self)).children = children;

            result = Rc::from_raw(raw_result);
        }

        result
    }
}

#[derive(Debug)]
pub struct GeneratorEnvironment<'proto_type> {
    // Hierarchy of known proto types.
    type_hierarchy: Rc<ProtoTypeHierarchy<'proto_type>>,

    // The type we're evaluating operations in the context of.
    type_context: Rc<ProtoTypeHierarchyNode<'proto_type>>,

    // Operations that should be performed when an environment fully unwinds.
    queued_ops: Vec<QueuedOp>,

    // Children of this environment.
    children: Vec<Rc<RefCell<GeneratorEnvironment<'proto_type>>>>,
}

impl<'pt> GeneratorEnvironment<'pt> {
    pub fn new(type_hierarchy: Rc<ProtoTypeHierarchy<'pt>>) -> Self {
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
        self.type_context.fully_qualified_identifier.clone()
    }

    pub fn resolve_proto_type(&self, identifier: &str) -> Option<Rc<ProtoTypeHierarchyNode>> {
        // TODO: this *should* work for types like "Foo" or "Bar", but nested expressions like
        // "Foo.Bar" will fail. We need to break up the identifier into ["Foo", "Bar"] and then
        // iteratively resolve Foo, then Bar from Foo, and then finally return Bar.

        let mut curr = Some(self.type_context.clone());

        loop {
            match curr.clone() {
                Some(node) => match node.proto_type.clone() {
                    Some(ref proto_type) if proto_type.get_name() == identifier => return curr,
                    _ => {
                        for child in &node.children {
                            if let Some(proto_type) = child.proto_type.clone() {
                                if proto_type.get_name() == identifier {
                                    return Some(child.clone());
                                }
                            }
                        }

                        curr = node.parent.clone();
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
