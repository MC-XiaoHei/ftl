include!(concat!(env!("OUT_DIR"), "/i18n_gen.rs"));

fn main() {
    println!("[en-US] settings = {}", t!(settings()));
    println!("[en-US] hello = {}", t!(hello("World")));
    println!("[en-US] files(1) = {}", t!(files(1)));
    println!("[en-US] files(5) = {}", t!(files(5)));
    println!();

    set_lang(Lang::ZhCn);
    println!("[zh-CN] settings = {}", t!(settings()));
    println!("[zh-CN] hello = {}", t!(hello("世界")));
    println!("[zh-CN] files(1) = {}", t!(files(1)));
    println!("[zh-CN] files(5) = {}", t!(files(5)));
    println!();

    set_lang(Lang::JaJp);
    println!("[ja-JP] settings = {}", t!(settings()));
    println!("[ja-JP] hello = {}", t!(hello("World")));
    println!("[ja-JP] files(1) = {}", t!(files(1)));
    println!("[ja-JP] files(5) = {}", t!(files(5)));
}
