# FTL

*Faster than light.*

Compile-time conversion of Fluent (.ftl) translation files to type-safe Rust functions. No parser, no hash table at runtime.

**This library is under active development and is not yet stable. ANY API MAY CHANGE WITHOUT NOTICE!**

[![rust 1.70+](https://img.shields.io/badge/rust-1.70%2B-orange?style=flat&logo=rust)](https://www.rust-lang.org)
[![license](https://img.shields.io/badge/license-MIT%20%7C%20Unicode--3.0-blue?style=flat)](#license)

## Benchmarks

Test Environment: AMD AI 9 H 365, `cargo bench -p example`.

- get locale: <500 ps
- pure text translate: <500 ps
- complex translate: <50 ns

## Limitations

- Fluent built-in functions (`NUMBER()`, `DATETIME()`, etc.) are currently unsupported
- Partially-formatted variables (`FluentDateTime` / `FluentNumber`) are unsupported
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

**Variable interpolation -> Pre-sized `String`**

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

**Select expressions -> `match` on numeric or `&str`**

```fluent
files =
    { $count ->
        [one] 1 file
       *[other] { $count } files
    }
welcome =
    { $gender ->
        [male] Welcome, sir
       *[other] Welcome
    }
```

Numeric (`$count`) via `impl Into<FluentNum>` — accepts all primitive numeric types:

```rust
pub fn files(count: impl Into<FluentNum>) -> String {
    let count: FluentNum = count.into();
    match *count {
        1.0 => "1 file".to_string(),
        _ => {
            let cap = 32 + 6; // 32 is a conservative fixed bound to avoid runtime width calculation
            let mut s = String::with_capacity(cap);
            write!(&mut s, "{}", count).unwrap();
            s.push_str(" files");
            s
        }
    }
}
```

String (`$gender`) via `&str`:

```rust
pub fn welcome(gender: &str) -> String {
    match gender {
        "male" => "Welcome, sir".to_string(),
        _ => "Welcome".to_string(),
    }
}
```

*`FluentNum` — unified numeric type*

All `{ $count }` variables and `{ $n -> ... }` selectors use `FluentNum`, a thin wrapper around `f64` with `From` impls for every primitive numeric type.

> **Precision note:** `f64` (IEEE-754 double) can represent integers up to 2⁵³ exactly.
> Larger integer values may lose precision and fail to match exact numeric selector keys like `[0]` or `[42]`.
> Plural-category matching additionally relies on `i64`-based guards.
> Values outside the `i64` range are not guaranteed to produce meaningful category matches.

Match behavior for numeric selectors:

| Variant key | Generated pattern | Notes |
|---|---|---|
| `[0]` `[42]` | `0.0 =>`, `42.0 =>` | Exact `f64` match; `-0.0` also matches `0.0` |
| `[one]` (EN) | `1.0 =>` | Same as `[1]` for English |
| `[one]` (RU etc.) | `n if (n.trunc() as i64) % 10 == 1` | Truncated to `i64` first |
| `[few]` | `n if (n.trunc() as i64) % 10 >= 2 && ...` | Same i64 truncation |
| `[other]` | `_ =>` | Fallback (always required) |
| `NaN` / `±∞` | — | Non-finite values never participate in category matching; fall to `[other]` |

Plural-category matching is only evaluated for finite integer-valued inputs.
Fractional values always fall through to the default variant.
`FluentNum` also implements `Display`, `PartialEq<f64>`, and `Deref<Target=f64>` for direct use in format strings.

**Message reference -> compile-time inlined**

```fluent
app-name = Zed
about = About { app-name }
```

```rust
pub fn about() -> &'static str { "About Zed" }
```

**Dynamic message ref -> free variable propagation**

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

**Parameterized term -> compile-time inlined**

```fluent
-brand-name = { $case } Zed
about = About { -brand-name(case: "Awesome") }
```

```rust
pub fn about() -> &'static str { "About Awesome Zed" }
```

**Parameterized term with var -> Pre-sized `String`**

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

**Inline attribute ref -> compile-time inlined**

```fluent
msg = Hello
    .name = World
greeting = { msg.name }!
```

```rust
pub fn greeting() -> &'static str { "World!" }
```

**Term attribute select -> compile-time inlined**

```fluent
-brand = Aurora
    .gender = feminine
-greeting =
    { -brand.gender ->
        [masculine] Mr.
        [feminine] Ms.
       *[other] Mx.
    }
title = Title: { -greeting }
```

```rust
pub fn title() -> &'static str { "Title: Ms." }
```

## License

MIT License, see [LICENSE](LICENSE).
CLDR plural rule data (`codegen/cldr/`) is Unicode-3.0.
