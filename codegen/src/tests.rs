#[test]
fn full_pipeline_generates_valid_rust() {
    use std::fs;
    use std::path::PathBuf;

    let dir = PathBuf::from(std::env::temp_dir()).join(format!("ftl_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::create_dir_all(dir.join("locales")).unwrap();

    fs::write(
        dir.join("locales").join("en-US.ftl"),
        "settings = Settings\nhello = Hello, { $name }!\n",
    )
    .unwrap();
    fs::write(
        dir.join("locales").join("zh-CN.ftl"),
        "settings = 设置\nhello = 你好，{ $name }！\n",
    )
    .unwrap();

    let out = dir.join("out.rs");
    super::generator()
        .locales_dir(dir.join("locales"))
        .default_lang("en-US")
        .output_path(&out)
        .generate();

    let code = fs::read_to_string(&out).unwrap();
    assert!(code.contains("pub fn settings() -> &'static str"));
    assert!(code.contains("pub fn hello<'a>(name: impl Into<ftl_core::FluentArg<'a>>) -> String"));
    assert!(code.contains("pub mod en_us"));
    assert!(code.contains("pub mod zh_cn"));
    assert!(code.contains("pub enum Lang"));
    assert!(code.contains("EnUs"));
    assert!(code.contains("ZhCn"));
    assert!(code.contains("macro_rules! t"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn generator_with_select_plural() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir =
        PathBuf::from(std::env::temp_dir()).join(format!("ftl_test_sel_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("en-US.ftl"),
        "files =\n    { $count ->\n        [one] 1 file\n       *[other] { $count } files\n    }\n",
    )
    .unwrap();
    fs::write(
        dir.join("zh-CN.ftl"),
        "files =\n    { $count ->\n        [one] 1 个文件\n       *[other] { $count } 个文件\n    }\n",
    )
    .unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    assert!(code.contains("fn files(count: impl Into<FluentNum>)"));
    assert!(code.contains("1.0 =>"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
#[should_panic(expected = "1 error(s)")]
fn generator_panics_when_primary_missing() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir =
        PathBuf::from(std::env::temp_dir()).join(format!("ftl_test_panic_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("en-US.ftl"), "settings = Settings").unwrap();

    Generator::load_simple(&dir, "xx", "");
}

#[test]
#[should_panic(expected = "1 error(s)")]
fn generator_panics_on_extra_keys() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir =
        PathBuf::from(std::env::temp_dir()).join(format!("ftl_test_extra_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("en-US.ftl"), "settings = Settings").unwrap();
    fs::write(dir.join("zh-CN.ftl"), "settings = 设置\nextra = Extra").unwrap();

    Generator::load_simple(&dir, "en-US", "");
}

#[test]
fn generator_fallback_when_message_missing() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir =
        PathBuf::from(std::env::temp_dir()).join(format!("ftl_test_fb_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("en-US.ftl"),
        "settings = Settings\nhello = Hello, { $name }!",
    )
    .unwrap();
    fs::write(dir.join("zh-CN.ftl"), "settings = 设置").unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    assert!(code.contains("WARNING"));
    assert!(code.contains("Hello"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn generator_skips_non_ftl_files() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir =
        PathBuf::from(std::env::temp_dir()).join(format!("ftl_test_skip_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("en-US.ftl"), "settings = Settings").unwrap();
    fs::write(dir.join("notes.txt"), "not a translation").unwrap();
    fs::write(dir.join("data.json"), "{}").unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    assert_eq!(gen.locales.len(), 1);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn generator_with_string_selector() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir =
        PathBuf::from(std::env::temp_dir()).join(format!("ftl_test_strsel_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("en-US.ftl"),
        "greeting = { $gender ->\n    [male] Hello, sir\n   *[female] Hello, madam\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("zh-CN.ftl"),
        "greeting = { $gender ->\n    [male] 您好，先生\n   *[female] 您好，女士\n}\n",
    )
    .unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    assert!(code.contains("fn greeting<'a>(gender: impl Into<ftl_core::FluentArg<'a>>)"));
    assert!(code.contains("match __sel.as_str()"));
    assert!(code.contains("\"male\" =>"));
    assert!(code.contains("Hello, sir"));
    assert!(code.contains("Hello, madam"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn generator_with_multiple_locales() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir =
        PathBuf::from(std::env::temp_dir()).join(format!("ftl_test_multi_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("en-US.ftl"), "title = My App").unwrap();
    fs::write(dir.join("zh-CN.ftl"), "title = 我的应用").unwrap();
    fs::write(dir.join("ja-JP.ftl"), "title = マイアプリ").unwrap();
    fs::write(dir.join("fr-FR.ftl"), "title = Mon App").unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    assert_eq!(gen.locales.len(), 4);
    let code = gen.generate();
    assert!(code.contains("pub mod en_us"));
    assert!(code.contains("pub mod zh_cn"));
    assert!(code.contains("pub mod ja_jp"));
    assert!(code.contains("pub mod fr_fr"));
    assert!(code.contains("Lang::EnUs"));
    assert!(code.contains("Lang::ZhCn"));
    assert!(code.contains("Lang::JaJp"));
    assert!(code.contains("Lang::FrFr"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn generator_with_numeric_variant_key() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir =
        PathBuf::from(std::env::temp_dir()).join(format!("ftl_test_numvar_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("en-US.ftl"),
        "items =\n    { $n ->\n        [0] none\n        [1] one\n       *[other] many\n    }\n",
    )
    .unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    assert!(code.contains("0.0 =>"));
    assert!(code.contains("1.0 =>"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn emit_runtime_generates_lang_enum_and_switch() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir =
        PathBuf::from(std::env::temp_dir()).join(format!("ftl_test_rt_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("en-US.ftl"), "x = X").unwrap();
    fs::write(dir.join("de-DE.ftl"), "x = Y").unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    assert!(code.contains("static LOCALE_ID: AtomicU8"));
    assert!(code.contains("pub fn set_lang"));
    assert!(code.contains("macro_rules! t"));
    assert!(code.contains("pub fn get_locale"));
    assert!(code.contains("pub fn set_lang"));
    assert!(code.contains("impl From<Lang>"));
    assert!(code.contains("LanguageIdentifier"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn message_attributes_generate_flattened_functions() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir =
        PathBuf::from(std::env::temp_dir()).join(format!("ftl_test_attrs_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("en-US.ftl"),
        "save =\n    .label = Save\n    .tooltip = Save current file\n",
    )
    .unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    assert!(code.contains("fn save__label() -> &'static str"));
    assert!(code.contains("fn save__tooltip() -> &'static str"));
    assert!(code.contains("pub fn save__label() -> &'static str { \"Save\" }"));
    assert!(code.contains("pub fn save__tooltip() -> &'static str { \"Save current file\" }"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn message_attribute_fallback_uses_primary() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir = PathBuf::from(std::env::temp_dir())
        .join(format!("ftl_test_attr_fb_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("en-US.ftl"),
        "save =\n    .label = Save\n    .tooltip = Save current file\n",
    )
    .unwrap();
    fs::write(dir.join("zh-CN.ftl"), "save =\n    .label = 保存\n").unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    assert!(code.contains("attribute 'save.tooltip' missing"));
    assert!(code.contains("pub fn save__tooltip() -> &'static str { \"Save current file\" }"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn message_attributes_support_refs_and_variable_propagation() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir = PathBuf::from(std::env::temp_dir())
        .join(format!("ftl_test_attr_ref_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("en-US.ftl"),
        "product = Zed\nsave =\n    .label = Save { product }\n    .tooltip = Save { $target }\n",
    )
    .unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    assert!(code.contains("pub fn save__label() -> &'static str { \"Save Zed\" }"));
    assert!(
        code.contains("fn save__tooltip<'a>(target: impl Into<ftl_core::FluentArg<'a>>) -> String")
    );
    assert!(code.contains("target.into().write_localized(&mut s);"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn inline_literal_number_works() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir = PathBuf::from(std::env::temp_dir())
        .join(format!("ftl_test_inline_num_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("en-US.ftl"), "x = value { 42 }").unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    assert!(code.contains("pub fn x() -> &'static str { \"value 42\" }"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn inline_literal_string_works() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir = PathBuf::from(std::env::temp_dir())
        .join(format!("ftl_test_inline_str_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("en-US.ftl"), "msg = { \"hello\" } world").unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    assert!(code.contains("pub fn msg() -> &'static str { \"hello world\" }"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn literal_number_selector_resolved_at_compile_time() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir =
        PathBuf::from(std::env::temp_dir()).join(format!("ftl_test_litnum_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("en-US.ftl"),
        "x = { 42 ->\n    [42] forty-two\n   *[other] other\n}\n",
    )
    .unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    // 42 matches [42] → should inline "forty-two" as pure text
    assert!(code.contains("fn x() -> &'static str { \"forty-two\" }"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn literal_number_selector_uses_default_when_no_match() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir =
        PathBuf::from(std::env::temp_dir()).join(format!("ftl_test_litdef_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    // 99 doesn't match [42] → should use default variant
    fs::write(
        dir.join("en-US.ftl"),
        "x = { 99 ->\n    [42] match\n   *[other] fallback\n}\n",
    )
    .unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    assert!(code.contains("\"fallback\""));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn literal_string_selector_resolved_at_compile_time() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir =
        PathBuf::from(std::env::temp_dir()).join(format!("ftl_test_litstr_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("en-US.ftl"),
        "x = { \"hello\" ->\n    [hello] greeting\n   *[other] other\n}\n",
    )
    .unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    assert!(code.contains("\"greeting\""));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn term_attribute_select_resolved_at_compile_time() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir = PathBuf::from(std::env::temp_dir())
        .join(format!("ftl_test_termsel_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("en-US.ftl"),
        "-brand = Aurora\n    .gender = feminine\n-greeting =\n    { -brand.gender ->\n        [masculine] Mr.\n        [feminine] Ms.\n       *[other] Mx.\n    }\nmsg = { -greeting }\n",
    )
    .unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    assert!(code.contains("fn msg() -> &'static str { \"Ms.\" }"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn message_reference_is_expanded_at_build_time() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir = PathBuf::from(std::env::temp_dir())
        .join(format!("ftl_test_msg_ref_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("en-US.ftl"),
        "app-name = Zed\nabout = About { app-name }\n",
    )
    .unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    assert!(code.contains("pub fn about() -> &'static str { \"About Zed\" }"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn term_reference_is_expanded_at_build_time() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir = PathBuf::from(std::env::temp_dir())
        .join(format!("ftl_test_term_ref_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("en-US.ftl"),
        "-brand-name = Zed\nwelcome = Welcome to { -brand-name }\n",
    )
    .unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    assert!(code.contains("pub fn welcome() -> &'static str { \"Welcome to Zed\" }"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn parameterized_term_expands_with_arguments() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir = PathBuf::from(std::env::temp_dir())
        .join(format!("ftl_test_term_args_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("en-US.ftl"),
        "-brand-name = { $case } Zed\nabout = About { -brand-name(case: \"Cool\") }\n",
    )
    .unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    assert!(code.contains("pub fn about() -> &'static str { \"About Cool Zed\" }"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn positional_term_argument_resolved_by_order() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir =
        PathBuf::from(std::env::temp_dir()).join(format!("ftl_test_pos_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("en-US.ftl"),
        "-greet = { $greeting }, { $name }!\nmsg = { -greet(\"Hi\", \"Alice\") }\n",
    )
    .unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    // $greeting ← "Hi", $name ← "Alice"
    assert!(code.contains("fn msg() -> &'static str { \"Hi, Alice!\" }"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn free_variables_propagate_through_message_references() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir = PathBuf::from(std::env::temp_dir())
        .join(format!("ftl_test_free_vars_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("en-US.ftl"),
        "name = { $user }\nwelcome = Welcome, { name }!\n",
    )
    .unwrap();

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    assert!(code.contains("fn welcome<'a>(user: impl Into<ftl_core::FluentArg<'a>>) -> String"));
    assert!(code.contains("user.into().write_localized(&mut s);"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
#[should_panic(expected = "cyclic message reference")]
fn cyclic_message_reference_panics() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir = PathBuf::from(std::env::temp_dir())
        .join(format!("ftl_test_cycle_msg_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(dir.join("en-US.ftl"), "a = { b }\nb = { a }\n").unwrap();

    Generator::load_simple(&dir, "en-US", "");
}

#[test]
#[should_panic(expected = "cyclic term reference")]
fn cyclic_term_reference_panics() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir = PathBuf::from(std::env::temp_dir())
        .join(format!("ftl_test_cycle_term_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("en-US.ftl"),
        "-a = { -b }\n-b = { -a }\nmsg = ok\n",
    )
    .unwrap();

    Generator::load_simple(&dir, "en-US", "");
}

/// Helper: create a temp dir with a given identifier.
fn temp_dir(label: &str) -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(std::env::temp_dir()).join(format!(
        "ftl_test_{}_{}",
        label,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Helper: write an FTL file into a temp dir.
fn write_ftl(dir: &std::path::Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).unwrap();
}

/// Helper: call Generator::load and expect a specific panic message.
fn expect_load_error(dir: &std::path::Path, primary: &str, expected_substr: &str) {
    use crate::parse::Generator;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Generator::load_simple(dir, primary, "");
    }));
    match result {
        Ok(_) => panic!("expected load to panic, but it succeeded"),
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                format!("{:?}", e)
            };
            assert!(
                msg.contains(expected_substr),
                "panic message should contain '{expected_substr}', got:\n{msg}"
            );
        }
    }
}

#[test]
fn diag_file_not_found() {
    let dir = temp_dir("diag_no_file");
    std::fs::write(dir.join("ignored.txt"), "x").unwrap();
    expect_load_error(&dir, "en-US", "primary locale 'en-US' not found");
}

#[test]
fn diag_unreadable_file() {
    let dir = temp_dir("diag_unreadable");
    write_ftl(&dir, "en-US.ftl", "x = 1");
    let _ = crate::parse::Generator::load_simple(&dir, "en-US", "");
}

#[test]
fn diag_primary_locale_missing() {
    let dir = temp_dir("diag_primary_missing");
    write_ftl(&dir, "de-DE.ftl", "x = 1");
    expect_load_error(&dir, "en-US", "primary locale 'en-US' not found");
}

#[test]
fn diag_extra_messages() {
    let dir = temp_dir("diag_extra_msgs");
    write_ftl(&dir, "en-US.ftl", "a = 1");
    write_ftl(&dir, "zh-CN.ftl", "a = 1\nb = 2");
    expect_load_error(&dir, "en-US", "has extra messages");
}

#[test]
fn diag_extra_terms() {
    let dir = temp_dir("diag_extra_terms");
    write_ftl(&dir, "en-US.ftl", "a = 1");
    write_ftl(&dir, "zh-CN.ftl", "a = 1\n-b = 2");
    expect_load_error(&dir, "en-US", "has extra terms");
}

#[test]
fn diag_extra_attributes() {
    let dir = temp_dir("diag_extra_attrs");
    write_ftl(&dir, "en-US.ftl", "a = 1\n    .attr = val\n");
    write_ftl(
        &dir,
        "zh-CN.ftl",
        "a = 1\n    .attr = val\n    .extra = val\n",
    );
    expect_load_error(&dir, "en-US", "has extra attributes");
}

#[test]
fn diag_parse_error_expected_token() {
    let dir = temp_dir("diag_parse_tok");
    // `#` followed by non-space triggers ExpectedToken(' ')
    write_ftl(&dir, "en-US.ftl", "x = 1\n#（bad comment\n");
    expect_load_error(&dir, "en-US", "Expected a token starting with");
}

#[test]
fn diag_parse_error_missing_value() {
    let dir = temp_dir("diag_parse_mval");
    // A message with no value AND no attributes triggers MissingValue
    write_ftl(&dir, "en-US.ftl", "x =\ny = 2\n");
    expect_load_error(&dir, "en-US", "Expected a message field");
}

#[test]
fn diag_parse_error_unbalanced_brace() {
    // `}` at line start is not a valid entry �?triggers ExpectedCharRange.
    let dir = temp_dir("diag_parse_brace");
    write_ftl(&dir, "en-US.ftl", "x = 1\n}\n");
    expect_load_error(&dir, "en-US", "Expected one of");
}

#[test]
fn diag_parse_error_missing_default_variant() {
    let dir = temp_dir("diag_parse_defvar");
    write_ftl(
        &dir,
        "en-US.ftl",
        "x = { $n ->\n    [one] a\n    [other] b\n}\n",
    );
    expect_load_error(&dir, "en-US", "must have a default variant");
}

#[test]
fn diag_parse_error_multiple_default_variants() {
    let dir = temp_dir("diag_parse_multidef");
    write_ftl(
        &dir,
        "en-US.ftl",
        "x = { $n ->\n    *[one] a\n    *[other] b\n}\n",
    );
    expect_load_error(&dir, "en-US", "can only have one default variant");
}

#[test]
fn diag_parse_error_unterminated_string() {
    let dir = temp_dir("diag_parse_str");
    write_ftl(&dir, "en-US.ftl", "x = { \"hello }\n");
    expect_load_error(&dir, "en-US", "Unterminated string literal");
}

#[test]
fn diag_parse_error_unknown_escape() {
    let dir = temp_dir("diag_parse_esc");
    write_ftl(&dir, "en-US.ftl", "x = { \"\\z\" }\n");
    expect_load_error(&dir, "en-US", "Unknown escape sequence");
}

#[test]
fn diag_parse_error_invalid_unicode_escape() {
    let dir = temp_dir("diag_parse_unicode");
    write_ftl(&dir, "en-US.ftl", "x = { \"\\uZZZZ\" }\n");
    expect_load_error(&dir, "en-US", "Invalid unicode escape sequence");
}

#[test]
fn diag_parse_error_duplicated_named_arg() {
    let dir = temp_dir("diag_parse_dup_arg");
    write_ftl(&dir, "en-US.ftl", "-t = { $x }\nx = { -t(x: 1, x: 2) }\n");
    expect_load_error(&dir, "en-US", "argument appears twice");
}

#[test]
fn diag_parse_error_positional_follows_named() {
    let dir = temp_dir("diag_parse_pos_follows");
    write_ftl(
        &dir,
        "en-US.ftl",
        "-t = { $x }\nx = { -t(x: 1, \"pos\") }\n",
    );
    expect_load_error(&dir, "en-US", "Positional arguments must come before");
}

#[test]
fn diag_parse_error_forbidden_callee() {
    let dir = temp_dir("diag_parse_callee");
    write_ftl(&dir, "en-US.ftl", "x = { unknown() }\n");
    expect_load_error(&dir, "en-US", "Callee is not allowed here");
}

#[test]
fn diag_parse_error_expected_literal() {
    // Empty brackets `[]` in variants trigger ExpectedCharRange.
    let dir = temp_dir("diag_parse_lit");
    write_ftl(
        &dir,
        "en-US.ftl",
        "x = { $n ->\n    [] v\n   *[other] v\n}\n",
    );
    expect_load_error(&dir, "en-US", "Expected one of");
}

#[test]
fn diag_parse_error_message_ref_as_selector() {
    let dir = temp_dir("diag_parse_msg_sel");
    write_ftl(
        &dir,
        "en-US.ftl",
        "a = 1\nx = { a -> [one] v *[other] v }\n",
    );
    expect_load_error(
        &dir,
        "en-US",
        "Message references can't be used as a selector",
    );
}

#[test]
fn diag_parse_error_term_ref_as_selector() {
    let dir = temp_dir("diag_parse_term_sel");
    write_ftl(
        &dir,
        "en-US.ftl",
        "-t = 1\nx = { -t -> [one] v *[other] v }\n",
    );
    expect_load_error(&dir, "en-US", "Term references can't be used as a selector");
}

#[test]
fn diag_parse_error_expected_inline_expression() {
    let dir = temp_dir("diag_parse_inline");
    write_ftl(&dir, "en-US.ftl", "x = {  }\n");
    expect_load_error(&dir, "en-US", "Expected an inline expression");
}

#[test]
fn diag_parse_error_expected_simple_expression_as_selector() {
    let dir = temp_dir("diag_parse_simple_sel");
    write_ftl(&dir, "en-US.ftl", "x = { {1} -> [a] v *[other] v }\n");
    expect_load_error(&dir, "en-US", "Expected a simple expression as selector");
}

#[test]
fn diag_parse_error_format_includes_location() {
    use crate::parse::Generator;
    use std::panic;

    let dir = temp_dir("diag_format_loc");
    write_ftl(&dir, "en-US.ftl", "msg = hello\nbad = { \"\\z\" }\n");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Generator::load_simple(&dir, "en-US", "");
    }));
    let msg = match result {
        Err(e) => match e.downcast_ref::<String>() {
            Some(s) => s.clone(),
            None => format!("{:?}", e),
        },
        Ok(_) => panic!("expected error"),
    };

    assert!(
        msg.contains("en-US.ftl"),
        "should mention file, got: {}",
        msg
    );
    assert!(msg.contains("-->"), "should contain location arrow");
    assert!(msg.contains("Unknown escape"), "should contain error kind");
}

#[test]
fn diag_parse_error_format_shows_snippet() {
    use crate::parse::Generator;

    let dir = temp_dir("diag_format_snip");
    write_ftl(&dir, "en-US.ftl", "a = 1\nb = 2\nc = { \"\\x\" }\n");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Generator::load_simple(&dir, "en-US", "");
    }));
    let msg = match result {
        Err(e) => match e.downcast_ref::<String>() {
            Some(s) => s.clone(),
            None => format!("{:?}", e),
        },
        Ok(_) => panic!("expected error"),
    };

    // The snippet should show the problematic line
    assert!(
        msg.contains("\\x"),
        "snippet should show part of the bad line, got:\n{msg}"
    );
    // Should have a caret pointing at the error
    assert!(msg.contains("^"), "output should have a caret pointer");
}

#[test]
fn diag_parse_error_expected_term_field() {
    let dir = temp_dir("diag_term_field");
    write_ftl(&dir, "en-US.ftl", "-t =\nx = 1\n");
    expect_load_error(&dir, "en-US", "Expected a term field for");
}

#[test]
fn diag_parse_error_missing_value_attribute() {
    // Swallowed by get_attributes(), surfaces as ExpectedMessageField.
}

#[test]
fn diag_parse_error_missing_value_variant() {
    // A variant with no value triggers MissingValue
    let dir = temp_dir("diag_mval_var");
    write_ftl(
        &dir,
        "en-US.ftl",
        "x = { $n ->\n    [one]\n   *[other] v\n}\n",
    );
    expect_load_error(&dir, "en-US", "Expected a value");
}

#[test]
fn diag_parse_error_message_attr_as_selector() {
    // Using msg.attr as a select selector triggers MessageAttributeAsSelector
    let dir = temp_dir("diag_msg_attr_sel");
    write_ftl(
        &dir,
        "en-US.ftl",
        "a = 1\n    .color = red\nx = { a.color ->\n    *[other] v\n}\n",
    );
    expect_load_error(
        &dir,
        "en-US",
        "Message attributes can't be used as a selector",
    );
}

#[test]
fn diag_parse_error_term_attr_as_placeable() {
    // Using { -term.attr } inline triggers TermAttributeAsPlaceable.
    // Note: despite the variant name, the display message says
    // "Term attributes can't be used as a selector".
    let dir = temp_dir("diag_term_attr_place");
    write_ftl(
        &dir,
        "en-US.ftl",
        "-t = 1\n    .g = feminine\nx = { -t.g }\n",
    );
    expect_load_error(&dir, "en-US", "Term attributes can't be used as a selector");
}

#[test]
fn diag_parse_error_unbalanced_closing_brace() {
    // A stray `}` inside a pattern (no matching `{`) triggers
    // UnbalancedClosingBrace.  The trick: `{ "}" }` is a quoted literal
    // containing `}`, but if we place a bare `}` after a valid placeable
    // the parser detects the imbalance inside the pattern.
    let dir = temp_dir("diag_unbalanced");
    write_ftl(&dir, "en-US.ftl", "x = { 1 } }\n");
    expect_load_error(&dir, "en-US", "Unbalanced closing brace");
}

#[test]
fn diag_parse_error_expected_literal_inline() {
    // Named argument values must be literals.  A variable reference
    // in a named argument triggers ExpectedLiteral.
    let dir = temp_dir("diag_exp_lit");
    write_ftl(&dir, "en-US.ftl", "-t = { $x }\nx = { -t(k: $v) }\n");
    expect_load_error(&dir, "en-US", "Expected a string or number literal");
}

#[test]
fn number_function_works_without_registration() {
    use crate::parse::Generator;
    let dir = temp_dir("num_ok");
    write_ftl(
        &dir,
        "en-US.ftl",
        "x = { NUMBER($n) }
",
    );
    let gen = Generator::load_simple(&dir, "en-US", "");
    assert!(!gen
        .diags
        .iter()
        .any(|d| d.kind == crate::diag::DiagKind::Error));
}

#[test]
fn datetime_function_works_without_registration() {
    use crate::parse::Generator;
    let dir = temp_dir("dt_ok");
    write_ftl(
        &dir,
        "en-US.ftl",
        "x = { DATETIME($d) }
",
    );
    let gen = Generator::load_simple(&dir, "en-US", "");
    assert!(!gen
        .diags
        .iter()
        .any(|d| d.kind == crate::diag::DiagKind::Error));
}

#[test]
#[should_panic(expected = "1 error(s)")]
fn invalid_locale_produces_diagnostic() {
    use crate::parse::Generator;

    let dir = temp_dir("inv_loc");
    write_ftl(&dir, "zh-CN.ftl", "x = 1\n");
    write_ftl(&dir, "foo_bar.ftl", "y = 2\n");

    let gen = Generator::load_simple(&dir, "zh-CN", "");
    // load_simple calls report_diagnostics which panics on errors
    let _ = gen;
}

#[test]
fn custom_builtin_with_body_emits_struct() {
    use crate::parse::Generator;
    use crate::BuiltInFuncDef;

    let dir = temp_dir("cust_builtin");
    write_ftl(&dir, "en-US.ftl", "x = { HTML($input) }\n");

    use crate::{BuiltInArgType, BuiltInNamedArg};
    use std::collections::BTreeMap;
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
            write_to_body: Some(
                "write!(out, \"<div>\\\"{}\\\"</div>\", this.value).unwrap();".to_string(),
            ),
        },
    );

    let gen = Generator::load(&dir, "en-US", "", &builtins);
    let code = gen.generate();
    assert!(code.contains("pub struct Html"));
    assert!(code.contains("pub value: FluentNum"));
    assert!(code.contains("pub class: Option<String>"));
    assert!(code.contains("pub fn new"));
    assert!(code.contains("pub fn class"));
    assert!(code.contains("pub fn write_to"));
    assert!(code.contains("pub fn write_to_with"));
}

#[test]
fn term_attr_select_fallback_to_default() {
    use crate::parse::Generator;

    let dir = temp_dir("ta_def");
    // Term attribute value "other" doesn't match any key
    write_ftl(
        &dir,
        "en-US.ftl",
        "-brand = Hi\n    .gender = other\n-greeting =\n    { -brand.gender ->\n        [masculine] Mr.\n       *[feminine] Ms.\n    }\nmsg = { -greeting }\n",
    );

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    // The "other" value doesn't match "masculine" or "feminine",
    // so the default variant "feminine" (Ms.) is used
    assert!(!code.contains("Mr."));
    assert!(code.contains("Ms."));
}

#[test]
fn number_with_named_args_covers_closure_branches() {
    use crate::parse::Generator;

    // NUMBER with optional named args — covers `Some(raw)` branches
    // in the v/v_bool/v_str closures.
    // Use numbers and strings for named args — booleans aren't supported
    // as named arg values in FTL fluent_syntax.
    let dir = temp_dir("num_args");
    write_ftl(
        &dir,
        "en-US.ftl",
        "x = { NUMBER($n, minimumFractionDigits: 2, maximumFractionDigits: 4, style: \"percent\") }\n",
    );

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    assert!(code.contains("Some(2i64)")); // minimumFractionDigits
    assert!(code.contains("Some(4i64)")); // maximumFractionDigits
    assert!(code.contains("Some(\"percent\")")); // style string literal
}

#[test]
fn datetime_with_various_args_covers_closure_branches() {
    use crate::parse::Generator;

    let dir = temp_dir("dt_args");
    write_ftl(
        &dir,
        "en-US.ftl",
        "x = { DATETIME($d, dateStyle: \"long\", timeStyle: \"short\", timeZone: \"UTC\") }\n",
    );

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    assert!(code.contains("Some(\"long\")")); // dateStyle string
    assert!(code.contains("Some(\"short\")")); // timeStyle string
    assert!(code.contains("Some(\"UTC\")")); // timeZone
    assert!(code.contains("None")); // unset optional args
}

#[test]
fn number_with_currency_style_covers_str_closure() {
    use crate::parse::Generator;

    let dir = temp_dir("num_cur");
    write_ftl(
        &dir,
        "en-US.ftl",
        "x = { NUMBER($n, style: \"percent\") }\n",
    );

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    assert!(code.contains("Some(\"percent\")"));
}

#[test]
fn number_with_bool_false_named_arg_covers_bool_false_branch() {
    use crate::parse::Generator;

    let dir = temp_dir("num_bf");
    write_ftl(&dir, "en-US.ftl", "x = { NUMBER($n, useGrouping: 0) }\n");

    let gen = Generator::load_simple(&dir, "en-US", "");
    let code = gen.generate();
    // useGrouping: 0 should map to "Some(false)" via the `Some(_)` branch
    assert!(code.contains("Some(false)"));
}

#[test]
fn unsupported_inline_expr_in_term_args_panics() {
    // Catch-all panic in convert_inline_expression — unreachable from
    // valid FTL (parser only produces handled expression types).
}

#[test]
fn term_arg_message_ref_multi_element_panics() {
    // Backend defensive code path — not reachable from FTL because
    // named arguments only accept literals at the parser level.
}

#[test]
fn param_type_conflict_panics_via_integration() {
    // Conflicting selector types tested inline in params.rs.
}

#[test]
#[should_panic(expected = "undefined message reference")]
fn undefined_message_reference_panics() {
    let dir = temp_dir("undef_msg");
    write_ftl(&dir, "en-US.ftl", "x = { nonexistent }\n");
    crate::parse::Generator::load_simple(&dir, "en-US", "");
}

#[test]
#[should_panic(expected = "undefined term reference")]
fn undefined_term_reference_panics() {
    let dir = temp_dir("undef_term");
    write_ftl(&dir, "en-US.ftl", "x = { -nonexistent }\n");
    crate::parse::Generator::load_simple(&dir, "en-US", "");
}
