use super::*;
use crate::parser::{Program, ProtoType};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

pub struct ProtoTypeHierarchy<'a> {
    // The head of this hierarchy.
    pub head: Rc<RefCell<ProtoTypeHierarchyNode<'a>>>,
}

impl<'a> fmt::Debug for ProtoTypeHierarchy<'a> {
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

        ProtoTypeHierarchy { head }
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
            if let result @ Some(_) = Self::find_type_node_rec(child.clone(), proto_type) {
                return result;
            }
        }

        None
    }
}
