# FTL

*Faster than light.*

Compile-time conversion of Fluent (.ftl) translation files to type-safe Rust functions. No parser, no hash table at runtime.

**This library is under active development and is not yet stable. ANY API MAY CHANGE WITHOUT NOTICE!**

[![rust 1.70+](https://img.shields.io/badge/rust-1.70%2B-orange?style=flat&logo=rust)](https://www.rust-lang.org)
[![license](https://img.shields.io/badge/license-MIT%20%7C%20Unicode--3.0-blue?style=flat)](#license)

## Benchmarks

Test Environment: AMD AI 9 H 365, `cargo bench -p example`.

- pure text: < 5ns
- translate with placeholder: < 50ns
- translate with NUMBER(): < 1us
- translate with DATETIME(): < 5us

## Limitations

- No context-based type inference: variables are not automatically inferred as `FluentDateTime`/`FluentNumber` — you must call `DATETIME()`/`NUMBER()` with explicit named args
- Function calls as selectors are unsupported
- The same free variable cannot be inferred as both string-like and numeric-like across all uses
- Non-primary locales must be structural subsets of the primary locale (messages, terms, and their attributes)
- Missing entries in secondary locales are resolved against the primary locale at code generation time; runtime fallback chains are not supported

A full walkthrough of supported FTL syntax is available in the example locale
files.

- [`locales/en-US.ftl`](example/locales/en-US.ftl)
- [`locales/zh-CN.ftl`](example/locales/zh-CN.ftl)
- [`locales/ja-JP.ftl`](example/locales/ja-JP.ftl)

## Usage

```toml
[build-dependencies]
ftl-codegen = "0.1"

[dependencies]
unic-langid = { version = "0.9", features = ["unic-langid-macros"] }
```

```rust
// build.rs
fn main() {
    println!("cargo:rerun-if-changed=locales");

    ftl_codegen::generator()
        .locales_dir("locales")
        .default_lang("en-US")
        .module_path("i18n")
        .output_path(Path::new(&env::var("OUT_DIR").unwrap()).join("i18n_gen.rs"))
        .generate();
}
```

```rust
// src/main.rs
use crate::i18n::*;
 
pub mod i18n {
    include!(concat!(env!("OUT_DIR"), "/i18n_gen.rs"));
}

fn main() {
    set_lang(Lang::EnUs);
    println!("{}", t!(hello_world()));
    println!("{}", t!(hello("World")));
    println!("{}", t!(files(1)));
}
```

## Features

### `builtin` feature (enabled by default)

When enabled, `NUMBER()` and `DATETIME()` built-in functions are automatically registered with locale-aware formatting backed by CLDR data.

```toml
[build-dependencies]
ftl-codegen = { version = "0.1", features = ["builtin"] }

[dependencies]
ftl-builtin = { version = "0.1" }
unic-langid = { version = "0.9", features = ["unic-langid-macros"] }
```

## Custom built-in functions

Register your own via `ftl_builtin!` macro and `.register()`:

```rust
// build.rs
use ftl_codegen::BuiltInFuncDef;

fn main() {
    ftl_codegen::generator()
        .register(test_builtin())
        .generate();
}

fn test_builtin() -> BuiltInFuncDef {
    ftl_codegen::ftl_builtin! {
        Test(FluentNum) {
            operator: String,
            operand: f64,
        }
        impl |this, out, _lang| {
            use std::fmt::Write;
            let result = match this.operator.as_deref() {
                Some("+") => *this.value + this.operand.unwrap_or(0.0),
                Some("-") => *this.value - this.operand.unwrap_or(0.0),
                _ => *this.value,
            };
            write!(out, "{}", result).unwrap();
        }
    }
}
```

Use in FTL:
```fluent
result = { TEST($value, operator: "+", operand: 10) }
```

The `impl |this, out, lang|` block receives `this` (the built-in struct with `value` and named args), an output `String`, and the `Lang` enum for locale. Omit `lang` if the function doesn't need locale.

Custom built-ins **do not** require the `builtin` feature.

## Message Compilation

**Plain text -> `&'static str`**

```fluent
settings = Settings
```

```rust
#[inline]
pub fn settings() -> &'static str { "Settings" }
```

**Variable interpolation -> Pre-sized `String`**

```fluent
hello = Hello, { $name }!
```

```rust
#[inline]
pub fn hello(name: &str) -> String {
    let cap = 7 + name.len() + 1;
    let mut s = String::with_capacity(cap);
    s.push_str("Hello, ");
    s.push_str(name);
    s.push_str("!");
    s
}
```

**Built-in function call -> `write_to` on builtin type**

```fluent
item-count = You have { NUMBER($count) } items.
```

```rust
#[inline]
pub fn item_count(count: impl Into<Number>) -> String {
    let mut s = String::with_capacity(9 + 32 + 7);
    s.push_str("You have ");
    count.into().write_to(&mut s);
    s.push_str(" items.");
    s
}
```

**Select expressions -> `match` on `FluentNum` or `&str`**

```fluent
files =
    { $count ->
        [one] 1 file
       *[other] { $count } files
    }
welcome =
    { $gender ->
        [male] Welcome, sir!
       *[other] Welcome!
    }
```

Numeric selector via `FluentNum`:

```rust
#[inline]
pub fn files(count: impl Into<FluentNum>) -> String {
    let count: FluentNum = count.into();
    match *count {
        1.0 => "1 file".to_string(),
        _ => {
            let mut s = String::with_capacity(32 + 6);
            write!(&mut s, "{}", count).unwrap();
            s.push_str(" files");
            s
        },
    }
}
```

String selector via `&str`:

```rust
#[inline]
pub fn user_greeting(gender: &str) -> String {
    match gender {
        "male" => "Welcome, sir!".to_string(),
        "female" => "Welcome, ma'am!".to_string(),
        _ => "Welcome!".to_string(),
    }
}
```

**Message reference -> compile-time inlined**

```fluent
app-name = My Application
about-app = About { app-name }
```

```rust
#[inline]
pub fn about_app() -> &'static str { "About My Application" }
```

**Dynamic message ref -> free variable propagation**

```fluent
name = { $user }
welcome-user = Welcome, { name }!
```

```rust
#[inline]
pub fn welcome_user(user: &str) -> String {
    let cap = 9 + user.len() + 1;
    let mut s = String::with_capacity(cap);
    s.push_str("Welcome, ");
    s.push_str(user);
    s.push_str("!");
    s
}
```

**Term reference -> compile-time inlined**

```fluent
-brand-name = My Application
welcome-term = Welcome to { -brand-name }
```

```rust
#[inline]
pub fn welcome_term() -> &'static str { "Welcome to My Application." }
```

**Parameterized term -> compile-time inlined**

```fluent
-brand =
    { $case ->
       *[nominative] MyBrand
        [genitive] MyBrand's
    }
about-brand = About { -brand(case: "genitive") }
```

```rust
#[inline]
pub fn about_brand(case: &str) -> String {
    match case {
        "genitive" => "MyBrand's".to_string(),
        _ => "MyBrand".to_string(),
    }
}
```

**Message attributes -> flattened functions**

```fluent
save =
    .label = Save { product }
    .tooltip = Save the current { $target }
```

```rust
#[inline]
pub fn save__label() -> &'static str { "Save Zed" }

#[inline]
pub fn save__tooltip(target: &str) -> String {
    let cap = 17 + target.len();
    let mut s = String::with_capacity(cap);
    s.push_str("Save the current ");
    s.push_str(target);
    s
}
```

**Inline attribute ref -> compile-time inlined**

```fluent
login-input = Default value
    .placeholder = Enter your email
attr-ref-demo = Placeholder: { login-input.placeholder }
```

```rust
#[inline]
pub fn attr_ref_demo() -> &'static str { "Placeholder: Enter your email" }
```

**Term attribute select -> compile-time inlined**

```fluent
-brand-aurora = Aurora
    .gender = feminine
-brand-gender =
    { -brand-aurora.gender ->
        [masculine] Mr.
        [feminine] Ms.
       *[other] Mx.
    }
attr-select-demo = Title: { -brand-gender }
```

```rust
#[inline]
pub fn attr_select_demo() -> &'static str { "Title: Ms." }
```

## License

MIT License, see [LICENSE](LICENSE).
CLDR json data (`cldr/`) is Unicode-3.0.
