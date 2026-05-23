# CLDR Plural Rules

This directory contains CLDR (Common Locale Data Repository) plural rule data published by the Unicode Consortium.

## Source

https://github.com/unicode-org/cldr-json/tree/main/cldr-json/cldr-core/supplemental

- **File**: `plurals.json`
- **Version**: CLDR 48 / Unicode 16.0.0
- **License**: Unicode-3.0 (see `LICENSE`)

## Usage

`plurals.json` is parsed at build time by `ftl-codegen` to generate Rust match arm guards for CLDR plural categories.
It is embedded in the library via `include_str!` and deserialized with `serde_json`.
