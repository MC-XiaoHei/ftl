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
    super::generate(dir.join("locales"), &out, "en-US");

    let code = fs::read_to_string(&out).unwrap();
    assert!(code.contains("pub fn settings() -> &'static str"));
    assert!(code.contains("pub fn hello(name: &str) -> String"));
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

    let gen = Generator::load(&dir, "en-US");
    let code = gen.generate();
    assert!(code.contains("fn files(count: usize) -> String"));
    assert!(code.contains("match count"));
    assert!(code.contains("1 =>"));
    assert!(code.contains("_ =>"));

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

    Generator::load(&dir, "xx");
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

    Generator::load(&dir, "en-US");
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

    let gen = Generator::load(&dir, "en-US");
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

    let gen = Generator::load(&dir, "en-US");
    assert_eq!(gen.locales.len(), 1);

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

    let gen = Generator::load(&dir, "en-US");
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

    let gen = Generator::load(&dir, "en-US");
    let code = gen.generate();
    assert!(code.contains("0 =>"));
    assert!(code.contains("1 =>"));

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

    let gen = Generator::load(&dir, "en-US");
    let code = gen.generate();
    assert!(code.contains("static LOCALE_ID: AtomicU8"));
    assert!(code.contains("pub fn set_lang"));
    assert!(code.contains("pub fn get_locale"));
    assert!(code.contains("Lang::DeDe => 0"));

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

    let gen = Generator::load(&dir, "en-US");
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

    let gen = Generator::load(&dir, "en-US");
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

    let gen = Generator::load(&dir, "en-US");
    let code = gen.generate();
    assert!(code.contains("pub fn save__label() -> &'static str { \"Save Zed\" }"));
    assert!(code.contains("fn save__tooltip(target: &str) -> String"));
    assert!(code.contains("s.push_str(target);"));

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

    let gen = Generator::load(&dir, "en-US");
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

    let gen = Generator::load(&dir, "en-US");
    let code = gen.generate();
    assert!(code.contains("pub fn msg() -> &'static str { \"hello world\" }"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
#[should_panic(expected = "Select selector must be a variable")]
fn convert_expression_panics_on_select_with_number_selector() {
    use crate::parse::Generator;
    use std::fs;
    use std::path::PathBuf;

    let dir =
        PathBuf::from(std::env::temp_dir()).join(format!("ftl_test_selnum_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("en-US.ftl"),
        "x = { 42 ->\n    [one] val\n   *[other] val\n}\n",
    )
    .unwrap();

    Generator::load(&dir, "en-US");
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

    let gen = Generator::load(&dir, "en-US");
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

    let gen = Generator::load(&dir, "en-US");
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

    let gen = Generator::load(&dir, "en-US");
    let code = gen.generate();
    assert!(code.contains("pub fn about() -> &'static str { \"About Cool Zed\" }"));

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

    let gen = Generator::load(&dir, "en-US");
    let code = gen.generate();
    assert!(code.contains("fn welcome(user: &str) -> String"));
    assert!(code.contains("s.push_str(user);"));

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

    Generator::load(&dir, "en-US");
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

    Generator::load(&dir, "en-US");
}
