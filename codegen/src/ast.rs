use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub name: String,
    pub elements: Vec<Element>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Term {
    pub name: String,
    pub elements: Vec<Element>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribute {
    pub owner: String,
    pub name: String,
    pub elements: Vec<Element>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocaleEntries {
    pub messages: BTreeMap<String, Message>,
    pub terms: BTreeMap<String, Term>,
    pub attributes: BTreeMap<String, Attribute>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Element {
    Text(String),
    VarRef(String),
    MessageRef(String),
    AttributeRef {
        owner: String,
        name: String,
    },
    TermRef {
        name: String,
        attribute: Option<String>,
        args: BTreeMap<String, Element>,
    },
    /// Term attribute reference as select selector, resolved at compile time.
    TermAttrSelect {
        term: String,
        attr: String,
        variants: Vec<Variant>,
    },
    Select {
        selector: String,
        variants: Vec<Variant>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    pub key: KeyType,
    pub elements: Vec<Element>,
    pub default: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyType {
    Ident(String),
    Num(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParamType {
    Str,
    Num,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefKind {
    Message,
    Term,
    Attribute,
}
