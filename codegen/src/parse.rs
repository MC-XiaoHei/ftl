use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use fluent_syntax::ast::{self, Entry, Expression, InlineExpression, PatternElement, VariantKey};
use fluent_syntax::parser::{self, ParserError};
use indoc::formatdoc;

use crate::ast::*;
use crate::diag::{report_diagnostics, Diag, DiagKind};
use crate::fmt::generate_one_function;
use crate::params::collect_params_with_context;
use crate::util::{sanitize, sanitize_upper};

pub struct Generator {
    pub primary: String,
    pub locales: BTreeMap<String, LocaleEntries>,
    pub diags: Vec<Diag>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VisitState {
    Visiting,
    Done,
}

struct Resolver<'a> {
    locale: &'a str,
    file: &'a str,
    entries: &'a LocaleEntries,
    messages: BTreeMap<String, Message>,
    terms: BTreeMap<String, Term>,
    attributes: BTreeMap<String, Attribute>,
    message_states: BTreeMap<String, VisitState>,
    term_states: BTreeMap<String, VisitState>,
    attribute_states: BTreeMap<String, VisitState>,
    stack: Vec<(RefKind, String)>,
    env_stack: Vec<BTreeMap<String, Element>>,
}

impl Generator {
    pub fn load(dir: &Path, primary: &str) -> Self {
        let mut diags = Vec::new();
        let mut file_map = BTreeMap::new();
        let mut locales = BTreeMap::new();
        for entry in fs::read_dir(dir).expect("Cannot read locales directory") {
            let entry = entry.expect("invalid directory entry");
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("ftl") {
                continue;
            }
            let locale = path
                .file_stem()
                .expect("file has no stem")
                .to_str()
                .expect("filename is not UTF-8")
                .to_string();

            // locale name is valid Unicode Language Identifier
            if unic_langid::LanguageIdentifier::from_str(&locale).is_err() {
                diags.push(Diag::error(
                    path.to_string_lossy(),
                    &locale,
                    "",
                    format!(
                        "'{}' is not a valid Unicode Language Identifier (expected e.g. en-US, zh-CN)",
                        locale
                    ),
                ));
                continue;
            }

            let file_path = path.to_string_lossy().to_string();
            file_map.insert(locale.clone(), file_path);
            let source = match fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    diags.push(Diag::error(
                        path.to_string_lossy(),
                        &locale,
                        "",
                        format!("cannot read file: {}", e),
                    ));
                    continue;
                }
            };
            let file_display = path.to_string_lossy();
            match parser::parse(source.as_str()) {
                Ok(r) => {
                    locales.insert(locale, Self::extract(&r));
                }
                Err((partial, errors)) => {
                    for err in &errors {
                        diags.push(format_parse_error(&source, &file_display, &locale, err));
                    }
                    // use partial result even on parse errors
                    locales.insert(locale, Self::extract(&partial));
                }
            }
        }
        if !locales.contains_key(primary) {
            diags.push(Diag::error(
                "",
                primary,
                "",
                format!("primary locale '{}' not found", primary),
            ));
        }
        let primary_entries = match locales.get(primary) {
            Some(e) => e,
            None => {
                report_diagnostics(&diags);
                unreachable!()
            }
        };
        let primary_message_keys: BTreeSet<&str> = primary_entries
            .messages
            .keys()
            .map(|k| k.as_str())
            .collect();
        let primary_term_keys: BTreeSet<&str> =
            primary_entries.terms.keys().map(|k| k.as_str()).collect();
        let primary_attribute_keys: BTreeSet<&str> = primary_entries
            .attributes
            .keys()
            .map(|k| k.as_str())
            .collect();

        for (name, entries) in &locales {
            if name == primary {
                continue;
            }
            let locale_message_keys: BTreeSet<&str> =
                entries.messages.keys().map(|k| k.as_str()).collect();
            let extra_msgs: Vec<&&str> = locale_message_keys
                .difference(&primary_message_keys)
                .collect();
            if !extra_msgs.is_empty() {
                diags.push(Diag::error(
                    file_map.get(name).cloned().unwrap_or_default(),
                    name,
                    "",
                    format!(
                        "has extra messages not present in primary locale '{}': {:?}",
                        primary, extra_msgs
                    ),
                ));
            }
            let locale_term_keys: BTreeSet<&str> =
                entries.terms.keys().map(|k| k.as_str()).collect();
            let extra_terms: Vec<&&str> = locale_term_keys.difference(&primary_term_keys).collect();
            if !extra_terms.is_empty() {
                diags.push(Diag::error(
                    file_map.get(name).cloned().unwrap_or_default(),
                    name,
                    "",
                    format!(
                        "has extra terms not present in primary locale '{}': {:?}",
                        primary, extra_terms
                    ),
                ));
            }
            let locale_attr_keys: BTreeSet<&str> =
                entries.attributes.keys().map(|k| k.as_str()).collect();
            let extra_attrs: Vec<&&str> = locale_attr_keys
                .difference(&primary_attribute_keys)
                .collect();
            if !extra_attrs.is_empty() {
                diags.push(Diag::error(
                    file_map.get(name).cloned().unwrap_or_default(),
                    name,
                    "",
                    format!(
                        "has extra attributes not present in primary locale '{}': {:?}",
                        primary, extra_attrs
                    ),
                ));
            }
        }
        if diags.iter().any(|d| d.kind == DiagKind::Error) {
            report_diagnostics(&diags);
        }

        let mut resolved_locales = BTreeMap::new();
        for (locale, entries) in locales {
            let mut resolver = Resolver::new(
                &locale,
                file_map.get(&locale).map(|s| s.as_str()).unwrap_or(""),
                &entries,
            );
            let messages = resolver.resolve_all_messages();
            let terms = resolver.resolve_all_terms();
            let attributes = resolver.resolve_all_attributes();
            resolved_locales.insert(
                locale,
                LocaleEntries {
                    messages,
                    terms,
                    attributes,
                },
            );
        }
        Generator {
            primary: primary.to_string(),
            locales: resolved_locales,
            diags,
        }
    }

    fn extract(resource: &ast::Resource<&str>) -> LocaleEntries {
        let mut messages = BTreeMap::new();
        let mut terms = BTreeMap::new();
        let mut attributes = BTreeMap::new();
        for entry in &resource.body {
            match entry {
                Entry::Message(msg) => {
                    let owner = msg.id.name.to_string();
                    if let Some(pattern) = &msg.value {
                        messages.insert(
                            owner.clone(),
                            Message {
                                name: owner.clone(),
                                elements: convert_elements(&pattern.elements),
                            },
                        );
                    }
                    for attr in &msg.attributes {
                        let attr_name = attr.id.name.to_string();
                        attributes.insert(
                            flatten_attr_name(&owner, &attr_name),
                            Attribute {
                                owner: owner.clone(),
                                name: attr_name,
                                elements: convert_elements(&attr.value.elements),
                            },
                        );
                    }
                }
                Entry::Term(term) => {
                    let owner = term.id.name.to_string();
                    terms.insert(
                        owner.clone(),
                        Term {
                            name: owner.clone(),
                            elements: convert_elements(&term.value.elements),
                        },
                    );
                    for attr in &term.attributes {
                        let attr_name = attr.id.name.to_string();
                        attributes.insert(
                            flatten_attr_name(&owner, &attr_name),
                            Attribute {
                                owner: owner.clone(),
                                name: attr_name,
                                elements: convert_elements(&attr.value.elements),
                            },
                        );
                    }
                }
                _ => {}
            }
        }
        LocaleEntries {
            messages,
            terms,
            attributes,
        }
    }

    pub fn generate(&self) -> String {
        let mut out = String::new();
        let locales: Vec<&String> = self.locales.keys().collect();
        writeln!(out, "// Auto-generated by ftl-codegen").unwrap();
        writeln!(out, "// Primary language: {}", self.primary).unwrap();
        writeln!(out, "#[allow(non_upper_case_globals, unused)]").unwrap();
        writeln!(out).unwrap();
        self.emit_fluent_num(&mut out);
        for locale in &locales {
            self.emit_module(locale, &mut out);
        }
        self.emit_runtime(&locales, &mut out);
        writeln!(out, "#[macro_export]").unwrap();
        writeln!(out, "macro_rules! t {{").unwrap();
        writeln!(out, "    ($key:ident) => {{").unwrap();
        writeln!(
            out,
            "        match $crate::LOCALE_ID.load(Ordering::Relaxed) {{"
        )
        .unwrap();
        for (idx, locale) in locales.iter().enumerate() {
            let mn = sanitize(locale);
            writeln!(out, "            {} => $crate::{}::$key(),", idx, mn).unwrap();
        }
        writeln!(
            out,
            "            _ => $crate::{}::$key(),",
            sanitize(&self.primary)
        )
        .unwrap();
        writeln!(out, "        }}").unwrap();
        writeln!(out, "    }};").unwrap();
        writeln!(out, "    ($key:ident($($args:expr),* $(,)?)) => {{").unwrap();
        writeln!(
            out,
            "        match $crate::LOCALE_ID.load(Ordering::Relaxed) {{"
        )
        .unwrap();
        for (idx, locale) in locales.iter().enumerate() {
            let mn = sanitize(locale);
            writeln!(
                out,
                "            {} => $crate::{}::$key($($args),*),",
                idx, mn
            )
            .unwrap();
        }
        writeln!(
            out,
            "            _ => $crate::{}::$key($($args),*),",
            sanitize(&self.primary)
        )
        .unwrap();
        writeln!(out, "        }}").unwrap();
        writeln!(out, "    }};").unwrap();
        writeln!(out, "}}").unwrap();
        out
    }

    fn emit_module(&self, locale: &str, out: &mut String) {
        let mod_name = sanitize(locale);
        writeln!(out, "pub mod {} {{", mod_name).unwrap();
        writeln!(out, "    #![allow(non_snake_case)]").unwrap();
        writeln!(out, "    use std::fmt::Write;").unwrap();
        writeln!(out, "    use super::FluentNum;").unwrap();
        let le = &self.locales[locale];
        let pe = &self.locales[&self.primary];
        let lmsg: BTreeSet<&str> = le.messages.keys().map(|k| k.as_str()).collect();
        let lattr: BTreeSet<&str> = le.attributes.keys().map(|k| k.as_str()).collect();
        for (name, p_msg) in &pe.messages {
            let params =
                collect_params_with_context(&p_msg.elements, &format!("message '{}'", p_msg.name));
            if lmsg.contains(name.as_str()) {
                let msg = le.messages.get(name).expect("message missing from locale");
                writeln!(
                    out,
                    "{}",
                    generate_one_function(&msg.name, &msg.elements, &params, locale).trim_end()
                )
                .unwrap();
            } else {
                writeln!(
                    out,
                    "{}",
                    generate_one_function(&p_msg.name, &p_msg.elements, &params, &self.primary)
                        .trim_end()
                )
                .unwrap();
                writeln!(
                    out,
                    "// WARNING: '{}' missing, using '{}' fallback",
                    p_msg.name, locale
                )
                .unwrap();
            }
        }
        for (flat, p_attr) in &pe.attributes {
            let params = collect_params_with_context(
                &p_attr.elements,
                &format!("attribute '{}.{}'", p_attr.owner, p_attr.name),
            );
            let fn_name = flatten_attr_name(&p_attr.owner, &p_attr.name);
            if lattr.contains(flat.as_str()) {
                let attr = le
                    .attributes
                    .get(flat)
                    .expect("attribute missing from locale");
                writeln!(
                    out,
                    "{}",
                    generate_one_function(&fn_name, &attr.elements, &params, locale).trim_end()
                )
                .unwrap();
            } else {
                writeln!(
                    out,
                    "{}",
                    generate_one_function(&fn_name, &p_attr.elements, &params, &self.primary)
                        .trim_end()
                )
                .unwrap();
                writeln!(
                    out,
                    "// WARNING: attribute '{}.{}' missing, using '{}' fallback",
                    p_attr.owner, p_attr.name, locale
                )
                .unwrap();
            }
        }
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }

    fn emit_fluent_num(&self, out: &mut String) {
        writeln!(out, "pub struct FluentNum(f64);").unwrap();
        writeln!(out).unwrap();
        let from_types: [(&str, &str); 12] = [
            ("usize", "v as f64"),
            ("u64", "v as f64"),
            ("u32", "v as f64"),
            ("u16", "v as f64"),
            ("u8", "v as f64"),
            ("i64", "v as f64"),
            ("i32", "v as f64"),
            ("i16", "v as f64"),
            ("i8", "v as f64"),
            ("isize", "v as f64"),
            ("f64", "v"),
            ("f32", "v as f64"),
        ];
        for (ty, conv) in &from_types {
            writeln!(
                out,
                "impl From<{ty}> for FluentNum {{ fn from(v: {ty}) -> Self {{ Self({conv}) }} }}"
            )
            .unwrap();
        }
        writeln!(out).unwrap();
        writeln!(
            out,
            "{}",
            formatdoc!(
                "
                impl PartialEq<f64> for FluentNum {{
                    fn eq(&self, other: &f64) -> bool {{
                        self.0 == *other
                    }}
                }}
            "
            )
        )
        .unwrap();
        writeln!(out).unwrap();
        writeln!(
            out,
            "{}",
            formatdoc!(
                "
                impl std::ops::Deref for FluentNum {{
                    type Target = f64;
                    fn deref(&self) -> &f64 {{ &self.0 }}
                }}
            "
            )
        )
        .unwrap();
        writeln!(out).unwrap();
        writeln!(
            out,
            "{}",
            formatdoc!(
                "
                impl FluentNum {{
                    pub fn operands(self) -> Operands {{
                        let i = self.0.trunc() as u64;
                        Operands {{ n: self.0, i, v: 0, w: 0, f: 0, t: 0 }}
                    }}
                }}
            "
            )
        )
        .unwrap();
        writeln!(out).unwrap();
        writeln!(
            out,
            "{}",
            formatdoc!(
                "
                pub struct Operands {{
                    pub n: f64,
                    pub i: u64,
                    pub v: u8,
                    pub w: u8,
                    pub f: u64,
                    pub t: u64,
                }}
            "
            )
        )
        .unwrap();
        writeln!(out).unwrap();
        writeln!(
            out,
            "{}",
            formatdoc!(
                "
                impl core::fmt::Display for FluentNum {{
                    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {{
                        write!(f, \"{{}}\", self.0)
                    }}
                }}
            "
            )
        )
        .unwrap();
    }

    fn emit_runtime(&self, locales: &[&String], out: &mut String) {
        writeln!(out, "use std::sync::atomic::{{AtomicU8, Ordering}};").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "use unic_langid::{{LanguageIdentifier, langid}};").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "static LOCALE_ID: AtomicU8 = AtomicU8::new(0);").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "pub enum Lang {{").unwrap();
        for locale in locales {
            writeln!(out, "    {},", sanitize_upper(locale)).unwrap();
        }
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "impl From<Lang> for LanguageIdentifier {{").unwrap();
        writeln!(out, "    fn from(l: Lang) -> Self {{").unwrap();
        writeln!(out, "        match l {{").unwrap();
        for locale in locales {
            writeln!(
                out,
                "            Lang::{} => langid!(\"{}\"),",
                sanitize_upper(locale),
                locale
            )
            .unwrap();
        }
        writeln!(out, "        }}").unwrap();
        writeln!(out, "    }}").unwrap();
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "pub fn get_locale() -> Lang {{").unwrap();
        writeln!(out, "    match LOCALE_ID.load(Ordering::Acquire) {{").unwrap();
        for (idx, locale) in locales.iter().enumerate() {
            writeln!(out, "        {} => Lang::{},", idx, sanitize_upper(locale)).unwrap();
        }
        writeln!(out, "        _ => Lang::{},", sanitize_upper(&self.primary)).unwrap();
        writeln!(out, "    }}").unwrap();
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "pub fn set_lang(lang: Lang) {{").unwrap();
        writeln!(out, "    let id = match lang {{").unwrap();
        for (idx, locale) in locales.iter().enumerate() {
            writeln!(out, "        Lang::{} => {},", sanitize_upper(locale), idx).unwrap();
        }
        writeln!(out, "    }};").unwrap();
        writeln!(out, "    LOCALE_ID.store(id, Ordering::Release);").unwrap();
        writeln!(out, "}}").unwrap();
    }
}

impl<'a> Resolver<'a> {
    fn new(locale: &'a str, file: &'a str, entries: &'a LocaleEntries) -> Self {
        Self {
            locale,
            file,
            entries,
            messages: BTreeMap::new(),
            terms: BTreeMap::new(),
            attributes: BTreeMap::new(),
            message_states: BTreeMap::new(),
            term_states: BTreeMap::new(),
            attribute_states: BTreeMap::new(),
            stack: Vec::new(),
            env_stack: Vec::new(),
        }
    }
    fn resolve_all_messages(&mut self) -> BTreeMap<String, Message> {
        let names: Vec<String> = self.entries.messages.keys().cloned().collect();
        for name in names {
            self.resolve_message(&name);
        }
        self.messages.clone()
    }
    fn resolve_all_terms(&mut self) -> BTreeMap<String, Term> {
        let names: Vec<String> = self.entries.terms.keys().cloned().collect();
        for name in names {
            self.resolve_term(&name);
        }
        self.terms.clone()
    }
    fn resolve_all_attributes(&mut self) -> BTreeMap<String, Attribute> {
        let names: Vec<String> = self.entries.attributes.keys().cloned().collect();
        for name in names {
            self.resolve_attribute(&name);
        }
        self.attributes.clone()
    }
    fn resolve_message(&mut self, name: &str) -> Message {
        if let Some(msg) = self.messages.get(name) {
            return msg.clone();
        }
        match self.message_states.get(name).copied() {
            Some(VisitState::Visiting) => self.cycle_panic(RefKind::Message, name),
            Some(VisitState::Done) => {
                return self
                    .messages
                    .get(name)
                    .expect("resolved message missing from cache")
                    .clone()
            }
            None => {}
        }
        let raw = self
            .entries
            .messages
            .get(name)
            .unwrap_or_else(|| {
                panic!(
                    "{}: [{}] undefined message reference '{}'",
                    self.file, self.locale, name
                )
            })
            .clone();
        self.message_states
            .insert(name.to_string(), VisitState::Visiting);
        self.stack.push((RefKind::Message, name.to_string()));
        let elements = self.resolve_elements(&raw.elements);
        self.stack.pop();
        self.message_states
            .insert(name.to_string(), VisitState::Done);
        let resolved = Message {
            name: raw.name,
            elements: fold_text(elements),
        };
        self.messages.insert(name.to_string(), resolved.clone());
        resolved
    }
    fn resolve_term(&mut self, name: &str) -> Term {
        if let Some(term) = self.terms.get(name) {
            return term.clone();
        }
        match self.term_states.get(name).copied() {
            Some(VisitState::Visiting) => self.cycle_panic(RefKind::Term, name),
            Some(VisitState::Done) => {
                return self
                    .terms
                    .get(name)
                    .expect("resolved term missing from cache")
                    .clone()
            }
            None => {}
        }
        let raw = self
            .entries
            .terms
            .get(name)
            .unwrap_or_else(|| {
                panic!(
                    "{}: [{}] undefined term reference '-{}'",
                    self.file, self.locale, name
                )
            })
            .clone();
        self.term_states
            .insert(name.to_string(), VisitState::Visiting);
        self.stack.push((RefKind::Term, name.to_string()));
        let elements = self.resolve_elements(&raw.elements);
        self.stack.pop();
        self.term_states.insert(name.to_string(), VisitState::Done);
        let resolved = Term {
            name: raw.name,
            elements: fold_text(elements),
        };
        self.terms.insert(name.to_string(), resolved.clone());
        resolved
    }
    fn resolve_attribute(&mut self, name: &str) -> Attribute {
        if let Some(attr) = self.attributes.get(name) {
            return attr.clone();
        }
        match self.attribute_states.get(name).copied() {
            Some(VisitState::Visiting) => self.cycle_panic(RefKind::Attribute, name),
            Some(VisitState::Done) => {
                return self
                    .attributes
                    .get(name)
                    .expect("resolved attribute missing from cache")
                    .clone()
            }
            None => {}
        }
        let raw = self
            .entries
            .attributes
            .get(name)
            .unwrap_or_else(|| {
                panic!(
                    "{}: [{}] undefined attribute reference '.{}'",
                    self.file, self.locale, name
                )
            })
            .clone();
        self.attribute_states
            .insert(name.to_string(), VisitState::Visiting);
        self.stack.push((RefKind::Attribute, name.to_string()));
        let elements = self.resolve_elements(&raw.elements);
        self.stack.pop();
        self.attribute_states
            .insert(name.to_string(), VisitState::Done);
        let resolved = Attribute {
            owner: raw.owner,
            name: raw.name,
            elements: fold_text(elements),
        };
        self.attributes.insert(name.to_string(), resolved.clone());
        resolved
    }
    fn resolve_elements(&mut self, elements: &[Element]) -> Vec<Element> {
        let mut out = Vec::new();
        for element in elements {
            match element {
                Element::Text(text) => out.push(Element::Text(text.clone())),
                Element::VarRef(name) => {
                    if let Some(bound) = self.lookup_bound_var(name) {
                        out.push(bound);
                    } else {
                        out.push(Element::VarRef(name.clone()));
                    }
                }
                Element::MessageRef(name) => {
                    let r = self.resolve_message(name);
                    out.extend(r.elements.clone());
                }
                Element::AttributeRef { owner, name } => {
                    let flat = flatten_attr_name(owner, name);
                    let r = self.resolve_attribute(&flat);
                    out.extend(r.elements.clone());
                }
                Element::TermRef {
                    name,
                    args,
                    positional,
                    ..
                } => {
                    // Free variables from term, in source order
                    let raw_term = self
                        .entries
                        .terms
                        .get(name.as_str())
                        .expect("undefined term reference");
                    let free_vars: Vec<&str> = raw_term
                        .elements
                        .iter()
                        .filter_map(|e| {
                            if let Element::VarRef(v) = e {
                                // Already bound in env_stack
                                if self.lookup_bound_var(v).is_none() {
                                    Some(v.as_str())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .collect();

                    let mut bindings = BTreeMap::new();
                    // Map positional args to free vars in order
                    for (i, pos_val) in positional.iter().enumerate() {
                        if let Some(var_name) = free_vars.get(i) {
                            bindings.insert(
                                (*var_name).to_string(),
                                self.resolve_argument_value(pos_val),
                            );
                        }
                    }
                    // Named args override positional
                    for (k, v) in args {
                        bindings.insert(k.clone(), self.resolve_argument_value(v));
                    }
                    let r = self.resolve_term_with_args(name, bindings);
                    out.extend(r.elements.clone());
                }
                Element::Select { selector, variants } => {
                    let vs = variants
                        .iter()
                        .map(|v| Variant {
                            key: v.key.clone(),
                            elements: fold_text(self.resolve_elements(&v.elements)),
                            default: v.default,
                        })
                        .collect();
                    out.push(Element::Select {
                        selector: selector.clone(),
                        variants: vs,
                    });
                }
                Element::TermAttrSelect {
                    term,
                    attr,
                    variants,
                } => {
                    let flat = flatten_attr_name(term, attr);
                    let r = self.resolve_attribute(&flat);
                    let attr_value: String = r
                        .elements
                        .iter()
                        .filter_map(|e| {
                            if let Element::Text(t) = e {
                                Some(t.as_str())
                            } else {
                                None
                            }
                        })
                        .collect();
                    let matched = variants.iter().find(|v| match &v.key {
                        KeyType::Ident(ident) => ident == &attr_value,
                        KeyType::Num(num) => num == &attr_value,
                    });
                    if let Some(variant) = matched {
                        out.extend(self.resolve_elements(&variant.elements));
                    } else if let Some(default) = variants.iter().find(|v| v.default) {
                        out.extend(self.resolve_elements(&default.elements));
                    }
                }
            }
        }
        fold_text(out)
    }
    fn resolve_term_with_args(&mut self, name: &str, args: BTreeMap<String, Element>) -> Term {
        self.env_stack.push(args);
        let r = self.resolve_term(name);
        self.env_stack.pop();
        r
    }
    fn resolve_argument_value(&mut self, value: &Element) -> Element {
        match value {
            Element::Text(text) => Element::Text(text.clone()),
            Element::VarRef(name) => self
                .lookup_bound_var(name)
                .unwrap_or_else(|| Element::VarRef(name.clone())),
            Element::MessageRef(name) => {
                let r = self.resolve_message(name);
                if r.elements.len() == 1 {
                    r.elements[0].clone()
                } else {
                    panic!(
                        "{}: [{}] term argument '{}' must resolve to a single element",
                        self.file, self.locale, name
                    );
                }
            }
            Element::AttributeRef { owner, name } => {
                let flat = flatten_attr_name(owner, name);
                let r = self.resolve_attribute(&flat);
                if r.elements.len() == 1 {
                    r.elements[0].clone()
                } else {
                    panic!(
                        "{}: [{}] term argument '{}.{}' must resolve to a single element",
                        self.file, self.locale, owner, name
                    );
                }
            }
            Element::TermRef { name, args, .. } => {
                let bindings = args
                    .iter()
                    .map(|(k, v)| (k.clone(), self.resolve_argument_value(v)))
                    .collect::<BTreeMap<_, _>>();
                let r = self.resolve_term_with_args(name, bindings);
                if r.elements.len() == 1 {
                    r.elements[0].clone()
                } else {
                    panic!(
                        "{}: [{}] parameterized term '-{}' must resolve to a single element",
                        self.file, self.locale, name
                    );
                }
            }
            Element::Select { .. } | Element::TermAttrSelect { .. } => {
                panic!("Term argument cannot be a select expression")
            }
        }
    }
    fn lookup_bound_var(&self, name: &str) -> Option<Element> {
        self.env_stack
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }
    fn cycle_panic(&self, ref_kind: RefKind, name: &str) -> ! {
        let mut chain = self
            .stack
            .iter()
            .map(|(k, e)| format!("{}{}", ref_prefix(*k), e))
            .collect::<Vec<_>>();
        chain.push(format!("{}{}", ref_prefix(ref_kind), name));
        panic!(
            "{}: [{}] cyclic {} reference: {}",
            self.file,
            self.locale,
            match ref_kind {
                RefKind::Message => "message",
                RefKind::Term => "term",
                RefKind::Attribute => "attribute",
            },
            chain.join(" -> ")
        );
    }
}

fn fold_text(elements: Vec<Element>) -> Vec<Element> {
    let mut out = Vec::new();
    for element in elements {
        match element {
            Element::Text(text) => {
                if let Some(Element::Text(last)) = out.last_mut() {
                    last.push_str(&text);
                } else if !text.is_empty() {
                    out.push(Element::Text(text));
                }
            }
            other => out.push(other),
        }
    }
    out
}

fn ref_prefix(kind: RefKind) -> &'static str {
    match kind {
        RefKind::Message => "",
        RefKind::Term => "-",
        RefKind::Attribute => ".",
    }
}
fn flatten_attr_name(owner: &str, name: &str) -> String {
    format!("{}__{}", owner, name)
}

fn convert_elements(elems: &[PatternElement<&str>]) -> Vec<Element> {
    elems.iter().map(convert_element).collect()
}
fn convert_element(e: &PatternElement<&str>) -> Element {
    match e {
        PatternElement::TextElement { value } => Element::Text(value.to_string()),
        PatternElement::Placeable { expression } => convert_expression(expression),
    }
}

/// Like convert_elements but returns a single Element, folding adjacent text.
/// Used to inline literal selectors at compile time.
fn convert_collected(elems: Vec<Element>) -> Element {
    let mut folded = fold_text(elems);
    if folded.is_empty() {
        Element::Text(String::new())
    } else if folded.len() == 1 {
        folded.swap_remove(0)
    } else {
        Element::Select {
            selector: "_resolved_literal".into(),
            variants: vec![Variant {
                key: KeyType::Ident("other".into()),
                elements: folded,
                default: true,
            }],
        }
    }
}

fn convert_expression(expr: &Expression<&str>) -> Element {
    match expr {
        Expression::Inline(inline) => match inline {
            InlineExpression::VariableReference { id } => Element::VarRef(id.name.to_string()),
            InlineExpression::MessageReference { id, attribute } => {
                if let Some(attr) = attribute {
                    Element::AttributeRef {
                        owner: id.name.to_string(),
                        name: attr.name.to_string(),
                    }
                } else {
                    Element::MessageRef(id.name.to_string())
                }
            }
            InlineExpression::TermReference {
                id,
                attribute,
                arguments,
            } => {
                let mut args_map = BTreeMap::new();
                let mut positional_args = Vec::new();
                if let Some(arguments) = arguments {
                    for arg in &arguments.positional {
                        positional_args.push(convert_inline_expression(arg));
                    }
                    for arg in &arguments.named {
                        args_map.insert(
                            arg.name.name.to_string(),
                            convert_inline_expression(&arg.value),
                        );
                    }
                }
                Element::TermRef {
                    name: id.name.to_string(),
                    attribute: attribute.as_ref().map(|a| a.name.to_string()),
                    args: args_map,
                    positional: positional_args,
                }
            }
            InlineExpression::StringLiteral { value } => Element::Text(value.to_string()),
            InlineExpression::NumberLiteral { value } => Element::Text(value.to_string()),
            other => panic!("Unsupported expression: {:?}", other),
        },
        Expression::Select { selector, variants } => {
            let (selector_str, is_literal) = match selector {
                InlineExpression::VariableReference { id } => (id.name.to_string(), false),
                InlineExpression::NumberLiteral { value } => (value.to_string(), true),
                InlineExpression::StringLiteral { value } => (value.to_string(), true),
                InlineExpression::TermReference {
                    id,
                    attribute: Some(attr),
                    ..
                } => {
                    // Term attribute reference in selector
                    let vs = variants
                        .iter()
                        .map(|v| {
                            let key = match &v.key {
                                VariantKey::Identifier { name } => KeyType::Ident(name.to_string()),
                                VariantKey::NumberLiteral { value } => {
                                    KeyType::Num(value.to_string())
                                }
                            };
                            Variant {
                                key,
                                elements: convert_elements(&v.value.elements),
                                default: v.default,
                            }
                        })
                        .collect();
                    return Element::TermAttrSelect {
                        term: id.name.to_string(),
                        attr: attr.name.to_string(),
                        variants: vs,
                    };
                }
                other => panic!(
                    "Select selector must be a variable, number, or string: {:?}",
                    other
                ),
            };

            let vs: Vec<Variant> = variants
                .iter()
                .map(|v| {
                    let key = match &v.key {
                        VariantKey::Identifier { name } => KeyType::Ident(name.to_string()),
                        VariantKey::NumberLiteral { value } => KeyType::Num(value.to_string()),
                    };
                    Variant {
                        key,
                        elements: convert_elements(&v.value.elements),
                        default: v.default,
                    }
                })
                .collect();

            // Literal selectors inlined at compile time
            if is_literal {
                let matched = vs.iter().find(|v| match &v.key {
                    KeyType::Num(val) => val == &selector_str,
                    KeyType::Ident(ident) => ident == &selector_str,
                });
                if let Some(variant) = matched {
                    return convert_collected(variant.elements.clone());
                }
                // No exact match — use default variant
                if let Some(variant) = vs.iter().find(|v| v.default) {
                    return convert_collected(variant.elements.clone());
                }
                panic!("Select expression has no matching variant and no default");
            }

            Element::Select {
                selector: selector_str,
                variants: vs,
            }
        }
    }
}

fn convert_inline_expression(expr: &InlineExpression<&str>) -> Element {
    match expr {
        InlineExpression::StringLiteral { value } => Element::Text(value.to_string()),
        InlineExpression::NumberLiteral { value } => Element::Text(value.to_string()),
        InlineExpression::VariableReference { id } => Element::VarRef(id.name.to_string()),
        InlineExpression::MessageReference { id, attribute } => {
            if let Some(attr) = attribute {
                Element::AttributeRef {
                    owner: id.name.to_string(),
                    name: attr.name.to_string(),
                }
            } else {
                Element::MessageRef(id.name.to_string())
            }
        }
        InlineExpression::TermReference {
            id,
            attribute,
            arguments,
        } => {
            let mut args_map = BTreeMap::new();
            let mut positional_args = Vec::new();
            if let Some(arguments) = arguments {
                for arg in &arguments.positional {
                    positional_args.push(convert_inline_expression(arg));
                }
                for arg in &arguments.named {
                    args_map.insert(
                        arg.name.name.to_string(),
                        convert_inline_expression(&arg.value),
                    );
                }
            }
            Element::TermRef {
                name: id.name.to_string(),
                attribute: attribute.as_ref().map(|a| a.name.to_string()),
                args: args_map,
                positional: positional_args,
            }
        }
        other => panic!(
            "Unsupported inline expression in term arguments: {:?}",
            other
        ),
    }
}

fn line_of(source: &str, offset: usize) -> usize {
    source[..offset].chars().filter(|&c| c == '\n').count() + 1
}

fn format_parse_error(source: &str, file: &str, locale: &str, error: &ParserError) -> Diag {
    let line = line_of(source, error.pos.start);
    let bol = source[..error.pos.start]
        .rfind('\n')
        .map(|p| p + 1)
        .unwrap_or(0);
    let eol = source[error.pos.start..]
        .find('\n')
        .map(|p| error.pos.start + p)
        .unwrap_or(source.len());
    let line_text = &source[bol..eol];
    let col = error.pos.start - bol + 1;
    let snippet = if line_text.len() > 78 {
        format!("{}...", &line_text[..75])
    } else {
        line_text.to_string()
    };
    let underline_len = usize::max(1, error.pos.end.saturating_sub(error.pos.start));
    let underline = "^".repeat(underline_len);
    let hint = match format!("{}", error).as_str() {
        msg if msg.contains("Expected a token starting with") => {
            "\n   = help: FTL comments must start with \"# \" (hash followed by a space)"
        }
        _ => "",
    };

    let message = format!(
        "\
{error}
  --> {file}:{line}:{col}
   |
{line:>4} | {snippet}
   | {underline:>col$}{hint}",
        error = error,
        file = file,
        line = line,
        col = col,
        snippet = snippet,
        underline = underline,
        hint = hint,
    );

    Diag::error(file, locale, "", message)
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn fold_text_merges_adjacent_text() {
        let elems = vec![
            Element::Text("Hello ".into()),
            Element::Text("World".into()),
            Element::VarRef("x".into()),
            Element::Text("!".into()),
        ];
        let result = fold_text(elems);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], Element::Text("Hello World".into()));
        assert_eq!(result[1], Element::VarRef("x".into()));
        assert_eq!(result[2], Element::Text("!".into()));
    }

    #[test]
    fn fold_text_skips_empty_text() {
        let elems = vec![Element::Text("".into()), Element::Text("a".into())];
        let result = fold_text(elems);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], Element::Text("a".into()));
    }

    #[test]
    fn fold_text_non_text_passthrough() {
        let elems = vec![Element::VarRef("x".into())];
        let result = fold_text(elems);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn fold_text_empty_input() {
        let result = fold_text(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn ref_prefix_variants() {
        assert_eq!(ref_prefix(RefKind::Message), "");
        assert_eq!(ref_prefix(RefKind::Term), "-");
        assert_eq!(ref_prefix(RefKind::Attribute), ".");
    }

    #[test]
    fn flatten_attr_name_with_hyphens() {
        assert_eq!(
            flatten_attr_name("app-name", "aria-label"),
            "app-name__aria-label"
        );
    }

    #[test]
    fn flatten_attr_name_simple() {
        assert_eq!(flatten_attr_name("save", "label"), "save__label");
    }

    #[test]
    fn line_of_first_line() {
        assert_eq!(line_of("hello", 0), 1);
        assert_eq!(line_of("hello\nworld", 3), 1);
    }

    #[test]
    fn line_of_second_line() {
        assert_eq!(line_of("hello\nworld", 7), 2);
        assert_eq!(line_of("\n", 1), 2);
    }

    #[test]
    fn line_of_empty_source() {
        assert_eq!(line_of("", 0), 1);
    }

    #[test]
    fn format_parse_error_shows_snippet_and_caret() {
        use fluent_syntax::parser::ParserError;
        let source = "msg = hello\nbad = { \"\\x\" }\n";
        // Construct a minimal ParserError (the fields are pub)
        let err = ParserError {
            pos: 21..22,
            slice: Some(20..28),
            kind: fluent_syntax::parser::ErrorKind::UnknownEscapeSequence("\\x".into()),
        };
        let diag = format_parse_error(source, "test.ftl", "en", &err);
        assert!(diag.message.contains("\\x"));
        assert!(diag.message.contains("-->"));
        assert!(diag.message.contains("test.ftl"));
        assert!(diag.message.contains("^"));
    }

    #[test]
    fn format_parse_error_hint_for_comment() {
        use fluent_syntax::parser::ParserError;
        let source = "x = 1\n#（bad\n";
        let err = ParserError {
            pos: 6..7,
            slice: Some(5..12),
            kind: fluent_syntax::parser::ErrorKind::ExpectedToken(' '),
        };
        let diag = format_parse_error(source, "f.ftl", "en", &err);
        assert!(diag.message.contains("help:"));
        assert!(diag.message.contains("hash"));
    }

    #[test]
    fn format_parse_error_truncates_long_line() {
        use fluent_syntax::parser::ParserError;
        let long = "x".repeat(100);
        let source = format!("{}\n", long);
        // Need a ParserError whose Display is not the hint pattern
        let err = ParserError {
            pos: 50..51,
            slice: None,
            kind: fluent_syntax::parser::ErrorKind::MissingDefaultVariant,
        };
        let diag = format_parse_error(&source, "f.ftl", "en", &err);
        assert!(diag.message.contains("..."));
    }
}
