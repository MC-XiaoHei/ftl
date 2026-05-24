include!(concat!(env!("OUT_DIR"), "/i18n_gen.rs"));

fn print_locale() {
    println!("  settings = {}", t!(settings()));
    println!("  hello    = {}", t!(hello("World")));
    println!("  item-count = {}", t!(item_count("42")));
    println!("  files(1)   = {}", t!(files(1)));
    println!("  files(99)  = {}", t!(files(99)));
    println!("  about-app  = {}", t!(about_app()));
    println!("  welcome-user = {}", t!(welcome_user("Jane")));
    println!("  welcome-term = {}", t!(welcome_term()));
    println!("  about-brand = {}", t!(about_brand("genitive")));
    println!(
        "  update-brand (nominative) = {}",
        t!(update_brand("nominative"))
    );
    println!("  login-input.title = {}", t!(login_input__title()));
    println!("  save.label = {}", t!(save__label()));
    println!("  save.tooltip = {}", t!(save__tooltip("file")));
    println!("  greet(male)   = {}", t!(user_greeting("male")));
    println!("  greet(female) = {}", t!(user_greeting("female")));
    println!("  greet(other)  = {}", t!(user_greeting("other")));
    println!("  finish(1) = {}", t!(finish_place(1)));
    println!("  finish(5) = {}", t!(finish_place(5)));
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
