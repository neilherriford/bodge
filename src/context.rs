pub const MINIMUM_CONTEXT_CHARACTERS: u64 = 5;
pub const DEFAULT_CONTEXT_CHARACTERS: u64 = 15;
const ELLIPSIS: char = '…';

fn is_removal(line: &str) -> bool {
    line.starts_with('-') && !line.starts_with("--- ")
}

fn is_addition(line: &str) -> bool {
    line.starts_with('+') && !line.starts_with("+++ ")
}

fn common_prefix_length(first: &[char], second: &[char]) -> usize {
    first
        .iter()
        .zip(second)
        .take_while(|(first_character, second_character)| first_character == second_character)
        .count()
}

fn common_suffix_length(first: &[char], second: &[char], prefix_length: usize) -> usize {
    let available = first.len().min(second.len()) - prefix_length;

    first
        .iter()
        .rev()
        .zip(second.iter().rev())
        .take(available)
        .take_while(|(first_character, second_character)| first_character == second_character)
        .count()
}

fn shorten(
    characters: &[char],
    prefix_length: usize,
    suffix_length: usize,
    context_characters: usize,
) -> String {
    let start = prefix_length.saturating_sub(context_characters);
    let trimmed_suffix = suffix_length.saturating_sub(context_characters);
    let end = characters.len() - trimmed_suffix;
    let mut result = String::new();

    if start > 0 {
        result.push(ELLIPSIS);
    }

    result.extend(&characters[start..end]);
    if trimmed_suffix > 0 {
        result.push(ELLIPSIS);
    }

    result
}

fn trim_pair(removed: &str, added: &str, context_characters: usize) -> (String, String) {
    let removed_characters: Vec<char> = removed[1..].chars().collect();
    let added_characters: Vec<char> = added[1..].chars().collect();
    let prefix_length = common_prefix_length(&removed_characters, &added_characters);
    let suffix_length = common_suffix_length(&removed_characters, &added_characters, prefix_length);

    (
        format!(
            "-{}",
            shorten(
                &removed_characters,
                prefix_length,
                suffix_length,
                context_characters
            )
        ),
        format!(
            "+{}",
            shorten(
                &added_characters,
                prefix_length,
                suffix_length,
                context_characters
            )
        ),
    )
}

pub fn trim_context(output: &str, context_characters: u64) -> String {
    let lines: Vec<&str> = output.lines().collect();
    let mut result = String::new();
    let mut index = 0;

    while index < lines.len() {
        let is_pair =
            index + 1 < lines.len() && is_removal(lines[index]) && is_addition(lines[index + 1]);
        if is_pair {
            let (removed, added) =
                trim_pair(lines[index], lines[index + 1], context_characters as usize);
            result.push_str(&removed);
            result.push('\n');
            result.push_str(&added);
            result.push('\n');
            index += 2;
        } else {
            result.push_str(lines[index]);
            result.push('\n');
            index += 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn focus(output: &str) -> String {
        trim_context(output, DEFAULT_CONTEXT_CHARACTERS)
    }

    #[test]
    fn honors_a_custom_context_size() {
        let result = trim_context(
            "-aaaaaaaaaaaaaaaaaaaaXbbbbbbbbbbbbbbbbbbbb\n+aaaaaaaaaaaaaaaaaaaaYbbbbbbbbbbbbbbbbbbbb\n",
            5,
        );
        assert_eq!(result, "-…aaaaaXbbbbb…\n+…aaaaaYbbbbb…\n");
    }

    #[test]
    fn a_larger_context_keeps_more_text() {
        let result = trim_context(
            "-aaaaaaaaaaaaaaaaaaaaXbbbbbbbbbbbbbbbbbbbb\n+aaaaaaaaaaaaaaaaaaaaYbbbbbbbbbbbbbbbbbbbb\n",
            18,
        );
        assert_eq!(
            result,
            "-…aaaaaaaaaaaaaaaaaaXbbbbbbbbbbbbbbbbbb…\n+…aaaaaaaaaaaaaaaaaaYbbbbbbbbbbbbbbbbbb…\n"
        );
    }

    #[test]
    fn omits_long_shared_context_on_both_sides() {
        let result = focus(
            "-aaaaaaaaaaaaaaaaaaaaXbbbbbbbbbbbbbbbbbbbb\n+aaaaaaaaaaaaaaaaaaaaYbbbbbbbbbbbbbbbbbbbb\n",
        );
        assert_eq!(
            result,
            "-…aaaaaaaaaaaaaaaXbbbbbbbbbbbbbbb…\n+…aaaaaaaaaaaaaaaYbbbbbbbbbbbbbbb…\n"
        );
    }

    #[test]
    fn keeps_short_context_untouched() {
        let result = focus("-one two three\n+one 2   three\n");
        assert_eq!(result, "-one two three\n+one 2   three\n");
    }

    #[test]
    fn keeps_exactly_fifteen_characters_without_ellipsis() {
        let result = focus("-aaaaaaaaaaaaaaaXbbbbbbbbbbbbbbb\n+aaaaaaaaaaaaaaaYbbbbbbbbbbbbbbb\n");
        assert_eq!(
            result,
            "-aaaaaaaaaaaaaaaXbbbbbbbbbbbbbbb\n+aaaaaaaaaaaaaaaYbbbbbbbbbbbbbbb\n"
        );
    }

    #[test]
    fn omits_only_the_side_that_is_long() {
        let result = focus("-aaaaaaaaaaaaaaaaaaaaXb\n+aaaaaaaaaaaaaaaaaaaaYb\n");
        assert_eq!(result, "-…aaaaaaaaaaaaaaaXb\n+…aaaaaaaaaaaaaaaYb\n");
        let result = focus("-aXbbbbbbbbbbbbbbbbbbbb\n+aYbbbbbbbbbbbbbbbbbbbb\n");
        assert_eq!(result, "-aXbbbbbbbbbbbbbbb…\n+aYbbbbbbbbbbbbbbb…\n");
    }

    #[test]
    fn handles_insertion_where_suffix_could_overlap_prefix() {
        let result = focus("-aaaaaaaaaaaaaaaaaaaa\n+aaaaaaaaaaaaaaaaaaaaa\n");
        assert_eq!(result, "-…aaaaaaaaaaaaaaa\n+…aaaaaaaaaaaaaaaa\n");
    }

    #[test]
    fn leaves_headers_hunks_and_unpaired_lines_alone() {
        let input = "--- a/f\n+++ b/f\n@@ -1 +1 @@\n-only removed\n-aaaaaaaaaaaaaaaaaaaaXbbbbbbbbbbbbbbbbbbbb\n+aaaaaaaaaaaaaaaaaaaaYbbbbbbbbbbbbbbbbbbbb\n+only added\n";
        let result = focus(input);
        assert_eq!(
            result,
            "--- a/f\n+++ b/f\n@@ -1 +1 @@\n-only removed\n-…aaaaaaaaaaaaaaaXbbbbbbbbbbbbbbb…\n+…aaaaaaaaaaaaaaaYbbbbbbbbbbbbbbb…\n+only added\n"
        );
    }

    #[test]
    fn focuses_the_aligned_keywords_example() {
        let before = "keywords: Engineering Management, Cross-functional Project Management, E-commerce, Health Tech, HIPAA, Rust, Ruby, Ruby on Rails, TypeScript, JavaScript, Node.js, React, GraphQL, Python, C#, F#, Java, PHP, PostgreSQL, MySQL, SQL Server, AWS, Terraform, Datadog, ";
        let after = " SFTP, Claude Code, MCP, GCP, Docker, Linux, Git, REST, Bash, Shopify, NetSuite, SuiteScript, Salesforce, Stripe";
        let input = format!("-{before}bservability, {after}\n+{before}Observability,{after}\n");
        let result = focus(&input);
        assert_eq!(
            result,
            "-…form, Datadog, bservability,  SFTP, Claude C…\n+…form, Datadog, Observability, SFTP, Claude C…\n"
        );
    }

    #[test]
    fn handles_multibyte_characters() {
        let fifteen = "é".repeat(15);
        let twenty = "é".repeat(20);
        let input = format!("-{twenty}X{twenty}\n+{twenty}Y{twenty}\n");
        let result = focus(&input);
        assert_eq!(
            result,
            format!("-…{fifteen}X{fifteen}…\n+…{fifteen}Y{fifteen}…\n")
        );
    }

    #[test]
    fn preserves_empty_output() {
        assert_eq!(focus(""), "");
    }
}
