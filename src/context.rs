pub const MINIMUM_CONTEXT_CHARACTERS: u64 = 5;
pub const DEFAULT_CONTEXT_CHARACTERS: u64 = 15;
const ELLIPSIS: char = '…';

fn is_removal(line: &str) -> bool {
    line.starts_with('-') && !line.starts_with("--- ")
}

fn is_addition(line: &str) -> bool {
    line.starts_with('+') && !line.starts_with("+++ ")
}

const MAXIMUM_TABLE_CELLS: usize = 4_000_000;

struct CommonRun {
    removed_start: usize,
    added_start: usize,
    length: usize,
}

fn common_prefix_length(first: &[char], second: &[char]) -> usize {
    first
        .iter()
        .zip(second)
        .take_while(|(first_character, second_character)| first_character == second_character)
        .count()
}

fn common_suffix_length(first: &[char], second: &[char]) -> usize {
    first
        .iter()
        .rev()
        .zip(second.iter().rev())
        .take_while(|(first_character, second_character)| first_character == second_character)
        .count()
}

fn matched_middle_pairs(removed: &[char], added: &[char]) -> Vec<(usize, usize)> {
    let removed_length = removed.len();
    let added_length = added.len();
    let row_width = added_length + 1;

    if removed_length * added_length > MAXIMUM_TABLE_CELLS {
        return Vec::new();
    }

    let mut longest_common = vec![0u32; (removed_length + 1) * row_width];

    for removed_index in (0..removed_length).rev() {
        for added_index in (0..added_length).rev() {
            longest_common[removed_index * row_width + added_index] =
                if removed[removed_index] == added[added_index] {
                    longest_common[(removed_index + 1) * row_width + added_index + 1] + 1
                } else {
                    longest_common[(removed_index + 1) * row_width + added_index]
                        .max(longest_common[removed_index * row_width + added_index + 1])
                };
        }
    }

    let mut result = Vec::new();
    let mut removed_index = 0;
    let mut added_index = 0;

    while removed_index < removed_length && added_index < added_length {
        if removed[removed_index] == added[added_index] {
            result.push((removed_index, added_index));
            removed_index += 1;
            added_index += 1;
        } else if longest_common[(removed_index + 1) * row_width + added_index]
            >= longest_common[removed_index * row_width + added_index + 1]
        {
            removed_index += 1;
        } else {
            added_index += 1;
        }
    }
    result
}

fn common_runs(removed: &[char], added: &[char]) -> Vec<CommonRun> {
    let prefix_length = common_prefix_length(removed, added);
    let suffix_length = common_suffix_length(&removed[prefix_length..], &added[prefix_length..]);
    let middle_removed = &removed[prefix_length..removed.len() - suffix_length];
    let middle_added = &added[prefix_length..added.len() - suffix_length];

    let mut pairs: Vec<(usize, usize)> = (0..prefix_length).map(|index| (index, index)).collect();

    pairs.extend(
        matched_middle_pairs(middle_removed, middle_added)
            .into_iter()
            .map(|(removed_index, added_index)| {
                (removed_index + prefix_length, added_index + prefix_length)
            }),
    );

    pairs.extend((0..suffix_length).map(|offset| {
        (
            removed.len() - suffix_length + offset,
            added.len() - suffix_length + offset,
        )
    }));

    let mut result: Vec<CommonRun> = Vec::new();

    for (removed_index, added_index) in pairs {
        match result.last_mut() {
            Some(run)
                if run.removed_start + run.length == removed_index
                    && run.added_start + run.length == added_index =>
            {
                run.length += 1;
            }
            _ => result.push(CommonRun {
                removed_start: removed_index,
                added_start: added_index,
                length: 1,
            }),
        }
    }
    result
}

fn shorten_common_text(
    text: &[char],
    is_leading: bool,
    is_trailing: bool,
    context_characters: usize,
) -> String {
    let mut result = String::new();
    let length = text.len();

    if is_leading && length > context_characters {
        result.push(ELLIPSIS);
        result.extend(&text[length - context_characters..]);
    } else if is_trailing && length > context_characters {
        result.extend(&text[..context_characters]);
        result.push(ELLIPSIS);
    } else if !is_leading && !is_trailing && length > 2 * context_characters {
        result.extend(&text[..context_characters]);
        result.push(ELLIPSIS);
        result.extend(&text[length - context_characters..]);
    } else {
        result.extend(text);
    }

    result
}

fn render_line(
    characters: &[char],
    other_characters: &[char],
    runs: &[CommonRun],
    is_removed_side: bool,
    context_characters: usize,
) -> String {
    let mut result = String::new();
    let mut position = 0;

    for run in runs {
        let (start, other_start) = if is_removed_side {
            (run.removed_start, run.added_start)
        } else {
            (run.added_start, run.removed_start)
        };
        result.extend(&characters[position..start]);
        let is_leading = start == 0 && other_start == 0;
        let is_trailing = start + run.length == characters.len()
            && other_start + run.length == other_characters.len();
        result.push_str(&shorten_common_text(
            &characters[start..start + run.length],
            is_leading,
            is_trailing,
            context_characters,
        ));
        position = start + run.length;
    }

    result.extend(&characters[position..]);
    result
}

fn trim_pair(removed: &str, added: &str, context_characters: usize) -> (String, String) {
    let removed_characters: Vec<char> = removed[1..].chars().collect();
    let added_characters: Vec<char> = added[1..].chars().collect();
    let runs = common_runs(&removed_characters, &added_characters);

    (
        format!(
            "-{}",
            render_line(
                &removed_characters,
                &added_characters,
                &runs,
                true,
                context_characters
            )
        ),
        format!(
            "+{}",
            render_line(
                &added_characters,
                &removed_characters,
                &runs,
                false,
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
    fn omits_long_context_between_multiple_changes() {
        let middle = "m".repeat(40);
        let input = format!(
            "-aaaaaaaaaaaaaaaaaaaaXX{middle}Y{middle}ZZbbbbbbbbbbbbbbbbbbbb\n+aaaaaaaaaaaaaaaaaaaaxx{middle}y{middle}zzbbbbbbbbbbbbbbbbbbbb\n"
        );
        let result = trim_context(&input, 5);
        assert_eq!(
            result,
            "-…aaaaaXXmmmmm…mmmmmYmmmmm…mmmmmZZbbbbb…\n+…aaaaaxxmmmmm…mmmmmymmmmm…mmmmmzzbbbbb…\n"
        );
    }

    #[test]
    fn keeps_short_context_between_multiple_changes() {
        let result = focus("-aXbbbbbbbbbYccc\n+aPbbbbbbbbbQccc\n");
        assert_eq!(result, "-aXbbbbbbbbbYccc\n+aPbbbbbbbbbQccc\n");
    }

    #[test]
    fn handles_the_multiple_change_prose_example() {
        let removed = "-… technical work  and review PRs, perform QA and unblock engineers.  I designed the fulfillment system and lab partner API for quip's first custom oral appliance, and I kept reliability visible with Datadog metrics and alerts so the team hears about problems early.  I use Claude Code every day, and have built MCP servers with access control and privacy in mind  and I'd like t…";
        let added = "+… technical work: I   review PRs, perform QA and unblock engineers.  I designed the fulfillment system and lab partner API for quip's first custom oral appliance, and I kept reliability visible with Datadog metrics and alerts so the team hears about problems early.  I use Claude Code every day  and have built MCP servers with access control and privacy in mind, and I'd like t…";
        let result = focus(&format!("{removed}\n{added}\n"));
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].len() < removed.len() / 2, "{result}");
        assert!(lines[0].contains("technical work  and review"), "{result}");
        assert!(lines[1].contains("technical work: I   review"), "{result}");
        assert!(lines[0].contains("every day, and have built"), "{result}");
        assert!(lines[1].contains("every day  and have built"), "{result}");
        assert!(lines[0].contains("in mind  and I"), "{result}");
        assert!(lines[1].contains("in mind, and I"), "{result}");
    }

    #[test]
    fn falls_back_to_one_changed_region_for_enormous_lines() {
        let long_text = "a".repeat(3000);
        let input = format!("-{long_text}X{long_text}\n+{long_text}Y{long_text}\n");
        let result = trim_context(&input, 5);
        assert_eq!(result, "-…aaaaaXaaaaa…\n+…aaaaaYaaaaa…\n");
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
