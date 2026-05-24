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
pub struct LocaleEntries {
    pub messages: BTreeMap<String, Message>,
    pub terms: BTreeMap<String, Term>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Element {
    Text(String),
    VarRef(String),
    MessageRef(String),
    TermRef(String),
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
}
