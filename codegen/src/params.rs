use crate::ast::*;
use std::collections::BTreeMap;

const NUMERIC_PLURAL_CATEGORIES: &[&str] = &["zero", "one", "two", "few", "many"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParamUsage {
    Interpolation,
    SelectorStr,
    SelectorNum,
}

pub fn collect_params_with_context(
    elements: &[Element],
    context: &str,
) -> BTreeMap<String, ParamType> {
    let mut map = BTreeMap::new();
    let mut usage = BTreeMap::new();
    collect_params_into(elements, &mut map, &mut usage, context);
    map
}

fn collect_params_into(
    elements: &[Element],
    map: &mut BTreeMap<String, ParamType>,
    usage: &mut BTreeMap<String, ParamUsage>,
    context: &str,
) {
    for e in elements {
        match e {
            Element::Text(_) | Element::MessageRef(_) => {}
            Element::TermRef { args, .. } => {
                for value in args.values() {
                    collect_params_into(std::slice::from_ref(value), map, usage, context);
                }
            }
            Element::VarRef(name) => {
                register_usage(name, ParamUsage::Interpolation, map, usage, context)
            }
            Element::Select { selector, variants } => {
                let selector_usage = match infer_selector_type(variants) {
                    ParamType::Str => ParamUsage::SelectorStr,
                    ParamType::Num => ParamUsage::SelectorNum,
                };
                register_usage(selector, selector_usage, map, usage, context);
                for v in variants {
                    collect_params_into(&v.elements, map, usage, context);
                }
            }
        }
    }
}

fn register_usage(
    name: &str,
    new_usage: ParamUsage,
    map: &mut BTreeMap<String, ParamType>,
    usage: &mut BTreeMap<String, ParamUsage>,
    context: &str,
) {
    match usage.get(name).copied() {
        None => {
            usage.insert(name.to_string(), new_usage);
            map.insert(name.to_string(), usage_to_type(new_usage));
        }
        Some(existing) => {
            let merged = merge_usage(existing, new_usage, name, context);
            usage.insert(name.to_string(), merged);
            map.insert(name.to_string(), usage_to_type(merged));
        }
    }
}

fn merge_usage(
    existing: ParamUsage,
    new_usage: ParamUsage,
    name: &str,
    context: &str,
) -> ParamUsage {
    use ParamUsage::*;
    match (existing, new_usage) {
        (Interpolation, Interpolation) => Interpolation,
        (Interpolation, SelectorStr)
        | (SelectorStr, Interpolation)
        | (SelectorStr, SelectorStr) => SelectorStr,
        (Interpolation, SelectorNum)
        | (SelectorNum, Interpolation)
        | (SelectorNum, SelectorNum) => SelectorNum,
        (SelectorStr, SelectorNum) | (SelectorNum, SelectorStr) => {
            panic!(
                "Parameter '{}' has conflicting inferred types in {}: {:?} vs {:?}",
                name,
                context,
                usage_to_type(existing),
                usage_to_type(new_usage)
            );
        }
    }
}

fn usage_to_type(usage: ParamUsage) -> ParamType {
    match usage {
        ParamUsage::Interpolation | ParamUsage::SelectorStr => ParamType::Str,
        ParamUsage::SelectorNum => ParamType::Num,
    }
}

fn infer_selector_type(variants: &[Variant]) -> ParamType {
    if variants.iter().any(|v| matches!(v.key, KeyType::Num(_))) {
        return ParamType::Num;
    }
    if variants.iter().any(|v| match &v.key {
        KeyType::Ident(name) => NUMERIC_PLURAL_CATEGORIES.contains(&name.as_str()),
        KeyType::Num(_) => false,
    }) {
        return ParamType::Num;
    }
    ParamType::Str
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn collect_params_simple() {
        let elems = [
            Element::Text("a".into()),
            Element::VarRef("name".into()),
            Element::Text("b".into()),
        ];
        let map = collect_params_with_context(&elems, "test");
        assert_eq!(map.len(), 1);
        assert_eq!(map["name"], ParamType::Str);
    }

    #[test]
    fn collect_params_selector_overrides() {
        let elems = [
            Element::VarRef("x".into()),
            Element::Select {
                selector: "x".into(),
                variants: vec![Variant {
                    key: KeyType::Ident("one".into()),
                    elements: vec![Element::Text("a".into())],
                    default: true,
                }],
            },
        ];
        let map = collect_params_with_context(&elems, "test");
        assert_eq!(map["x"], ParamType::Num);
    }

    #[test]
    fn infer_string_selector() {
        let elems = [Element::Select {
            selector: "gender".into(),
            variants: vec![
                Variant {
                    key: KeyType::Ident("male".into()),
                    elements: vec![Element::Text("a".into())],
                    default: false,
                },
                Variant {
                    key: KeyType::Ident("other".into()),
                    elements: vec![Element::Text("b".into())],
                    default: true,
                },
            ],
        }];
        let map = collect_params_with_context(&elems, "test");
        assert_eq!(map["gender"], ParamType::Str);
    }

    #[test]
    #[should_panic(expected = "conflicting inferred types")]
    fn detect_selector_type_conflict() {
        let elems = [
            Element::Select {
                selector: "x".into(),
                variants: vec![
                    Variant {
                        key: KeyType::Ident("male".into()),
                        elements: vec![Element::Text("a".into())],
                        default: false,
                    },
                    Variant {
                        key: KeyType::Ident("other".into()),
                        elements: vec![Element::Text("b".into())],
                        default: true,
                    },
                ],
            },
            Element::Select {
                selector: "x".into(),
                variants: vec![
                    Variant {
                        key: KeyType::Num("0".into()),
                        elements: vec![Element::Text("c".into())],
                        default: false,
                    },
                    Variant {
                        key: KeyType::Ident("other".into()),
                        elements: vec![Element::Text("d".into())],
                        default: true,
                    },
                ],
            },
        ];
        let _ = collect_params_with_context(&elems, "test message");
    }
}
