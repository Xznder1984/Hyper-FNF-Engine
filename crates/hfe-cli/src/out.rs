use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};

static NO_COLOR: AtomicBool = AtomicBool::new(false);

pub fn init(no_color_flag: bool) {
    let env_disabled = std::env::var_os("NO_COLOR").is_some();
    NO_COLOR.store(no_color_flag || env_disabled, Ordering::Relaxed);
}

pub fn color_on() -> bool {
    !NO_COLOR.load(Ordering::Relaxed) && std::io::stdout().is_terminal()
}

macro_rules! paint {
    ($name:ident, $color:expr) => {
        pub fn $name(text: &str) -> String {
            if color_on() {
                format!("{}{}\x1b[0m", $color, text)
            } else {
                text.to_string()
            }
        }
    };
}

paint!(green, anstyle::AnsiColor::Green.render_fg());
paint!(red, anstyle::AnsiColor::Red.render_fg());
paint!(yellow, anstyle::AnsiColor::Yellow.render_fg());
paint!(cyan, anstyle::AnsiColor::Cyan.render_fg());
paint!(dim, anstyle::Style::new().dimmed().render());
paint!(bold, anstyle::Style::new().bold().render());

pub fn status(label: &str, msg: &str) {
    println!("{} {}", cyan(&format!("{label:>10}")), msg);
}

pub fn ok(msg: &str) {
    println!("{} {}", green("ok"), msg);
}

pub fn warn(msg: &str) {
    println!("{} {}", yellow("warn"), msg);
}

pub fn fail(msg: &str) {
    eprintln!("{} {}", red("error"), msg);
}

pub fn note(msg: &str) {
    println!("{} {}", dim("note"), msg);
}