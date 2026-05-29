pub mod i18n {
    include!(concat!(env!("OUT_DIR"), "/i18n_gen.rs"));
}

use crate::i18n::*;

fn print_locale() {
    println!("  settings = {}", t!(settings()));
    println!("  hello    = {}", t!(hello("World")));
    println!("  item-count = {}", t!(item_count(42)));
    println!("  files(1)   = {}", t!(files(1)));
    println!("  files(99)  = {}", t!(files(99)));
    println!("  about-app  = {}", t!(about_app()));
    println!("  welcome-user = {}", t!(welcome_user("Jane")));
    println!("  welcome-term = {}", t!(welcome_term()));
    println!("  about-brand = {}", t!(about_brand("genitive")));
    println!("  pos-args-demo = {}", t!(pos_args_demo()));
    println!(
        "  update-brand(nominative) = {}",
        t!(update_brand("nominative"))
    );
    println!("  login-input.title = {}", t!(login_input__title()));
    println!("  save.label = {}", t!(save__label()));
    println!("  save.tooltip = {}", t!(save__tooltip("file")));
    println!("  attr-ref-demo = {}", t!(attr_ref_demo()));
    println!("  attr-select-demo = {}", t!(attr_select_demo()));
    println!("  greet(male)   = {}", t!(user_greeting("male")));
    println!("  greet(female) = {}", t!(user_greeting("female")));
    println!("  greet(other)  = {}", t!(user_greeting("other")));
    println!("  finish(1) = {}", t!(finish_place(1)));
    println!("  finish(5) = {}", t!(finish_place(5)));
    println!("  test-add-10(7) = {}", t!(test_add_10(Test::new(7))));
    println!("  test-sub-5(7) = {}", t!(test_sub_5(Test::new(7))));
    println!("  test-mul-3(7) = {}", t!(test_mul_3(Test::new(7))));
    println!("  test-div-2(7) = {}", t!(test_div_2(Test::new(7))));
    println!(
        "  dpi-ratio = {}",
        t!(dpi_ratio(Number::new(96.0).minimum_fraction_digits(2)))
    );
    println!(
        "  today-is = {}",
        t!(today_is(DateTime::new(0).year(2024).month(5).day(17)))
    );
    println!(
        "  full-date = {}",
        t!(full_date(
            DateTime::new(0)
                .year(2024)
                .month(5)
                .day(17)
                .month_format("long".to_string())
                .year_format("numeric".to_string())
                .day_format("numeric".to_string())
        ))
    );
}

fn main() {
    set_lang(Lang::EnUs);
    println!("=== en-US ===");
    print_locale();

    set_lang(Lang::ZhCn);
    println!("=== zh-CN ===");
    print_locale();

    set_lang(Lang::JaJp);
    println!("=== ja-JP ===");
    print_locale();
}
