use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_bodge")
}

struct Sandbox {
    directory: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Sandbox {
        let directory =
            std::env::temp_dir().join(format!("bodge-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        Sandbox { directory }
    }

    fn file(&self, name: &str, contents: &str) -> PathBuf {
        let file_path = self.directory.join(name);
        fs::write(&file_path, contents).unwrap();
        file_path
    }

    fn fake_pbcopy(&self) -> PathBuf {
        let fake_bin_directory = self.directory.join("fakebin");
        fs::create_dir_all(&fake_bin_directory).unwrap();
        let script_path = fake_bin_directory.join("pbcopy");
        let captured_path = self.directory.join("clipboard.txt");
        fs::write(
            &script_path,
            format!("#!/bin/sh\n/bin/cat > '{}'\n", captured_path.display()),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        fake_bin_directory
    }

    fn clipboard(&self) -> Option<String> {
        fs::read_to_string(self.directory.join("clipboard.txt")).ok()
    }

    fn empty_bin_directory(&self) -> PathBuf {
        let empty_directory = self.directory.join("emptybin");
        fs::create_dir_all(&empty_directory).unwrap();
        empty_directory
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn run(arguments: &[&Path], path_directory: &Path) -> Output {
    Command::new(bin())
        .args(arguments)
        .env("PATH", path_directory)
        .output()
        .unwrap()
}

fn run_str(arguments: &[&str], path_directory: &Path) -> Output {
    Command::new(bin())
        .args(arguments)
        .env("PATH", path_directory)
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

const PATCH: &str = "--- a/f.md\n+++ b/f.md\n@@ -1,2 +1,2 @@\n the same line\n-It was the best of times, it was the worst of times\n+It was the most tubular of times, it was the most bogus of times\n";

#[test]
fn compares_two_files() {
    let sandbox = Sandbox::new("two");
    let old_file = sandbox.file("a.txt", "one two three\n");
    let new_file = sandbox.file("b.txt", "one 2 three\n");
    let output = run(
        &[&old_file, &new_file, Path::new("--display-only")],
        &sandbox.empty_bin_directory(),
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "-one two three\n+one 2   three\n");
}

#[test]
fn identical_files_print_nothing() {
    let sandbox = Sandbox::new("same");
    let old_file = sandbox.file("a.txt", "same\n");
    let new_file = sandbox.file("b.txt", "same\n");
    let output = run(
        &[&old_file, &new_file, Path::new("--display-only")],
        &sandbox.empty_bin_directory(),
    );
    assert!(output.status.success());
    assert_eq!(stdout(&output), "");
}

#[test]
fn converts_a_patch_file() {
    let sandbox = Sandbox::new("patch");
    let patch_file = sandbox.file("x.patch", PATCH);
    let output = run(
        &[&patch_file, Path::new("--display-only")],
        &sandbox.empty_bin_directory(),
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(
        text.starts_with("--- a/f.md\n+++ b/f.md\n@@ -1,2 +1,2 @@\n"),
        "{text}"
    );
    assert!(!text.contains("the same line"), "{text}");
    let removed_line = text
        .lines()
        .find(|line| line.starts_with("-It was"))
        .expect("removed_line line");
    let added_line = text
        .lines()
        .find(|line| line.starts_with("+It was"))
        .expect("added_line line");
    assert_eq!(
        &added_line[1..],
        "It was the most tubular of times, it was the most bogus of times"
    );
    assert_eq!(
        removed_line
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
        "-It was the best of times, it was the worst of times"
    );
    assert_eq!(
        removed_line.find("of times,"),
        added_line.find("of times,"),
        "{text}"
    );
}

#[test]
fn reads_a_patch_from_stdin() {
    let sandbox = Sandbox::new("stdin");
    let mut process = Command::new(bin())
        .args(["-", "--display-only"])
        .env("PATH", sandbox.empty_bin_directory())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    process
        .stdin
        .take()
        .unwrap()
        .write_all(PATCH.as_bytes())
        .unwrap();
    let output = process.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("+It was the most tubular"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn copies_output_to_the_clipboard_by_default() {
    let sandbox = Sandbox::new("copy");
    let patch_file = sandbox.file("x.patch", PATCH);
    let output = run(&[&patch_file], &sandbox.fake_pbcopy());
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("copied to clipboard"),
        "{}",
        stderr(&output)
    );
    assert_eq!(
        sandbox.clipboard().as_deref(),
        Some(stdout(&output).as_str())
    );
}

#[test]
fn display_only_leaves_the_clipboard_alone() {
    let sandbox = Sandbox::new("noclip");
    let patch_file = sandbox.file("x.patch", PATCH);
    let output = run(
        &[&patch_file, Path::new("--display-only")],
        &sandbox.fake_pbcopy(),
    );
    assert!(output.status.success());
    assert!(!stdout(&output).is_empty());
    assert_eq!(sandbox.clipboard(), None);
    assert!(!stderr(&output).contains("copied"), "{}", stderr(&output));
}

#[test]
fn empty_output_does_not_overwrite_the_clipboard() {
    let sandbox = Sandbox::new("emptyclip");
    let old_file = sandbox.file("a.txt", "same\n");
    let new_file = sandbox.file("b.txt", "same\n");
    let output = run(&[&old_file, &new_file], &sandbox.fake_pbcopy());
    assert!(output.status.success());
    assert_eq!(sandbox.clipboard(), None);
}

#[test]
fn missing_pbcopy_still_prints_and_succeeds() {
    let sandbox = Sandbox::new("nopbcopy");
    let patch_file = sandbox.file("x.patch", PATCH);
    let output = run(&[&patch_file], &sandbox.empty_bin_directory());
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(!stdout(&output).is_empty());
    assert!(
        stderr(&output).contains("pbcopy unavailable"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn help_describes_the_flag_and_exits_zero() {
    let sandbox = Sandbox::new("help");
    for help_flag in ["--help", "-h"] {
        let output = run_str(&[help_flag], &sandbox.empty_bin_directory());
        assert!(output.status.success());
        let text = stdout(&output);
        assert!(text.contains("--display-only"), "{text}");
        assert!(text.contains("Usage:"), "{text}");
    }
}

#[test]
fn version_flag_prints_the_version() {
    let sandbox = Sandbox::new("version");
    let output = run_str(&["--version"], &sandbox.empty_bin_directory());
    assert!(output.status.success());
    assert!(
        stdout(&output).contains(env!("CARGO_PKG_VERSION")),
        "{}",
        stdout(&output)
    );
}

#[test]
fn no_arguments_is_a_usage_error() {
    let sandbox = Sandbox::new("noargs");
    let output = run_str(&[], &sandbox.empty_bin_directory());
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("Usage:"), "{}", stderr(&output));
}

#[test]
fn three_files_is_a_usage_error() {
    let sandbox = Sandbox::new("three");
    let output = run_str(&["a", "b", "c"], &sandbox.empty_bin_directory());
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn unknown_flag_is_a_usage_error() {
    let sandbox = Sandbox::new("unknown");
    let output = run_str(&["--nope", "a"], &sandbox.empty_bin_directory());
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("--nope"), "{}", stderr(&output));
}

#[test]
fn missing_file_reports_the_path_and_fails() {
    let sandbox = Sandbox::new("missing");
    let output = run_str(
        &["/definitely/not/here.patch", "--display-only"],
        &sandbox.empty_bin_directory(),
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("/definitely/not/here.patch"),
        "{}",
        stderr(&output)
    );
    assert_eq!(stdout(&output), "");
}

#[test]
fn utf8_text_aligns_by_characters() {
    let sandbox = Sandbox::new("utf8");
    let old_file = sandbox.file("a.txt", "a café b\n");
    let new_file = sandbox.file("b.txt", "a cafe b\n");
    let output = run(
        &[&old_file, &new_file, Path::new("--display-only")],
        &sandbox.empty_bin_directory(),
    );
    assert!(output.status.success());
    assert_eq!(stdout(&output), "-a café b\n+a cafe b\n");
}
