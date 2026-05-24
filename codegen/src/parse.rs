use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::fs;
use std::path::Path;

use fluent_syntax::ast::{self, Entry, Expression, InlineExpression, PatternElement, VariantKey};
use fluent_syntax::parser;

use crate::ast::*;
use crate::fmt::{gen_fn_decl, generate_one_function};
use crate::params::collect_params;
use crate::util::{sanitize, sanitize_const, sanitize_upper};

pub struct Generator {
    pub primary: String,
    pub locales: BTreeMap<String, LocaleEntries>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VisitState {
    Visiting,
    Done,
}

struct Resolver<'a> {
    locale: &'a str,
    entries: &'a LocaleEntries,
    messages: BTreeMap<String, Message>,
    terms: BTreeMap<String, Term>,
    message_states: BTreeMap<String, VisitState>,
    term_states: BTreeMap<String, VisitState>,
    stack: Vec<(RefKind, String)>,
}

impl Generator {
    pub fn load(dir: &Path, primary: &str) -> Self {
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
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("Cannot read {}: {}", path.display(), e));
            let resource = parser::parse(source.as_str())
                .unwrap_or_else(|e| panic!("Parse error in {}: {:?}", path.display(), e));
            locales.insert(locale, Self::extract(&resource));
        }
        assert!(
            locales.contains_key(primary),
            "Primary locale '{}' not found",
            primary,
        );

        let primary_entries = &locales[primary];
        let primary_message_keys: BTreeSet<&str> = primary_entries
            .messages
            .keys()
            .map(|k| k.as_str())
            .collect();
        let primary_term_keys: BTreeSet<&str> =
            primary_entries.terms.keys().map(|k| k.as_str()).collect();

        for (name, entries) in &locales {
            if name == primary {
                continue;
            }
            let locale_message_keys: BTreeSet<&str> =
                entries.messages.keys().map(|k| k.as_str()).collect();
            let extra_messages: Vec<&&str> = locale_message_keys
                .difference(&primary_message_keys)
                .collect();
            if !extra_messages.is_empty() {
                panic!("Locale '{}' has extra messages: {:?}", name, extra_messages);
            }

            let locale_term_keys: BTreeSet<&str> =
                entries.terms.keys().map(|k| k.as_str()).collect();
            let extra_terms: Vec<&&str> = locale_term_keys.difference(&primary_term_keys).collect();
            if !extra_terms.is_empty() {
                panic!("Locale '{}' has extra terms: {:?}", name, extra_terms);
            }
        }

        let mut resolved_locales = BTreeMap::new();
        for (locale, entries) in locales {
            let mut resolver = Resolver::new(&locale, &entries);
            let messages = resolver.resolve_all_messages();
            let terms = resolver.resolve_all_terms();
            resolved_locales.insert(locale, LocaleEntries { messages, terms });
        }

        Generator {
            primary: primary.to_string(),
            locales: resolved_locales,
        }
    }

    fn extract(resource: &ast::Resource<&str>) -> LocaleEntries {
        let mut messages = BTreeMap::new();
        let mut terms = BTreeMap::new();

        for entry in &resource.body {
            match entry {
                Entry::Message(msg) => {
                    if let Some(pattern) = &msg.value {
                        let name = msg.id.name.to_string();
                        messages.insert(
                            name.clone(),
                            Message {
                                name,
                                elements: convert_elements(&pattern.elements),
                            },
                        );
                    }
                }
                Entry::Term(term) => {
                    let name = term.id.name.to_string();
                    terms.insert(
                        name.clone(),
                        Term {
                            name,
                            elements: convert_elements(&term.value.elements),
                        },
                    );
                }
                _ => {}
            }
        }

        LocaleEntries { messages, terms }
    }

    pub fn generate(&self) -> String {
        let mut out = String::new();
        let locales: Vec<&String> = self.locales.keys().collect();

        writeln!(out, "// Auto-generated by ftl-codegen").unwrap();
        writeln!(out, "// Primary language: {}", self.primary).unwrap();
        writeln!(out, "#[allow(non_upper_case_globals, unused)]").unwrap();
        writeln!(out).unwrap();

        for locale in &locales {
            self.emit_module(locale, &mut out);
        }
        self.emit_trait(&mut out);
        for locale in &locales {
            self.emit_impl(locale, &mut out);
        }
        self.emit_runtime(&locales, &mut out);

        writeln!(out, "#[macro_export]").unwrap();
        writeln!(out, "macro_rules! t {{").unwrap();
        writeln!(
            out,
            "    ($key:ident) => {{ $crate::get_locale().$key() }};"
        )
        .unwrap();
        writeln!(out, "    ($key:ident($($args:expr),* $(,)?)) => {{ $crate::get_locale().$key($($args),*) }};").unwrap();
        writeln!(out, "}}").unwrap();
        out
    }

    fn emit_module(&self, locale: &str, out: &mut String) {
        let mod_name = sanitize(locale);
        writeln!(out, "pub mod {} {{", mod_name).unwrap();
        writeln!(out, "    use std::fmt::Write;").unwrap();

        let msgs = &self.locales[locale].messages;
        let primary_msgs = &self.locales[&self.primary].messages;
        let locale_keys: BTreeSet<&str> = msgs.keys().map(|k| k.as_str()).collect();

        for (name, p_msg) in primary_msgs {
            let params = collect_params(&p_msg.elements);
            if locale_keys.contains(name.as_str()) {
                let msg = msgs.get(name).expect("message missing from locale");
                let code = generate_one_function(&msg.name, &msg.elements, &params, locale);
                writeln!(out, "{}", code.trim_end()).unwrap();
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
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }

    fn emit_trait(&self, out: &mut String) {
        writeln!(out, "pub trait I18n {{").unwrap();
        for msg in self.locales[&self.primary].messages.values() {
            let params = collect_params(&msg.elements);
            writeln!(out, "    fn {};", gen_fn_decl(&msg.name, &params, true)).unwrap();
        }
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }

    fn emit_impl(&self, locale: &str, out: &mut String) {
        let struct_name = sanitize_upper(locale);
        let mod_name = sanitize(locale);
        writeln!(out, "pub struct {};", struct_name).unwrap();
        writeln!(out, "impl I18n for {} {{", struct_name).unwrap();
        for msg in self.locales[&self.primary].messages.values() {
            let params = collect_params(&msg.elements);
            let args = params
                .keys()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(out, "    fn {} {{", gen_fn_decl(&msg.name, &params, true)).unwrap();
            writeln!(out, "        {}::{}({})", mod_name, msg.name, args).unwrap();
            writeln!(out, "    }}").unwrap();
        }
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
        writeln!(
            out,
            "pub const {}: {} = {};",
            sanitize_const(locale),
            struct_name,
            struct_name
        )
        .unwrap();
        writeln!(out).unwrap();
    }

    fn emit_runtime(&self, locales: &[&String], out: &mut String) {
        let primary = sanitize_const(&self.primary);

        writeln!(out, "use std::sync::atomic::{{AtomicU8, Ordering}};").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "static LOCALE_ID: AtomicU8 = AtomicU8::new(0);").unwrap();
        writeln!(out).unwrap();

        writeln!(out, "pub fn get_locale() -> &'static (dyn I18n + Sync) {{").unwrap();
        writeln!(out, "    match LOCALE_ID.load(Ordering::Acquire) {{").unwrap();
        for (idx, locale) in locales.iter().enumerate() {
            if **locale != self.primary {
                writeln!(out, "        {} => &{},", idx, sanitize_const(locale)).unwrap();
            }
        }
        writeln!(out, "        _ => &{},", primary).unwrap();
        writeln!(out, "    }}").unwrap();
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();

        writeln!(out, "pub enum Lang {{").unwrap();
        for locale in locales {
            writeln!(out, "    {},", sanitize_upper(locale)).unwrap();
        }
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
        writeln!(out).unwrap();
    }
}

impl<'a> Resolver<'a> {
    fn new(locale: &'a str, entries: &'a LocaleEntries) -> Self {
        Self {
            locale,
            entries,
            messages: BTreeMap::new(),
            terms: BTreeMap::new(),
            message_states: BTreeMap::new(),
            term_states: BTreeMap::new(),
            stack: Vec::new(),
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
                    "Undefined message reference '{}' in locale '{}'",
                    name, self.locale
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
                    "Undefined term reference '-{}' in locale '{}'",
                    name, self.locale
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

    fn resolve_elements(&mut self, elements: &[Element]) -> Vec<Element> {
        let mut out = Vec::new();
        for element in elements {
            match element {
                Element::Text(text) => out.push(Element::Text(text.clone())),
                Element::VarRef(name) => out.push(Element::VarRef(name.clone())),
                Element::MessageRef(name) => {
                    let resolved = self.resolve_message(name);
                    out.extend(resolved.elements.clone());
                }
                Element::TermRef(name) => {
                    let resolved = self.resolve_term(name);
                    out.extend(resolved.elements.clone());
                }
                Element::Select { selector, variants } => {
                    let variants = variants
                        .iter()
                        .map(|variant| Variant {
                            key: variant.key.clone(),
                            elements: fold_text(self.resolve_elements(&variant.elements)),
                            default: variant.default,
                        })
                        .collect();
                    out.push(Element::Select {
                        selector: selector.clone(),
                        variants,
                    });
                }
            }
        }
        fold_text(out)
    }

    fn cycle_panic(&self, ref_kind: RefKind, name: &str) -> ! {
        let mut chain = self
            .stack
            .iter()
            .map(|(kind, entry)| format!("{}{}", ref_prefix(*kind), entry))
            .collect::<Vec<_>>();
        chain.push(format!("{}{}", ref_prefix(ref_kind), name));
        panic!(
            "Cyclic {} reference in locale '{}': {}",
            match ref_kind {
                RefKind::Message => "message",
                RefKind::Term => "term",
            },
            self.locale,
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
    }
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

fn convert_expression(expr: &Expression<&str>) -> Element {
    match expr {
        Expression::Inline(inline) => match inline {
            InlineExpression::VariableReference { id } => Element::VarRef(id.name.to_string()),
            InlineExpression::MessageReference { id, .. } => {
                Element::MessageRef(id.name.to_string())
            }
            InlineExpression::TermReference { id, .. } => Element::TermRef(id.name.to_string()),
            other => panic!("Unsupported expression: {:?}", other),
        },
        Expression::Select { selector, variants } => {
            let name = match selector {
                InlineExpression::VariableReference { id } => id.name.to_string(),
                other => panic!("Select selector must be a variable: {:?}", other),
            };
            let vs = variants
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
            Element::Select {
                selector: name,
                variants: vs,
            }
        }
    }
}
