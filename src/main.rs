mod align;
mod diff;
mod patch;

use std::io::{Read, Write};
use std::process::{Command, Stdio};

use clap::Parser;

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

    print!("{output}");

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

    #[test]
    fn read_input_reports_missing_files() {
        let message = read_input("/definitely/not/here").unwrap_err();
        assert!(message.contains("/definitely/not/here"), "{message}");
    }
}
