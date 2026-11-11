mod align;
mod diff;
mod patch;

use clap::Parser;
use colored::{Color, Colorize};
use std::io::{IsTerminal, Read, Write};
use std::process::{Command, Stdio};

#[derive(Parser)]
#[command(
    name = "bodge",
    version,
    about = "Convert a patch for plain text prose to something more human readable.",
    long_about = "Convert a patch for plain text prose to something more human readable.
Work alongside a robot editor to improve plaintext prose.  Have it
communicate suggestions via git patch, then use this tool to make it
a little more visible and clear what changes are being recommended.
Groups like lines, and word changes spaced out so that the differences
are clear.",
    override_usage = "bodge [OPTIONS] <PATCH|->\n       bodge [OPTIONS] <OLD> <NEW>"
)]
struct Cli {
    #[arg(long, help = "Print only; do not copy the output to the clipboard")]
    display_only: bool,

    #[arg(long, help = "Do not colorize the output")]
    plain: bool,

    #[arg(
        value_name = "FILE",
        num_args = 1..=2,
        required = true,
        help = "A patch (or - for stdin), or an old and a new file to compare"
    )]
    files: Vec<String>,
}

fn read_input(path: &str) -> Result<String, String> {
    if path == "-" {
        let mut contents = String::new();
        std::io::stdin()
            .read_to_string(&mut contents)
            .map_err(|error| format!("stdin: {error}"))?;
        Ok(contents)
    } else {
        std::fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))
    }
}

fn compare(old_text: &str, new_text: &str) -> String {
    let old_lines: Vec<&str> = old_text.lines().collect();
    let new_lines: Vec<&str> = new_text.lines().collect();
    let mut result = String::new();

    for block in diff::change_blocks(&old_lines, &new_lines) {
        align::emit(
            &old_lines[block.old_start..block.old_end],
            &new_lines[block.new_start..block.new_end],
            &mut result,
        );
    }
    result
}

const TAB_MARKER: &str = "\u{2192}   ";

fn colorize_body(text: &str, color: Color) -> String {
    let trimmed = text.trim_end();
    let mut result = String::new();
    let mut pending_text = String::new();

    let flush = |pending_text: &mut String, result: &mut String| {
        if !pending_text.is_empty() {
            result.push_str(&pending_text.as_str().color(color).to_string());
            pending_text.clear();
        }
    };

    for character in trimmed.chars() {
        if character == '\t' {
            flush(&mut pending_text, &mut result);
            result.push_str(&TAB_MARKER.on_red().to_string());
        } else {
            pending_text.push(character);
        }
    }
    flush(&mut pending_text, &mut result);

    let trailing_whitespace = &text[trimmed.len()..];
    if !trailing_whitespace.is_empty() {
        result.push_str(
            &trailing_whitespace
                .replace('\t', TAB_MARKER)
                .on_red()
                .to_string(),
        );
    }
    result
}

fn colorize(output: &str) -> String {
    let mut result = String::new();
    for line in output.split_inclusive('\n') {
        let (text, newline) = match line.strip_suffix('\n') {
            Some(text) => (text, "\n"),
            None => (line, ""),
        };
        let colored_line = if text.starts_with("+++") || text.starts_with("---") {
            text.bold().to_string()
        } else if text.starts_with("@@") {
            text.cyan().to_string()
        } else if let Some(removed_text) = text.strip_prefix('-') {
            format!("{}{}", "-".red(), colorize_body(removed_text, Color::Red))
        } else if let Some(added_text) = text.strip_prefix('+') {
            format!("{}{}", "+".green(), colorize_body(added_text, Color::Green))
        } else {
            text.to_string()
        };
        result.push_str(&colored_line);
        result.push_str(newline);
    }
    result
}

fn wants_color(plain: bool, is_terminal: bool, color_forced: bool) -> bool {
    !plain && (is_terminal || color_forced)
}

fn copy_to_clipboard(text: &str) -> std::io::Result<()> {
    let mut child = Command::new("pbcopy").stdin(Stdio::piped()).spawn()?;
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(text.as_bytes())?;

    if child.wait()?.success() {
        Ok(())
    } else {
        Err(std::io::Error::other("pbcopy failed"))
    }
}

fn run(cli: Cli) -> Result<(), String> {
    let output = match cli.files.as_slice() {
        [patch_file] => patch::convert(&read_input(patch_file)?),
        [old_file, new_file] => compare(&read_input(old_file)?, &read_input(new_file)?),
        _ => unreachable!("clap enforces one or two files"),
    };

    let color_forced = std::env::var_os("CLICOLOR_FORCE").is_some_and(|value| value != "0");
    let wants_color = wants_color(cli.plain, std::io::stdout().is_terminal(), color_forced);
    if wants_color && colored::control::SHOULD_COLORIZE.should_colorize() {
        print!("{}", colorize(&output));
    } else {
        print!("{output}");
    }

    if !cli.display_only && !output.is_empty() {
        match copy_to_clipboard(&output) {
            Ok(()) => eprintln!("(copied to clipboard)"),
            Err(_) => eprintln!("(pbcopy unavailable; not copied)"),
        }
    }

    Ok(())
}

fn main() {
    if let Err(message) = run(Cli::parse()) {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_one_file() {
        let cli = Cli::try_parse_from(["bodge", "a.patch"]).unwrap();
        assert_eq!(cli.files, vec!["a.patch"]);
        assert!(!cli.display_only);
    }

    #[test]
    fn plain_flag_defaults_to_off_and_can_go_anywhere() {
        assert!(!Cli::try_parse_from(["bodge", "a"]).unwrap().plain);
        assert!(
            Cli::try_parse_from(["bodge", "--plain", "a"])
                .unwrap()
                .plain
        );
        assert!(
            Cli::try_parse_from(["bodge", "a", "--plain"])
                .unwrap()
                .plain
        );
    }

    #[test]
    fn wants_color_on_terminal_or_when_forced() {
        assert!(wants_color(false, true, false));
        assert!(wants_color(false, false, true));
        assert!(!wants_color(false, false, false));
    }

    #[test]
    fn plain_overrides_terminal_and_forced_color() {
        assert!(!wants_color(true, true, false));
        assert!(!wants_color(true, true, true));
        assert!(!wants_color(true, false, true));
    }

    #[test]
    fn parses_two_files() {
        let cli = Cli::try_parse_from(["bodge", "a", "b"]).unwrap();
        assert_eq!(cli.files, vec!["a", "b"]);
    }

    #[test]
    fn parses_dash_as_stdin_file() {
        let cli = Cli::try_parse_from(["bodge", "-"]).unwrap();
        assert_eq!(cli.files, vec!["-"]);
    }

    #[test]
    fn display_only_flag_can_go_anywhere() {
        let flag_first = Cli::try_parse_from(["bodge", "--display-only", "a"]).unwrap();
        let flag_last = Cli::try_parse_from(["bodge", "a", "--display-only"]).unwrap();
        assert!(flag_first.display_only && flag_last.display_only);
    }

    #[test]
    fn rejects_no_files() {
        assert!(Cli::try_parse_from(["bodge"]).is_err());
    }

    #[test]
    fn rejects_three_files() {
        assert!(Cli::try_parse_from(["bodge", "a", "b", "c"]).is_err());
    }

    #[test]
    fn rejects_unknown_flag() {
        assert!(Cli::try_parse_from(["bodge", "--nope", "a"]).is_err());
    }

    #[test]
    fn compare_identical_texts_is_empty() {
        assert_eq!(compare("a b c\n", "a b c\n"), "");
    }

    #[test]
    fn compare_shows_changed_line_pair() {
        assert_eq!(compare("a b c\n", "a X c\n"), "-a b c\n+a X c\n");
    }

    #[test]
    fn compare_shows_added_and_removed_lines() {
        assert_eq!(compare("keep\n", "keep\nnew line\n"), "+new line\n");
        assert_eq!(compare("keep\nold line\n", "keep\n"), "-old line\n");
    }

    #[test]
    fn compare_pairs_edits_around_an_insertion() {
        let old_text = "alpha beta gamma delta epsilon\nzeta eta theta iota kappa\n";
        let new_text = "a completely new unrelated line\nalpha beta GAMMA delta epsilon\nzeta eta theta iota KAPPA\n";
        let output = compare(old_text, new_text);
        assert!(output.contains("+a completely new unrelated line\n-alpha beta gamma delta epsilon\n+alpha beta GAMMA delta epsilon\n"), "{output}");
    }

    static COLOR_OVERRIDE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn with_color_override<T>(enabled: bool, action: impl FnOnce() -> T) -> T {
        let _guard = COLOR_OVERRIDE_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        colored::control::set_override(enabled);
        let result = action();
        colored::control::unset_override();
        result
    }

    fn with_colors_forced<T>(action: impl FnOnce() -> T) -> T {
        with_color_override(true, action)
    }

    #[test]
    fn colorize_colors_removed_and_added_lines() {
        let result = with_colors_forced(|| colorize("-old\n+new\n"));
        assert!(result.contains("\x1b[31mold\x1b[0m"), "{result:?}");
        assert!(result.contains("\x1b[32mnew\x1b[0m"), "{result:?}");
        assert!(result.ends_with('\n'));
    }

    #[test]
    fn colorize_colors_hunk_and_file_headers() {
        let result = with_colors_forced(|| colorize("--- a/f\n+++ b/f\n@@ -1 +1 @@\n"));
        assert!(result.contains("\x1b[1m--- a/f\x1b[0m"), "{result:?}");
        assert!(result.contains("\x1b[1m+++ b/f\x1b[0m"), "{result:?}");
        assert!(result.contains("\x1b[36m@@ -1 +1 @@\x1b[0m"), "{result:?}");
    }

    #[test]
    fn colorize_highlights_trailing_whitespace() {
        let result = with_colors_forced(|| colorize("+word  \n"));
        assert!(result.contains("\x1b[41m  \x1b[0m"), "{result:?}");
    }

    #[test]
    fn colorize_highlights_tabs() {
        let result = with_colors_forced(|| colorize("-a\tb\n"));
        assert!(
            result.contains(&format!("\x1b[41m{TAB_MARKER}\x1b[0m")),
            "{result:?}"
        );
    }

    #[test]
    fn colorize_leaves_interior_spaces_unhighlighted() {
        let result = with_colors_forced(|| colorize("+a    b\n"));
        assert!(!result.contains("\x1b[41m"), "{result:?}");
    }

    #[test]
    fn colorize_without_colors_returns_plain_text() {
        let result = with_color_override(false, || colorize("-old\n+new\n"));
        assert_eq!(result, "-old\n+new\n");
    }

    #[test]
    fn read_input_reports_missing_files() {
        let message = read_input("/definitely/not/here").unwrap_err();
        assert!(message.contains("/definitely/not/here"), "{message}");
    }
}
