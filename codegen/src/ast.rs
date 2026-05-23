pub struct Message {
    pub name: String,
    pub elements: Vec<PatternElement>,
}

pub enum PatternElement {
    Text(String),
    VarRef(String),
    Select {
        selector: String,
        variants: Vec<Variant>,
    },
}

pub struct Variant {
    pub key: KeyType,
    pub elements: Vec<PatternElement>,
    pub default: bool,
}

pub enum KeyType {
    Ident(String),
    Num(String),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParamType {
    Str,
    Num,
}
