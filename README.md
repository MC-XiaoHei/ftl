# FTL

*Faster than light.*

Compile-time conversion of Fluent (.ftl) translation files to type-safe Rust functions. No parser, no hash table at runtime.

[![rust 1.70+](https://img.shields.io/badge/rust-1.70%2B-orange?style=flat&logo=rust)](https://www.rust-lang.org)
[![license](https://img.shields.io/badge/license-MIT%20%7C%20Unicode--3.0-blue?style=flat)](#license)

## Benchmarks

Test Environment: AMD AI 9 H 365, `cargo bench -p example`.

- pure text: <500 ps
- get locale: <500 ps
- translate: <50 ns

## Limitations

A full walkthrough of supported and unsupported FTL syntax is available in
the example locale files:

- [`locales/en-US.ftl`](example/locales/en-US.ftl)
- [`locales/zh-CN.ftl`](example/locales/zh-CN.ftl)
- [`locales/ja-JP.ftl`](example/locales/ja-JP.ftl)

Unsupported features are commented out with explanations.

Currently, only a subset of Fluent syntax is supported (see example `.ftl` files
for a full walkthrough).  Unsupported features:

- Fluent built-in functions (`NUMBER()`, `DATETIME()`, etc.)
- Positional term arguments
- Partially-formatted variables (`FluentDateTime` / `FluentNumber`)
- Function calls as selectors
- Term attribute references (`-term.attr`)
- Non-variable selectors (e.g. `{ 42 -> ... }`)
- Ordinal plural via `NUMBER(…, type: "ordinal")`

**Select expressions**

- Select selector must be a variable reference (`{ $var -> ... }`). Other selector types are rejected at build time.
- Select selector type is inferred: numeric variant keys or plural categories (`[one]`, `[few]`, etc.) → `usize`; string variant keys like `[male]` / `[female]` → `&str`.
- Cyclic references between messages or terms are detected and rejected at build time.
- Parameters with conflicting inferred types (e.g. used as both `&str` and `usize`) are detected and rejected at build time.

**Locales**

- Non-primary locales must be a subset of the primary locale's message keys, terms, and attributes. Extra keys will report a build error.
- Missing messages in secondary locales fall back to the primary locale's content with a comment in generated code — no runtime fallback mechanism.

## Usage

```toml
[build-dependencies]
ftl-codegen = "0.1"
```

```rust
// build.rs
use std::path::Path;

ftl_codegen::generate(
    "locales",
    Path::new(&std::env::var("OUT_DIR").unwrap()).join("i18n_gen.rs"),
    "en-US",
);
```

```rust
// src/main.rs
include!(concat!(env!("OUT_DIR"), "/i18n_gen.rs"));

fn main() {
    set_lang(Lang::EnUs);
    println!("{}", t!(hello_world()));
    println!("{}", t!(hello("World")));
    println!("{}", t!(files(1)));
}
```

## Message Compilation

**Plain text -> `&'static str`**

```fluent
hello_world = Hello, World!
msg = { "hello" } world
count = { 42 }
```

```rust
pub fn hello_world() -> &'static str { "Hello, World!" }
pub fn msg() -> &'static str { "hello world" }
pub fn count() -> &'static str { "42" }
```

**Variable interpolation -> Pre-allocated `String`**

```fluent
hello = Hello, { $name }!
```

```rust
pub fn hello(name: &str) -> String {
    let cap = 7 + name.len() + 1;
    let mut s = String::with_capacity(cap);
    s.push_str("Hello, ");
    s.push_str(name);
    s.push_str("!");
    s
}
```

**Select (plural) -> `match`**

```fluent
files =
    { $count ->
        [one] 1 file
       *[other] { $count } files
    }
```

```rust
pub fn files(count: usize) -> String {
    match count {
        1 => "1 file".to_string(),
        _ => {
            let cap = if count == 0 { 1 } else { count.ilog10() as usize + 1 } + 6;
            let mut s = String::with_capacity(cap);
            write!(&mut s, "{}", count).unwrap();
            s.push_str(" files");
            s
        }
    }
}
```

**String select -> `match` with `&str` keys**

```fluent
welcome =
    { $gender ->
        [male] Welcome, sir
       *[other] Welcome
    }
```

```rust
pub fn welcome(gender: &str) -> String {
    match gender {
        "male" => "Welcome, sir".to_string(),
        _ => "Welcome".to_string(),
    }
}
```

**Message reference -> compile-time inlined**

```fluent
app-name = Zed
about = About { app-name }
```

```rust
pub fn about() -> &'static str { "About Zed" }
```

**Dynamic message reference inlining -> free variable propagation**

```fluent
name = { $user }
welcome = Welcome, { name }!
```

```rust
pub fn welcome(user: &str) -> String {
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
-brand-name = Zed
welcome = Welcome to { -brand-name }
```

```rust
pub fn welcome() -> &'static str { "Welcome to Zed" }
```

**Parameterized term -> compile-time substitution**

```fluent
-brand-name = { $case } Zed
about = About { -brand-name(case: "Awesome") }
```

```rust
pub fn about() -> &'static str { "About Awesome Zed" }
```

**Parameterized term with variable argument**

```fluent
-brand-name = { $case } Zed
about = About { -brand-name(case: $variant) }
```

```rust
pub fn about(variant: &str) -> String {
    let cap = 7 + variant.len() + 4;
    let mut s = String::with_capacity(cap);
    s.push_str("About ");
    s.push_str(variant);
    s.push_str(" Zed");
    s
}
```

**Message attributes -> flattened functions**

```fluent
save =
    .label = Save
    .tooltip = Save current file
```

```rust
pub fn save__label() -> &'static str { "Save" }
pub fn save__tooltip() -> &'static str { "Save current file" }
```

## License

MIT License, see [LICENSE](LICENSE).
CLDR plural rule data (`codegen/cldr/`) is Unicode-3.0.
