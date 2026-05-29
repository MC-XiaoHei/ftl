use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use fluent_syntax::ast::{self, Entry, Expression, InlineExpression, PatternElement, VariantKey};
use fluent_syntax::parser::{self, ParserError};

use crate::ast::*;
use crate::diag::{report_diagnostics, Diag, DiagKind};
use crate::fmt::generate_one_function;
use crate::params::collect_params_with_context;
use crate::util::{sanitize, sanitize_upper};
use crate::BuiltInFuncDef;

pub struct Generator {
    pub primary: String,
    pub locales: BTreeMap<String, LocaleEntries>,
    pub diags: Vec<Diag>,
    pub module_path: String,
    pub builtins: BTreeMap<String, BuiltInFuncDef>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VisitState {
    Visiting,
    Done,
}

struct Resolver<'a> {
    #[allow(dead_code)]
    locale: &'a str,
    #[allow(dead_code)]
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
    /// Load locale files (NUMBER and DATETIME are auto-registered).
    pub fn load_simple(dir: &Path, primary: &str, module_path: &str) -> Self {
        let mut builtins = BTreeMap::new();
        // Always auto-register NUMBER and DATETIME.
        builtins
            .entry("NUMBER".to_string())
            .or_insert_with(|| BuiltInFuncDef {
                name: "NUMBER".to_string(),
                ty_name: "Number".to_string(),
                named_args: Vec::new(),
                write_to_body: None,
            });
        builtins
            .entry("DATETIME".to_string())
            .or_insert_with(|| BuiltInFuncDef {
                name: "DATETIME".to_string(),
                ty_name: "DateTime".to_string(),
                named_args: Vec::new(),
                write_to_body: None,
            });
        Self::load(dir, primary, module_path, &builtins)
    }

    pub fn load(
        dir: &Path,
        primary: &str,
        module_path: &str,
        builtins: &BTreeMap<String, BuiltInFuncDef>,
    ) -> Self {
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
                    locales.insert(locale, Self::extract(&r, builtins));
                }
                Err((partial, errors)) => {
                    for err in &errors {
                        diags.push(format_parse_error(&source, &file_display, &locale, err));
                    }
                    // use partial result even on parse errors
                    locales.insert(locale, Self::extract(&partial, builtins));
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
            module_path: module_path.to_string(),
            builtins: builtins.clone(),
        }
    }

    fn extract(
        resource: &ast::Resource<&str>,
        builtins: &BTreeMap<String, BuiltInFuncDef>,
    ) -> LocaleEntries {
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
                                elements: convert_elements(&pattern.elements, builtins),
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
                                elements: convert_elements(&attr.value.elements, builtins),
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
                            elements: convert_elements(&term.value.elements, builtins),
                        },
                    );
                    for attr in &term.attributes {
                        let attr_name = attr.id.name.to_string();
                        attributes.insert(
                            flatten_attr_name(&owner, &attr_name),
                            Attribute {
                                owner: owner.clone(),
                                name: attr_name,
                                elements: convert_elements(&attr.value.elements, builtins),
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
        let pfx = if self.module_path.is_empty() {
            "$crate".to_string()
        } else {
            format!("$crate::{}", self.module_path)
        };
        writeln!(out, "// Auto-generated by ftl-codegen").unwrap();
        writeln!(out, "// Primary language: {}", self.primary).unwrap();
        writeln!(out, "#[allow(non_upper_case_globals, unused, dead_code)]").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "use ftl_core::FluentNum;").unwrap();
        writeln!(out).unwrap();
        self.emit_builtin_types(&mut out);
        for locale in &locales {
            self.emit_module(locale, &mut out);
        }
        self.emit_runtime(&locales, &mut out);
        writeln!(out, "#[macro_export]").unwrap();
        writeln!(out, "macro_rules! t {{").unwrap();
        writeln!(out, "    ($key:ident) => {{").unwrap();
        writeln!(out, "        match {pfx}::__ftl_locale_id() {{").unwrap();
        for (idx, locale) in locales.iter().enumerate() {
            let mn = sanitize(locale);
            writeln!(out, "            {} => {}::{}::$key(),", idx, pfx, mn).unwrap();
        }
        writeln!(
            out,
            "            _ => {}::{}::$key(),",
            pfx,
            sanitize(&self.primary)
        )
        .unwrap();
        writeln!(out, "        }}").unwrap();
        writeln!(out, "    }};").unwrap();
        writeln!(out, "    ($key:ident($($args:expr),* $(,)?)) => {{").unwrap();
        writeln!(out, "        match {pfx}::__ftl_locale_id() {{").unwrap();
        for (idx, locale) in locales.iter().enumerate() {
            let mn = sanitize(locale);
            writeln!(
                out,
                "            {} => {}::{}::$key($($args),*),",
                idx, pfx, mn
            )
            .unwrap();
        }
        writeln!(
            out,
            "            _ => {}::{}::$key($($args),*),",
            pfx,
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
        writeln!(
            out,
            "    #![allow(non_snake_case, unused_imports, dead_code, unused_variables)]"
        )
        .unwrap();
        writeln!(out, "    use std::fmt::Write;").unwrap();
        writeln!(out, "    use ftl_core::{{FluentNum, FluentArg}};").unwrap();
        for def in self.builtins.values() {
            if def.write_to_body.is_some() {
                writeln!(out, "    use super::{};", def.ty_name).unwrap();
            }
        }
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
                    generate_one_function(
                        &msg.name,
                        &msg.elements,
                        &params,
                        locale,
                        &self.builtins,
                    )
                    .trim_end()
                )
                .unwrap();
            } else {
                writeln!(
                    out,
                    "{}",
                    generate_one_function(
                        &p_msg.name,
                        &p_msg.elements,
                        &params,
                        &self.primary,
                        &self.builtins,
                    )
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
                    generate_one_function(
                        &fn_name,
                        &attr.elements,
                        &params,
                        locale,
                        &self.builtins,
                    )
                    .trim_end()
                )
                .unwrap();
            } else {
                writeln!(
                    out,
                    "{}",
                    generate_one_function(
                        &fn_name,
                        &p_attr.elements,
                        &params,
                        &self.primary,
                        &self.builtins,
                    )
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

    fn emit_builtin_types(&self, out: &mut String) {
        use std::fmt::Write;
        for def in self.builtins.values() {
            let Some(body) = &def.write_to_body else {
                continue;
            };
            writeln!(out).unwrap();
            writeln!(out, "pub struct {} {{", def.ty_name).unwrap();
            writeln!(out, "    pub value: FluentNum,").unwrap();
            for arg in &def.named_args {
                let arg_ty = match arg.arg_type {
                    crate::BuiltInArgType::String => "String",
                    crate::BuiltInArgType::Int => "i64",
                    crate::BuiltInArgType::Float => "f64",
                    crate::BuiltInArgType::Bool => "bool",
                };
                writeln!(out, "    pub {}: Option<{}>,", arg.rust_name, arg_ty).unwrap();
            }
            writeln!(out, "}}").unwrap();
            writeln!(out).unwrap();
            writeln!(out, "impl {} {{", def.ty_name).unwrap();
            writeln!(out, "    pub fn new(v: impl Into<FluentNum>) -> Self {{").unwrap();
            writeln!(out, "        Self {{").unwrap();
            writeln!(out, "            value: v.into(),").unwrap();
            for arg in &def.named_args {
                writeln!(out, "            {}: None,", arg.rust_name).unwrap();
            }
            writeln!(out, "        }}").unwrap();
            writeln!(out, "    }}").unwrap();
            writeln!(out).unwrap();
            for arg in &def.named_args {
                let arg_ty = match arg.arg_type {
                    crate::BuiltInArgType::String => "String",
                    crate::BuiltInArgType::Int => "i64",
                    crate::BuiltInArgType::Float => "f64",
                    crate::BuiltInArgType::Bool => "bool",
                };
                writeln!(
                    out,
                    "    pub fn {}(mut self, v: {}) -> Self {{",
                    arg.rust_name, arg_ty
                )
                .unwrap();
                writeln!(out, "        self.{} = Some(v);", arg.rust_name).unwrap();
                writeln!(out, "        self").unwrap();
                writeln!(out, "    }}").unwrap();
                writeln!(out).unwrap();
            }
            writeln!(out, "    pub fn write_to(self, out: &mut String) {{").unwrap();
            writeln!(out, "        self.write_to_with(out, get_locale())").unwrap();
            writeln!(out, "    }}").unwrap();
            writeln!(out).unwrap();
            writeln!(out, "    #[allow(unused_variables)]").unwrap();
            writeln!(
                out,
                "    pub fn write_to_with(self, out: &mut String, lang: Lang) {{"
            )
            .unwrap();
            writeln!(out, "        let this = &self;").unwrap();
            writeln!(out, "        {}", body).unwrap();
            writeln!(out, "    }}").unwrap();
            writeln!(out, "}}").unwrap();
        }
    }

    fn emit_runtime(&self, locales: &[&String], out: &mut String) {
        writeln!(out, "use std::sync::atomic::{{AtomicU8, Ordering}};").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "use ftl_core::unic_langid::LanguageIdentifier;").unwrap();
        writeln!(out, "use std::str::FromStr;").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "static LOCALE_ID: AtomicU8 = AtomicU8::new(0);").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "#[doc(hidden)]").unwrap();
        writeln!(out, "#[inline]").unwrap();
        writeln!(out, "pub fn __ftl_locale_id() -> u8 {{").unwrap();
        writeln!(out, "    LOCALE_ID.load(Ordering::Acquire)").unwrap();
        writeln!(out, "}}").unwrap();
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
                "            Lang::{} => LanguageIdentifier::from_str(\"{}\").expect(\"valid locale\"),",
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
        writeln!(out, "    let lid: LanguageIdentifier = lang.into();").unwrap();
        writeln!(out, "    ftl_core::set_locale(&lid);").unwrap();
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
        let keys: Vec<String> = self.entries.messages.keys().cloned().collect();
        for key in keys {
            self.resolve_message(&key);
        }
        self.messages.clone()
    }

    fn resolve_all_terms(&mut self) -> BTreeMap<String, Term> {
        let keys: Vec<String> = self.entries.terms.keys().cloned().collect();
        for key in keys {
            self.resolve_term(&key);
        }
        self.terms.clone()
    }

    fn resolve_all_attributes(&mut self) -> BTreeMap<String, Attribute> {
        let keys: Vec<String> = self.entries.attributes.keys().cloned().collect();
        for key in keys {
            self.resolve_attribute(&key);
        }
        self.attributes.clone()
    }

    fn resolve_message(&mut self, name: &str) {
        match self.message_states.get(name).copied() {
            Some(VisitState::Visiting) => {
                self.cycle_panic(RefKind::Message, name);
            }
            Some(VisitState::Done) => return,
            None => {}
        }
        self.message_states
            .insert(name.to_string(), VisitState::Visiting);
        self.stack.push((RefKind::Message, name.to_string()));
        let entry = self
            .entries
            .messages
            .get(name)
            .unwrap_or_else(|| panic!("undefined message reference: '{}'", name));
        let resolved = self.resolve_elements(&entry.elements);
        self.stack.pop();
        self.message_states
            .insert(name.to_string(), VisitState::Done);
        self.messages.insert(
            name.to_string(),
            Message {
                name: name.to_string(),
                elements: resolved,
            },
        );
    }

    fn resolve_term(&mut self, name: &str) {
        match self.term_states.get(name).copied() {
            Some(VisitState::Visiting) => {
                self.cycle_panic(RefKind::Term, name);
            }
            Some(VisitState::Done) => return,
            None => {}
        }
        self.term_states
            .insert(name.to_string(), VisitState::Visiting);
        self.stack.push((RefKind::Term, name.to_string()));
        let entry = self
            .entries
            .terms
            .get(name)
            .unwrap_or_else(|| panic!("undefined term reference: '-{}'", name));
        let resolved = self.resolve_elements(&entry.elements);
        self.stack.pop();
        self.term_states.insert(name.to_string(), VisitState::Done);
        self.terms.insert(
            name.to_string(),
            Term {
                name: name.to_string(),
                elements: resolved,
            },
        );
    }

    fn resolve_attribute(&mut self, flat_name: &str) {
        match self.attribute_states.get(flat_name).copied() {
            Some(VisitState::Visiting) => {
                // Extract owner from flat name (before `__`)
                let owner = flat_name.split("__").next().unwrap_or(flat_name);
                self.cycle_panic(RefKind::Attribute, owner);
            }
            Some(VisitState::Done) => return,
            None => {}
        }
        self.attribute_states
            .insert(flat_name.to_string(), VisitState::Visiting);
        let entry = self
            .entries
            .attributes
            .get(flat_name)
            .unwrap_or_else(|| panic!("undefined attribute reference: '{}'", flat_name));
        let resolved = self.resolve_elements(&entry.elements);
        self.attribute_states
            .insert(flat_name.to_string(), VisitState::Done);
        self.attributes.insert(
            flat_name.to_string(),
            Attribute {
                owner: entry.owner.clone(),
                name: entry.name.clone(),
                elements: resolved,
            },
        );
    }

    fn resolve_elements(&mut self, elements: &[Element]) -> Vec<Element> {
        let mut out = Vec::new();
        for e in elements {
            match e {
                Element::Text(_) | Element::BuiltInCall { .. } => {
                    out.push(e.clone());
                }
                Element::VarRef(name) => {
                    let bound = self.env_stack.last().and_then(|env| env.get(name).cloned());
                    if let Some(bound) = bound {
                        out.push(bound);
                    } else {
                        out.push(Element::VarRef(name.clone()));
                    }
                }
                Element::Select { selector, variants } => {
                    let resolved_variants: Vec<Variant> = variants
                        .iter()
                        .map(|v| Variant {
                            key: v.key.clone(),
                            elements: self.resolve_elements(&v.elements),
                            default: v.default,
                        })
                        .collect();
                    out.push(Element::Select {
                        selector: selector.clone(),
                        variants: resolved_variants,
                    });
                }
                Element::MessageRef(name) => {
                    self.resolve_message(name);
                    let m = self.messages.get(name).unwrap();
                    out.extend(m.elements.clone());
                }
                Element::TermRef {
                    name,
                    attribute,
                    args,
                    positional,
                } => {
                    let resolved = self.resolve_term_with_args(name, attribute, args, positional);
                    out.extend(resolved);
                }
                Element::AttributeRef { owner, name } => {
                    self.resolve_attribute(&flatten_attr_name(owner, name));
                    let flat_name = flatten_attr_name(owner, name);
                    let a = self.attributes.get(&flat_name).unwrap();
                    out.extend(a.elements.clone());
                }
                Element::TermAttrSelect {
                    term,
                    attr,
                    variants,
                } => {
                    // Resolve the term attribute's value at build time
                    let flat_name = flatten_attr_name(term, attr);
                    self.resolve_attribute(&flat_name);
                    let attr_entry = self.attributes.get(&flat_name).unwrap();
                    // The attribute value should be a single literal string
                    // Inline the attribute value as the selector
                    let attr_text: String = attr_entry
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
                        KeyType::Ident(ident) => ident == &attr_text,
                        KeyType::Num(val) => val == &attr_text,
                    });
                    if let Some(variant) = matched {
                        out.extend(self.resolve_elements(&variant.elements));
                    } else if let Some(variant) = variants.iter().find(|v| v.default) {
                        out.extend(self.resolve_elements(&variant.elements));
                    }
                }
            }
        }
        fold_text(out)
    }

    fn resolve_term_with_args(
        &mut self,
        name: &str,
        attribute: &Option<String>,
        args: &BTreeMap<String, Element>,
        positional: &[Element],
    ) -> Vec<Element> {
        self.resolve_term(name);
        let term_elements = self
            .terms
            .get(name)
            .map(|t| t.elements.clone())
            .unwrap_or_default();
        let term_name = self
            .terms
            .get(name)
            .map(|t| t.name.clone())
            .unwrap_or_default();

        // Build the binding environment (arg name → element)
        let mut env = BTreeMap::new();
        let term_params =
            collect_params_with_context(&term_elements, &format!("term '-{}'", term_name));
        // Map positional args to parameter names (in order of param discovery)
        let term_param_names: Vec<&String> = term_params.keys().collect();
        for (i, pos) in positional.iter().enumerate() {
            if let Some(pname) = term_param_names.get(i) {
                env.insert((*pname).clone(), pos.clone());
            }
        }
        for (k, v) in args {
            env.insert(k.clone(), v.clone());
        }

        // Resolve the term elements with this environment
        self.env_stack.push(env);
        let resolved = self.resolve_elements(&term_elements);
        self.env_stack.pop();

        // If an attribute is specified, extract just that attribute
        if let Some(attr_name) = attribute {
            let flat_name = flatten_attr_name(name, attr_name);
            self.resolve_attribute(&flat_name);
            let attr_elements = self.attributes.get(&flat_name).map(|a| a.elements.clone());
            if let Some(ae) = attr_elements {
                // Resolve attribute elements too (variables may be forwarded)
                self.env_stack.push(BTreeMap::new()); // empty env for attribute
                let attr_resolved = self.resolve_elements(&ae);
                self.env_stack.pop();
                return attr_resolved;
            }
        }

        resolved
    }

    fn cycle_panic(&self, kind: RefKind, name: &str) {
        let prefix = ref_prefix(kind);
        let mut msg = format!(
            "cyclic {} reference: {}{}",
            match kind {
                RefKind::Message => "message",
                RefKind::Term => "term",
                RefKind::Attribute => "attribute",
            },
            prefix,
            name
        );
        msg.push_str("\n  resolution stack:");
        for (k, n) in &self.stack {
            let p = ref_prefix(*k);
            msg.push_str(&format!("\n    - {}{}", p, n));
        }
        panic!("{}", msg);
    }
}

fn fold_text(elems: Vec<Element>) -> Vec<Element> {
    let mut out = Vec::new();
    for e in elems {
        match e {
            Element::Text(s) => {
                if s.is_empty() {
                    continue;
                }
                match out.last_mut() {
                    Some(Element::Text(last)) => last.push_str(&s),
                    _ => out.push(Element::Text(s)),
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

fn convert_elements(
    elems: &[PatternElement<&str>],
    builtins: &BTreeMap<String, BuiltInFuncDef>,
) -> Vec<Element> {
    elems.iter().map(|e| convert_element(e, builtins)).collect()
}

fn convert_element(
    e: &PatternElement<&str>,
    builtins: &BTreeMap<String, BuiltInFuncDef>,
) -> Element {
    match e {
        PatternElement::TextElement { value } => Element::Text(value.to_string()),
        PatternElement::Placeable { expression } => convert_expression(expression, builtins),
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

fn convert_expression(
    expr: &Expression<&str>,
    builtins: &BTreeMap<String, BuiltInFuncDef>,
) -> Element {
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
            InlineExpression::FunctionReference { id, arguments } => {
                let func_name = id.name.to_string();
                let def = builtins.get(&func_name).unwrap_or_else(|| {
                    panic!(
                        "Unrecognized function '{}'. Register it with .register() before calling .generate()",
                        func_name
                    )
                });
                let base: String = match arguments.positional.first() {
                    Some(InlineExpression::VariableReference { id }) => id.name.to_string(),
                    _ => panic!(
                        "{}() expects a variable reference as the first argument",
                        func_name
                    ),
                };
                let mut named_args = BTreeMap::new();
                for arg in &arguments.named {
                    let value: String = match &arg.value {
                        InlineExpression::StringLiteral { value } => (*value).to_string(),
                        InlineExpression::NumberLiteral { value } => (*value).to_string(),
                        _ => panic!(
                            "Named argument '{}' to {}() must be a string or number literal",
                            arg.name.name, func_name
                        ),
                    };
                    named_args.insert(arg.name.name.to_string(), value);
                }
                Element::BuiltInCall {
                    func_name,
                    ty_name: def.ty_name.clone(),
                    var_name: base,
                    named_args,
                }
            }
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
                                elements: convert_elements(&v.value.elements, builtins),
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
                        elements: convert_elements(&v.value.elements, builtins),
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
    use crate::{BuiltInArgType, BuiltInNamedArg};

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
    fn convert_element_text() {
        let pe = PatternElement::TextElement { value: "hello" };
        let result = convert_element(&pe, &BTreeMap::new());
        assert_eq!(result, Element::Text("hello".to_string()));
    }

    #[test]
    fn convert_expression_variable_ref() {
        let expr = Expression::Inline(InlineExpression::VariableReference {
            id: ast::Identifier { name: "name" },
        });
        let result = convert_expression(&expr, &BTreeMap::new());
        assert_eq!(result, Element::VarRef("name".to_string()));
    }

    #[test]
    fn convert_expression_message_ref() {
        let expr = Expression::Inline(InlineExpression::MessageReference {
            id: ast::Identifier { name: "greeting" },
            attribute: None,
        });
        let result = convert_expression(&expr, &BTreeMap::new());
        assert_eq!(result, Element::MessageRef("greeting".to_string()));
    }

    #[test]
    fn convert_expression_message_ref_with_attr() {
        let expr = Expression::Inline(InlineExpression::MessageReference {
            id: ast::Identifier { name: "save" },
            attribute: Some(ast::Identifier { name: "label" }),
        });
        let result = convert_expression(&expr, &BTreeMap::new());
        assert_eq!(
            result,
            Element::AttributeRef {
                owner: "save".to_string(),
                name: "label".to_string(),
            }
        );
    }

    #[test]
    fn convert_expression_string_literal() {
        let expr = Expression::Inline(InlineExpression::StringLiteral { value: "hello" });
        let result = convert_expression(&expr, &BTreeMap::new());
        assert_eq!(result, Element::Text("hello".to_string()));
    }

    #[test]
    fn convert_expression_number_literal() {
        let expr = Expression::Inline(InlineExpression::NumberLiteral { value: "42" });
        let result = convert_expression(&expr, &BTreeMap::new());
        assert_eq!(result, Element::Text("42".to_string()));
    }

    #[test]
    fn convert_expression_function_ref() {
        let mut builtins = BTreeMap::new();
        builtins.insert(
            "HTML".to_string(),
            BuiltInFuncDef {
                name: "HTML".to_string(),
                ty_name: "Html".to_string(),
                named_args: vec![BuiltInNamedArg {
                    ftl_name: "class".to_string(),
                    rust_name: "class".to_string(),
                    arg_type: BuiltInArgType::String,
                }],
                write_to_body: None,
            },
        );

        let expr = Expression::Inline(InlineExpression::FunctionReference {
            id: ast::Identifier { name: "HTML" },
            arguments: ast::CallArguments {
                positional: vec![InlineExpression::VariableReference {
                    id: ast::Identifier { name: "input" },
                }],
                named: vec![ast::NamedArgument {
                    name: ast::Identifier { name: "class" },
                    value: InlineExpression::StringLiteral { value: "btn" },
                }],
            },
        });
        let result = convert_expression(&expr, &builtins);
        assert!(matches!(result, Element::BuiltInCall { .. }));
        if let Element::BuiltInCall {
            func_name,
            var_name,
            named_args,
            ..
        } = result
        {
            assert_eq!(func_name, "HTML");
            assert_eq!(var_name, "input");
            assert_eq!(named_args.get("class").map(|s| s.as_str()), Some("btn"));
        }
    }

    #[test]
    fn convert_collected_empty() {
        let result = convert_collected(vec![]);
        assert_eq!(result, Element::Text(String::new()));
    }

    #[test]
    fn convert_collected_single() {
        let result = convert_collected(vec![Element::VarRef("x".to_string())]);
        assert_eq!(result, Element::VarRef("x".to_string()));
    }

    #[test]
    fn convert_expression_number_literal_selector_matches() {
        use ast::VariantKey;
        let expr = Expression::Select {
            selector: ast::InlineExpression::NumberLiteral { value: "42" },
            variants: vec![
                ast::Variant {
                    key: VariantKey::NumberLiteral { value: "42" },
                    value: ast::Pattern {
                        elements: vec![PatternElement::TextElement { value: "meaning" }],
                    },
                    default: false,
                },
                ast::Variant {
                    key: VariantKey::Identifier { name: "other" },
                    value: ast::Pattern {
                        elements: vec![PatternElement::TextElement { value: "unknown" }],
                    },
                    default: true,
                },
            ],
        };
        let result = convert_expression(&expr, &BTreeMap::new());
        assert_eq!(result, Element::Text("meaning".to_string()));
    }

    #[test]
    fn convert_expression_number_literal_selector_uses_default() {
        use ast::VariantKey;
        let expr = Expression::Select {
            selector: ast::InlineExpression::NumberLiteral { value: "99" },
            variants: vec![
                ast::Variant {
                    key: VariantKey::NumberLiteral { value: "42" },
                    value: ast::Pattern {
                        elements: vec![PatternElement::TextElement { value: "meaning" }],
                    },
                    default: false,
                },
                ast::Variant {
                    key: VariantKey::Identifier { name: "other" },
                    value: ast::Pattern {
                        elements: vec![PatternElement::TextElement { value: "fallback" }],
                    },
                    default: true,
                },
            ],
        };
        let result = convert_expression(&expr, &BTreeMap::new());
        assert_eq!(result, Element::Text("fallback".to_string()));
    }

    #[test]
    fn convert_expression_string_literal_selector_matches() {
        use ast::VariantKey;
        let expr = Expression::Select {
            selector: ast::InlineExpression::StringLiteral { value: "hello" },
            variants: vec![
                ast::Variant {
                    key: VariantKey::Identifier { name: "hello" },
                    value: ast::Pattern {
                        elements: vec![PatternElement::TextElement { value: "greeting" }],
                    },
                    default: false,
                },
                ast::Variant {
                    key: VariantKey::Identifier { name: "other" },
                    value: ast::Pattern {
                        elements: vec![PatternElement::TextElement { value: "unknown" }],
                    },
                    default: true,
                },
            ],
        };
        let result = convert_expression(&expr, &BTreeMap::new());
        assert_eq!(result, Element::Text("greeting".to_string()));
    }

    #[test]
    fn convert_inline_expression_message_ref() {
        let expr = InlineExpression::MessageReference {
            id: ast::Identifier { name: "settings" },
            attribute: None,
        };
        let result = convert_inline_expression(&expr);
        assert_eq!(result, Element::MessageRef("settings".to_string()));
    }

    #[test]
    fn convert_inline_expression_message_ref_with_attr() {
        let expr = InlineExpression::MessageReference {
            id: ast::Identifier { name: "save" },
            attribute: Some(ast::Identifier { name: "label" }),
        };
        let result = convert_inline_expression(&expr);
        assert_eq!(
            result,
            Element::AttributeRef {
                owner: "save".to_string(),
                name: "label".to_string(),
            }
        );
    }

    #[test]
    fn convert_inline_expression_term_ref() {
        let expr = InlineExpression::TermReference {
            id: ast::Identifier { name: "link" },
            attribute: None,
            arguments: Some(ast::CallArguments {
                positional: vec![InlineExpression::VariableReference {
                    id: ast::Identifier { name: "url" },
                }],
                named: vec![ast::NamedArgument {
                    name: ast::Identifier { name: "class" },
                    value: InlineExpression::StringLiteral { value: "btn" },
                }],
            }),
        };
        let result = convert_inline_expression(&expr);
        assert!(matches!(result, Element::TermRef { .. }));
        if let Element::TermRef {
            name,
            attribute,
            args,
            positional,
        } = result
        {
            assert_eq!(name, "link");
            assert!(attribute.is_none());
            assert_eq!(args.len(), 1);
            assert_eq!(positional.len(), 1);
        }
    }

    #[test]
    fn convert_inline_expression_string_literal() {
        let expr = InlineExpression::StringLiteral { value: "hello" };
        let result = convert_inline_expression(&expr);
        assert_eq!(result, Element::Text("hello".to_string()));
    }

    #[test]
    fn convert_inline_expression_number_literal() {
        let expr = InlineExpression::NumberLiteral { value: "3.14" };
        let result = convert_inline_expression(&expr);
        assert_eq!(result, Element::Text("3.14".to_string()));
    }

    #[test]
    fn convert_inline_expression_variable_ref() {
        let expr = InlineExpression::VariableReference {
            id: ast::Identifier { name: "name" },
        };
        let result = convert_inline_expression(&expr);
        assert_eq!(result, Element::VarRef("name".to_string()));
    }

    #[test]
    fn convert_expression_term_ref_as_selector() {
        use ast::VariantKey;
        let expr = Expression::Select {
            selector: InlineExpression::TermReference {
                id: ast::Identifier { name: "brand" },
                attribute: Some(ast::Identifier { name: "gender" }),
                arguments: None,
            },
            variants: vec![
                ast::Variant {
                    key: VariantKey::Identifier { name: "masculine" },
                    value: ast::Pattern {
                        elements: vec![PatternElement::TextElement { value: "Mr." }],
                    },
                    default: false,
                },
                ast::Variant {
                    key: VariantKey::Identifier { name: "feminine" },
                    value: ast::Pattern {
                        elements: vec![PatternElement::TextElement { value: "Ms." }],
                    },
                    default: true,
                },
            ],
        };
        let result = convert_expression(&expr, &BTreeMap::new());
        assert!(matches!(result, Element::TermAttrSelect { .. }));
        if let Element::TermAttrSelect {
            term,
            attr,
            variants,
        } = result
        {
            assert_eq!(term, "brand");
            assert_eq!(attr, "gender");
            assert_eq!(variants.len(), 2);
        }
    }

    #[test]
    fn convert_collected_multiple_folds_to_select() {
        let result = convert_collected(vec![
            Element::Text("a".to_string()),
            Element::VarRef("x".to_string()),
        ]);
        assert!(matches!(result, Element::Select { .. }));
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
