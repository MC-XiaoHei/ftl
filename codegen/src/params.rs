use crate::ast::*;
use std::collections::BTreeMap;

const NUMERIC_PLURAL_CATEGORIES: &[&str] = &["zero", "one", "two", "few", "many"];

#[derive(Clone, Debug, PartialEq, Eq)]
enum ParamUsage {
    Interpolation,
    SelectorStr,
    SelectorNum,
    Builtin(String),
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
            Element::Text(_) | Element::MessageRef(_) | Element::AttributeRef { .. } => {}
            Element::TermAttrSelect { variants, .. } => {
                for v in variants {
                    collect_params_into(&v.elements, map, usage, context);
                }
            }
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
                    ParamType::Builtin(_) => unreachable!(),
                };
                register_usage(selector, selector_usage, map, usage, context);
                for v in variants {
                    collect_params_into(&v.elements, map, usage, context);
                }
            }
            Element::BuiltInCall {
                func_name,
                var_name,
                ty_name,
                ..
            } if func_name == "NUMBER" || func_name == "DATETIME" => {
                register_usage(var_name, ParamUsage::SelectorNum, map, usage, context);
            }
            Element::BuiltInCall {
                var_name, ty_name, ..
            } => {
                register_usage(
                    var_name,
                    ParamUsage::Builtin(ty_name.clone()),
                    map,
                    usage,
                    context,
                );
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
    match usage.get(name) {
        None => {
            usage.insert(name.to_string(), new_usage.clone());
            map.insert(name.to_string(), usage_to_type(&new_usage));
        }
        Some(existing) => {
            let merged = merge_usage(existing.clone(), new_usage, name, context);
            usage.insert(name.to_string(), merged.clone());
            map.insert(name.to_string(), usage_to_type(&merged));
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
    match (&existing, &new_usage) {
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
                usage_to_type(&existing),
                usage_to_type(&new_usage)
            );
        }
        (Builtin(a), Builtin(b)) if a == b => Builtin(a.clone()),
        (Builtin(a), Builtin(b)) => {
            panic!(
                "Parameter '{}' has conflicting built-in types '{}' and '{}' in {}",
                name, a, b, context
            );
        }
        (Builtin(_), _other) | (_other, Builtin(_)) => {
            panic!(
                "Parameter '{}' has conflicting inferred types in {}: {:?} vs {:?}",
                name,
                context,
                usage_to_type(&existing),
                usage_to_type(&new_usage)
            );
        }
    }
}

fn usage_to_type(usage: &ParamUsage) -> ParamType {
    match usage {
        ParamUsage::Interpolation | ParamUsage::SelectorStr => ParamType::Str,
        ParamUsage::SelectorNum => ParamType::Num,
        ParamUsage::Builtin(ty) => ParamType::Builtin(ty.clone()),
    }
}

fn infer_selector_type(variants: &[Variant]) -> ParamType {
    if variants.iter().any(|v| matches!(v.key, KeyType::Num(_))) {
        return ParamType::Num;
    }
    if variants.iter().any(|v| matches!(&v.key, KeyType::Ident(name) if NUMERIC_PLURAL_CATEGORIES.contains(&name.as_str()))) {
        return ParamType::Num;
    }
    // All variants have Ident keys but not numeric categories → string selector
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
    fn collect_params_interpolation_and_string_selector() {
        // Variable used as interpolation ({ $x }) AND string selector
        // merges to SelectorStr → ParamType::Str
        let elems = [
            Element::VarRef("x".into()),
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
        ];
        let map = collect_params_with_context(&elems, "test");
        assert_eq!(map["x"], ParamType::Str);
    }

    #[test]
    fn collect_params_string_selector_then_interpolation() {
        // Reverse order: SelectorStr first, then Interpolation
        let elems = [
            Element::Select {
                selector: "x".into(),
                variants: vec![Variant {
                    key: KeyType::Ident("male".into()),
                    elements: vec![Element::Text("a".into())],
                    default: true,
                }],
            },
            Element::VarRef("x".into()),
        ];
        let map = collect_params_with_context(&elems, "test");
        assert_eq!(map["x"], ParamType::Str);
    }

    #[test]
    fn builtin_call_custom_function() {
        let elems = [Element::BuiltInCall {
            func_name: "HTML".to_string(),
            var_name: "input".to_string(),
            ty_name: "Html".to_string(),
            named_args: [("class".to_string(), "btn".to_string())].into(),
        }];
        let map = collect_params_with_context(&elems, "test");
        assert_eq!(map["input"], ParamType::Builtin("Html".to_string()));
    }

    #[test]
    fn builtin_call_number_infers_num_parameter() {
        let elems = [Element::BuiltInCall {
            func_name: "NUMBER".to_string(),
            var_name: "count".to_string(),
            ty_name: "".to_string(),
            named_args: BTreeMap::new(),
        }];
        let map = collect_params_with_context(&elems, "test");
        assert_eq!(map["count"], ParamType::Num);
    }

    #[test]
    fn builtin_call_datetime_infers_num_parameter() {
        let elems = [Element::BuiltInCall {
            func_name: "DATETIME".to_string(),
            var_name: "date".to_string(),
            ty_name: "".to_string(),
            named_args: BTreeMap::new(),
        }];
        let map = collect_params_with_context(&elems, "test");
        assert_eq!(map["date"], ParamType::Num);
    }

    #[test]
    fn term_ref_propagates_args() {
        let elems = [
            Element::TermRef {
                name: "link".to_string(),
                attribute: None,
                args: [("url".to_string(), Element::VarRef("url".to_string()))].into(),
                positional: vec![],
            },
            Element::VarRef("url".to_string()),
        ];
        let map = collect_params_with_context(&elems, "test");
        assert_eq!(map["url"], ParamType::Str);
    }

    #[test]
    fn term_attr_select_propagates_elements() {
        let elems = [Element::TermAttrSelect {
            term: "link".to_string(),
            attr: "text".to_string(),
            variants: vec![Variant {
                key: KeyType::Ident("text".into()),
                elements: vec![Element::VarRef("label".into())],
                default: true,
            }],
        }];
        let map = collect_params_with_context(&elems, "test");
        assert_eq!(map["label"], ParamType::Str);
    }

    #[test]
    fn infer_selector_type_num_via_digit_key() {
        let variants = vec![
            Variant {
                key: KeyType::Num("1".into()),
                elements: vec![Element::Text("one".into())],
                default: false,
            },
            Variant {
                key: KeyType::Ident("other".into()),
                elements: vec![Element::Text("many".into())],
                default: true,
            },
        ];
        assert_eq!(infer_selector_type(&variants), ParamType::Num);
    }

    #[test]
    fn infer_selector_type_ident_but_non_numeric_is_str() {
        let variants = vec![
            Variant {
                key: KeyType::Ident("male".into()),
                elements: vec![Element::Text("m".into())],
                default: false,
            },
            Variant {
                key: KeyType::Ident("female".into()),
                elements: vec![Element::Text("f".into())],
                default: false,
            },
            Variant {
                key: KeyType::Ident("other".into()),
                elements: vec![Element::Text("o".into())],
                default: true,
            },
        ];
        assert_eq!(infer_selector_type(&variants), ParamType::Str);
    }

    #[test]
    fn builtin_merge_same_type() {
        let elems = [
            Element::BuiltInCall {
                func_name: "HTML".to_string(),
                var_name: "input".to_string(),
                ty_name: "Html".to_string(),
                named_args: [("class".to_string(), "btn".to_string())].into(),
            },
            Element::BuiltInCall {
                func_name: "HTML".to_string(),
                var_name: "input".to_string(),
                ty_name: "Html".to_string(),
                named_args: [("id".to_string(), "x".to_string())].into(),
            },
        ];
        let map = collect_params_with_context(&elems, "test");
        assert_eq!(map["input"], ParamType::Builtin("Html".to_string()));
    }

    #[test]
    #[should_panic(expected = "conflicting inferred types")]
    fn builtin_vs_interpolation_conflict() {
        // BuiltInCall first → existing = Builtin, new = Interpolation
        // hits (Builtin(_), _other)
        let elems = [
            Element::BuiltInCall {
                func_name: "HTML".to_string(),
                var_name: "input".to_string(),
                ty_name: "Html".to_string(),
                named_args: BTreeMap::new(),
            },
            Element::VarRef("input".to_string()),
        ];
        let _ = collect_params_with_context(&elems, "test");
    }

    #[test]
    #[should_panic(expected = "conflicting inferred types")]
    fn interpolation_vs_builtin_conflict() {
        // VarRef first → existing = Interpolation, new = Builtin
        // hits (_other, Builtin(_))
        let elems = [
            Element::VarRef("input".to_string()),
            Element::BuiltInCall {
                func_name: "HTML".to_string(),
                var_name: "input".to_string(),
                ty_name: "Html".to_string(),
                named_args: BTreeMap::new(),
            },
        ];
        let _ = collect_params_with_context(&elems, "test");
    }

    #[test]
    #[should_panic(expected = "conflicting built-in types")]
    fn detect_builtin_type_conflict() {
        let elems = [
            Element::BuiltInCall {
                func_name: "CUSTOM_A".to_string(),
                var_name: "x".to_string(),
                ty_name: "CustomA".to_string(),
                named_args: BTreeMap::new(),
            },
            Element::BuiltInCall {
                func_name: "CUSTOM_B".to_string(),
                var_name: "x".to_string(),
                ty_name: "CustomB".to_string(),
                named_args: BTreeMap::new(),
            },
        ];
        let _ = collect_params_with_context(&elems, "test");
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
