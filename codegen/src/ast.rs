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
        positional: Vec<Element>,
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
    /// Call to a registered built-in function like `NUMBER($x, ...)`.
    BuiltInCall {
        /// Function name in FTL (e.g. "NUMBER").
        func_name: String,
        /// The Rust type name (e.g. "Number").
        ty_name: String,
        /// The variable name being passed (e.g. "x" from $x).
        var_name: String,
        /// Named arguments: FTL camelCase name -> raw value as string.
        named_args: BTreeMap<String, String>,
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
    /// A registered built-in type (e.g. "Number", "DateTime").
    Builtin(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefKind {
    Message,
    Term,
    Attribute,
}
