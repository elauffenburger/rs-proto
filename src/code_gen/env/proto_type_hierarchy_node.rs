use super::*;
use crate::parser::ProtoType;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

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

impl<'a> fmt::Debug for ProtoTypeHierarchyNode<'a> {
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
