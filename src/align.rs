use crate::diff::{Pair, change_blocks, pair_lines};

fn tokens(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut token_start = 0;
    let mut previous_was_whitespace: Option<bool> = None;
    for (byte_index, character) in text.char_indices() {
        let is_whitespace = character.is_whitespace();
        if previous_was_whitespace.is_some() && previous_was_whitespace != Some(is_whitespace) {
            found.push(&text[token_start..byte_index]);
            token_start = byte_index;
        }
        previous_was_whitespace = Some(is_whitespace);
    }
    if previous_was_whitespace.is_some() {
        found.push(&text[token_start..]);
    }
    found
}

fn pad(text: &str, width: usize) -> String {
    let length = text.chars().count();
    format!("{}{}", text, " ".repeat(width.saturating_sub(length)))
}

pub fn align(old_line: &str, new_line: &str) -> (String, String) {
    let old_tokens = tokens(old_line);
    let new_tokens = tokens(new_line);
    let mut old_aligned = String::new();
    let mut new_aligned = String::new();
    let mut old_cursor = 0;
    let mut new_cursor = 0;

    for block in change_blocks(&old_tokens, &new_tokens) {
        old_aligned.push_str(&old_tokens[old_cursor..block.old_start].concat());
        new_aligned.push_str(&new_tokens[new_cursor..block.new_start].concat());

        let removed = old_tokens[block.old_start..block.old_end].concat();
        let added = new_tokens[block.new_start..block.new_end].concat();
        let width = removed.chars().count().max(added.chars().count());

        old_aligned.push_str(&pad(&removed, width));
        new_aligned.push_str(&pad(&added, width));
        old_cursor = block.old_end;
        new_cursor = block.new_end;
    }

    old_aligned.push_str(&old_tokens[old_cursor..].concat());
    new_aligned.push_str(&new_tokens[new_cursor..].concat());

    (
        old_aligned.trim_end().to_string(),
        new_aligned.trim_end().to_string(),
    )
}

pub fn emit(old_lines: &[&str], new_lines: &[&str], output: &mut String) {
    for operation in pair_lines(old_lines, new_lines) {
        match operation {
            Pair::Both(old_index, new_index) => {
                let (old_aligned, new_aligned) = align(old_lines[old_index], new_lines[new_index]);
                output.push_str(&format!("-{old_aligned}\n+{new_aligned}\n"));
            }
            Pair::Del(old_index) => output.push_str(&format!("-{}\n", old_lines[old_index])),
            Pair::Add(new_index) => output.push_str(&format!("+{}\n", new_lines[new_index])),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_alternate_words_and_whitespace() {
        assert_eq!(tokens("a  b\tc"), vec!["a", "  ", "b", "\t", "c"]);
    }

    #[test]
    fn tokens_keep_leading_and_trailing_whitespace() {
        assert_eq!(tokens("  a b "), vec!["  ", "a", " ", "b", " "]);
    }

    #[test]
    fn tokens_of_empty_string_is_empty() {
        assert!(tokens("").is_empty());
    }

    #[test]
    fn tokens_treat_punctuation_as_part_of_a_word() {
        assert_eq!(tokens("hi, there."), vec!["hi,", " ", "there."]);
    }

    #[test]
    fn identical_lines_are_unchanged() {
        let (old_aligned, new_aligned) = align("alpha beta gamma", "alpha beta gamma");
        assert_eq!(old_aligned, "alpha beta gamma");
        assert_eq!(new_aligned, "alpha beta gamma");
    }

    #[test]
    fn replaced_word_keeps_other_words_in_place() {
        let (old_aligned, new_aligned) = align("alpha beta gamma delta", "alpha beta GAMMA delta");
        assert_eq!(old_aligned, "alpha beta gamma delta");
        assert_eq!(new_aligned, "alpha beta GAMMA delta");
    }

    #[test]
    fn replaced_word_of_different_length_is_padded() {
        let (old_aligned, new_aligned) = align("one two three", "one 2 three");
        assert_eq!(old_aligned, "one two three");
        assert_eq!(new_aligned, "one 2   three");
    }

    #[test]
    fn deleted_word_pads_the_new_side() {
        let (old_aligned, new_aligned) = align("one two three", "one three");
        assert_eq!(old_aligned, "one two three");
        assert_eq!(new_aligned, "one     three");
    }

    #[test]
    fn inserted_word_pads_the_old_side() {
        let (old_aligned, new_aligned) = align("one three", "one two three");
        assert_eq!(old_aligned, "one     three");
        assert_eq!(new_aligned, "one two three");
    }

    #[test]
    fn trailing_padding_is_trimmed() {
        let (old_aligned, new_aligned) = align("one two", "one");
        assert_eq!(old_aligned, "one two");
        assert_eq!(new_aligned, "one");
    }

    #[test]
    fn padding_counts_characters_not_bytes() {
        let (old_aligned, new_aligned) = align("a café b", "a cafe b");
        assert_eq!(old_aligned, "a café b");
        assert_eq!(new_aligned, "a cafe b");
    }

    #[test]
    fn columns_line_up_after_a_change() {
        let (old_aligned, new_aligned) = align("start old end", "start brand new end");
        assert_eq!(old_aligned.find("end"), new_aligned.find("end"));
    }

    #[test]
    fn emit_writes_a_pair_as_minus_then_plus() {
        let mut output = String::new();
        emit(
            &["the quick brown fox"],
            &["the quick red fox"],
            &mut output,
        );
        assert_eq!(output, "-the quick brown fox\n+the quick red   fox\n");
    }

    #[test]
    fn emit_writes_lone_deletion_and_addition() {
        let mut output = String::new();
        emit(&["aaaaaaaa"], &["zzzzzzzz"], &mut output);
        assert_eq!(output, "-aaaaaaaa\n+zzzzzzzz\n");
    }

    #[test]
    fn emit_with_nothing_writes_nothing() {
        let mut output = String::new();
        emit(&[], &[], &mut output);
        assert_eq!(output, "");
    }
}
