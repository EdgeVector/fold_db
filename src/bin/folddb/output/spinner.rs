use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub fn new_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&[
                "\u{28fe}", "\u{28fd}", "\u{28fb}", "\u{28f7}", "\u{28ef}", "\u{28df}",
                "\u{28bf}", "\u{287f}", "\u{28fe}",
            ]),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

pub fn finish_spinner(pb: &ProgressBar, message: &str) {
    pb.set_style(ProgressStyle::with_template("{msg}").unwrap());
    pb.finish_with_message(format!(
        "{} {}",
        console::style("\u{2713}").green().bold(),
        message
    ));
}

pub fn new_progress_bar(total: u64, prefix: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template("{prefix} [{bar:30.cyan/dim}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("\u{2588}\u{2592}\u{2591}"),
    );
    pb.set_prefix(prefix.to_string());
    pb
}
