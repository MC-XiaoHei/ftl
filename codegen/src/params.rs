use crate::ast::*;
use std::collections::BTreeMap;

pub fn collect_params(elements: &[PatternElement]) -> BTreeMap<String, ParamType> {
    let mut map = BTreeMap::new();
    collect_params_into(elements, &mut map);
    map
}

fn collect_params_into(elements: &[PatternElement], map: &mut BTreeMap<String, ParamType>) {
    for e in elements {
        match e {
            PatternElement::Text(_) => {}
            PatternElement::VarRef(name) => {
                map.entry(name.clone()).or_insert(ParamType::Str);
            }
            PatternElement::Select { selector, variants } => {
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
            PatternElement::Text("a".into()),
            PatternElement::VarRef("name".into()),
            PatternElement::Text("b".into()),
        ];
        let map = collect_params(&elems);
        assert_eq!(map.len(), 1);
        assert_eq!(map["name"], ParamType::Str);
    }

    #[test]
    fn collect_params_selector_overrides() {
        let elems = [
            PatternElement::VarRef("x".into()),
            PatternElement::Select {
                selector: "x".into(),
                variants: vec![Variant {
                    key: KeyType::Ident("one".into()),
                    elements: vec![PatternElement::Text("a".into())],
                    default: true,
                }],
            },
        ];
        let map = collect_params(&elems);
        assert_eq!(map["x"], ParamType::Num);
    }
}
