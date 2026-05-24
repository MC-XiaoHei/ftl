use crate::ast::*;
use std::collections::BTreeMap;

pub fn collect_params(elements: &[Element]) -> BTreeMap<String, ParamType> {
    let mut map = BTreeMap::new();
    collect_params_into(elements, &mut map);
    map
}

fn collect_params_into(elements: &[Element], map: &mut BTreeMap<String, ParamType>) {
    for e in elements {
        match e {
            Element::Text(_) | Element::MessageRef(_) => {}
            Element::TermRef { args, .. } => {
                for value in args.values() {
                    collect_params_into(std::slice::from_ref(value), map);
                }
            }
            Element::VarRef(name) => {
                map.entry(name.clone()).or_insert(ParamType::Str);
            }
            Element::Select { selector, variants } => {
                map.insert(selector.clone(), ParamType::Num);
                for v in variants {
                    collect_params_into(&v.elements, map);
                }
            }
        }
    }
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
        let map = collect_params(&elems);
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
        let map = collect_params(&elems);
        assert_eq!(map["x"], ParamType::Num);
    }
}
