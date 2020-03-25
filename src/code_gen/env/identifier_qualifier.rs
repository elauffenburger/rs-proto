use super::*;
use crate::parser::ProtoType;
use std::cell::RefCell;
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
