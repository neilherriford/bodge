use crate::align::emit;

const HEADER_PREFIXES: &[&str] = &[
    "diff ",
    "index ",
    "--- ",
    "+++ ",
    "new file",
    "deleted file",
    "similarity",
    "rename ",
    "old mode",
    "new mode",
    "Binary files",
];

fn parse_hunk(line: &str) -> Option<(usize, usize)> {
    let after_marker = line.strip_prefix("@@ -")?;
    let (old_range, after_old) = after_marker.split_once(" +")?;
    let (new_range, _) = after_old.split_once(" @@")?;

    let parse_line_count = |range: &str| -> Option<usize> {
        let (start_text, count_text) = match range.split_once(',') {
            Some((start, count)) => (start, Some(count)),
            None => (range, None),
        };
        start_text.parse::<usize>().ok()?;
        match count_text {
            Some(count) => count.parse().ok(),
            None => Some(1),
        }
    };

    Some((parse_line_count(old_range)?, parse_line_count(new_range)?))
}

fn flush<'text>(
    removed_lines: &mut Vec<&'text str>,
    added_lines: &mut Vec<&'text str>,
    output: &mut String,
) {
    if !removed_lines.is_empty() || !added_lines.is_empty() {
        emit(removed_lines, added_lines, output);
    }
    removed_lines.clear();
    added_lines.clear();
}

pub fn convert(patch_text: &str) -> String {
    let mut result = String::new();
    let mut removed_lines: Vec<&str> = Vec::new();
    let mut added_lines: Vec<&str> = Vec::new();
    let mut old_remaining = 0usize;
    let mut new_remaining = 0usize;

    for line in patch_text.lines() {
        if old_remaining > 0 || new_remaining > 0 {
            if line.starts_with('\\') {
                continue;
            }
            if let Some(content) = line.strip_prefix('-') {
                removed_lines.push(content);
                old_remaining = old_remaining.saturating_sub(1);
            } else if let Some(content) = line.strip_prefix('+') {
                added_lines.push(content);
                new_remaining = new_remaining.saturating_sub(1);
            } else {
                flush(&mut removed_lines, &mut added_lines, &mut result);
                old_remaining = old_remaining.saturating_sub(1);
                new_remaining = new_remaining.saturating_sub(1);
            }
            if old_remaining == 0 && new_remaining == 0 {
                flush(&mut removed_lines, &mut added_lines, &mut result);
            }
            continue;
        }

        if let Some((old_count, new_count)) = parse_hunk(line) {
            flush(&mut removed_lines, &mut added_lines, &mut result);
            result.push_str(line);
            result.push('\n');
            old_remaining = old_count;
            new_remaining = new_count;
        } else if HEADER_PREFIXES
            .iter()
            .any(|prefix| line.starts_with(prefix))
        {
            flush(&mut removed_lines, &mut added_lines, &mut result);
            result.push_str(line);
            result.push('\n');
        }
    }

    flush(&mut removed_lines, &mut added_lines, &mut result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hunk_with_both_counts() {
        assert_eq!(parse_hunk("@@ -1,3 +1,4 @@"), Some((3, 4)));
    }

    #[test]
    fn hunk_without_counts_means_one() {
        assert_eq!(parse_hunk("@@ -5 +7 @@"), Some((1, 1)));
    }

    #[test]
    fn hunk_with_zero_counts() {
        assert_eq!(parse_hunk("@@ -0,0 +1,2 @@"), Some((0, 2)));
        assert_eq!(parse_hunk("@@ -3,2 +2,0 @@"), Some((2, 0)));
    }

    #[test]
    fn hunk_ignores_trailing_section_heading() {
        assert_eq!(parse_hunk("@@ -10,2 +10,3 @@ fn main() {"), Some((2, 3)));
    }

    #[test]
    fn non_hunk_lines_do_not_parse() {
        assert_eq!(parse_hunk("not a hunk"), None);
        assert_eq!(parse_hunk("@@ -a,b +c,d @@"), None);
        assert_eq!(parse_hunk("@@ -1,2 +3,4"), None);
        assert_eq!(parse_hunk("@@ -1,x +1,2 @@"), None);
        assert_eq!(parse_hunk(""), None);
    }

    #[test]
    fn empty_input_converts_to_nothing() {
        assert_eq!(convert(""), "");
    }

    #[test]
    fn text_that_is_not_a_patch_is_ignored() {
        assert_eq!(convert("hello\nworld\n"), "");
    }

    #[test]
    fn file_and_hunk_headers_are_kept() {
        let patch_text = "--- a/f\n+++ b/f\n@@ -1 +1 @@\n-old words here\n+new words here\n";
        let converted = convert(patch_text);
        assert!(
            converted.starts_with("--- a/f\n+++ b/f\n@@ -1 +1 @@\n"),
            "{converted}"
        );
        assert!(
            converted.ends_with("-old words here\n+new words here\n"),
            "{converted}"
        );
    }

    #[test]
    fn git_extended_headers_are_kept() {
        let patch_text = "diff --git a/f b/f\nindex 123..456 100644\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-a b c\n+a b d\n";
        let converted = convert(patch_text);
        assert!(
            converted.starts_with("diff --git a/f b/f\nindex 123..456 100644\n--- a/f\n"),
            "{converted}"
        );
    }

    #[test]
    fn context_lines_are_dropped() {
        let patch_text =
            "--- a/f\n+++ b/f\n@@ -1,3 +1,3 @@\n same\n-old words here\n+new words here\n same\n";
        let converted = convert(patch_text);
        assert!(!converted.contains("same"), "{converted}");
        assert!(
            converted.contains("-old words here\n+new words here\n"),
            "{converted}"
        );
    }

    #[test]
    fn empty_context_line_is_handled() {
        let patch_text =
            "--- a/f\n+++ b/f\n@@ -1,3 +1,3 @@\n-old words here\n+new words here\n\n same\n";
        let converted = convert(patch_text);
        assert!(
            converted.contains("-old words here\n+new words here\n"),
            "{converted}"
        );
    }

    #[test]
    fn content_lines_starting_with_dashes_are_not_headers() {
        let patch_text = "--- a/f\n+++ b/f\n@@ -1,2 +1,2 @@\n--- an hr rule\n-keep going now\n+--- an hr rule!\n+keep going NOW\n";
        let converted = convert(patch_text);
        assert!(
            converted.starts_with("--- a/f\n+++ b/f\n@@ -1,2 +1,2 @@\n"),
            "{converted}"
        );
        assert!(
            converted.contains("-keep going now\n+keep going NOW\n"),
            "{converted}"
        );
    }

    #[test]
    fn no_newline_marker_is_skipped() {
        let patch_text = "--- a/f\n+++ b/f\n@@ -1 +1 @@\n-old words here\n\\ No newline at end of file\n+new words here\n\\ No newline at end of file\n";
        let converted = convert(patch_text);
        assert!(!converted.contains("No newline"), "{converted}");
        assert!(
            converted.contains("-old words here\n+new words here\n"),
            "{converted}"
        );
    }

    #[test]
    fn deletion_only_hunk() {
        let patch_text = "--- a/f\n+++ b/f\n@@ -1,2 +0,0 @@\n-gone one\n-gone two\n";
        let converted = convert(patch_text);
        assert!(
            converted.ends_with("@@ -1,2 +0,0 @@\n-gone one\n-gone two\n"),
            "{converted}"
        );
    }

    #[test]
    fn addition_only_hunk() {
        let patch_text = "--- a/f\n+++ b/f\n@@ -0,0 +1,2 @@\n+new one\n+new two\n";
        let converted = convert(patch_text);
        assert!(converted.ends_with("+new one\n+new two\n"), "{converted}");
    }

    #[test]
    fn multiple_hunks_and_files() {
        let patch_text = "--- a/one\n+++ b/one\n@@ -1 +1 @@\n-a b c d\n+a b c e\n@@ -9 +9 @@\n-x y z w\n+x y z v\n--- a/two\n+++ b/two\n@@ -1 +1 @@\n-p q r s\n+p q r t\n";
        let converted = convert(patch_text);
        assert_eq!(
            converted
                .lines()
                .filter(|line| line.starts_with("@@ "))
                .count(),
            3,
            "{converted}"
        );
        assert!(converted.contains("--- a/two\n+++ b/two\n"), "{converted}");
        assert!(converted.contains("-x y z w\n+x y z v\n"), "{converted}");
    }

    #[test]
    fn changes_separated_by_context_are_paired_separately() {
        let patch_text = "--- a/f\n+++ b/f\n@@ -1,3 +1,3 @@\n-first old line here\n+first new line here\n middle\n-second old line here\n+second new line here\n";
        let converted = convert(patch_text);
        assert!(
            converted.contains("-first old line here\n+first new line here\n"),
            "{converted}"
        );
        assert!(
            converted.contains("-second old line here\n+second new line here\n"),
            "{converted}"
        );
    }
}
