pub mod add;
pub mod bench;
pub mod doctor;
pub mod engines;
pub mod init;
pub mod list;
pub mod misc;
pub mod pkg;
pub mod run;
pub mod update;

use std::io::IsTerminal;

/// Confirm a possibly destructive/consent-requiring action.
/// `--yes` skips the prompt; non-interactive without --yes refuses safely.
pub fn confirm(msg: &str, yes: bool) -> bool {
    if yes {
        return true;
    }
    if !std::io::stdin().is_terminal() {
        println!(
            "{}: {msg} (pass --yes to accept)",
            crate::out::yellow("needs-confirmation")
        );
        return false;
    }
    use std::io::Write;
    print!("{msg} [y/N] ");
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
    let t = line.trim().to_ascii_lowercase();
    t == "y" || t == "yes"
}

/// A progress callback that prints one line per percent for big downloads.
pub fn printer() -> impl FnMut(u64, Option<u64>) {
    let mut last_pct = -1i32;
    move |done, total| {
        if let Some(t) = total {
            if t == 0 {
                return;
            }
            let pct = ((done as f64 / t as f64) * 100.0) as i32;
            if pct != last_pct && pct % 10 == 0 {
                last_pct = pct;
                eprint!("\r  {pct}%");
            }
        }
    }
}
