# FTL

Compile-time conversion of Fluent (.ftl) translation files to type-safe Rust functions. No parser, no hash table at runtime.

[![rust 1.70+](https://img.shields.io/badge/rust-1.70%2B-orange?style=flat&logo=rust)](https://www.rust-lang.org)
[![license](https://img.shields.io/badge/license-MIT%20%7C%20Unicode--3.0-blue?style=flat)](#license)

---

## Benchmarks

Test Environment: AMD AI 9 H 365, `cargo bench -p example`.

- settings (pure text): ~460 ps
- hello (short &str): ~35 ns
- hello (long &str): ~35 ns
- files (select one): ~25 ns
- files (select other): ~60 ns
- get_locale (id match): ~460 ps

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
    println!("{}", t!(hello("MC_XiaoHei")));
    println!("{}", t!(files(114514)));
}
```

## Message Compilation

**Plain text -> `&'static str`**

```ftl
hello_world = Hello, World!
```

```rust
pub fn hello_world() -> &'static str { "Hello, World!" }
```

**Variable interpolation -> Pre-allocated `String`**

```ftl
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

```ftl
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
            let cap = count.to_string().len() + 6;
            let mut s = String::with_capacity(cap);
            s.push_str(&count.to_string());
            s.push_str(" files");
            s
        }
    }
}
```

## License

MIT License, see [LICENSE](LICENSE).
CLDR plural rule data (`codegen/cldr/`) is Unicode-3.0.
