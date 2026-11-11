use std::ops::Range;

pub const PAIR_THRESHOLD: f64 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChangeBlock {
    pub old_start: usize,
    pub old_end: usize,
    pub new_start: usize,
    pub new_end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct MatchingBlock {
    old_start: usize,
    new_start: usize,
    length: usize,
}

fn longest_match<Item: PartialEq>(
    old_items: &[Item],
    new_items: &[Item],
    old_range: Range<usize>,
    new_range: Range<usize>,
) -> MatchingBlock {
    let mut result = MatchingBlock {
        old_start: old_range.start,
        new_start: new_range.start,
        length: 0,
    };
    let row_width = new_range.len() + 1;
    let mut previous_row = vec![0usize; row_width];
    let mut current_row = vec![0usize; row_width];

    for old_index in old_range {
        for new_index in new_range.clone() {
            let column = new_index - new_range.start;

            if old_items[old_index] == new_items[new_index] {
                let length = previous_row[column] + 1;
                current_row[column + 1] = length;
                if length > result.length {
                    result = MatchingBlock {
                        old_start: old_index + 1 - length,
                        new_start: new_index + 1 - length,
                        length,
                    };
                }
            } else {
                current_row[column + 1] = 0;
            }
        }

        std::mem::swap(&mut previous_row, &mut current_row);
    }

    result
}

fn matching_blocks<Item: PartialEq>(old_items: &[Item], new_items: &[Item]) -> Vec<MatchingBlock> {
    let mut result = Vec::new();
    let mut pending = vec![(0..old_items.len(), 0..new_items.len())];

    while let Some((old_range, new_range)) = pending.pop() {
        let block = longest_match(old_items, new_items, old_range.clone(), new_range.clone());
        if block.length == 0 {
            continue;
        }

        result.push(block);
        if old_range.start < block.old_start && new_range.start < block.new_start {
            pending.push((
                old_range.start..block.old_start,
                new_range.start..block.new_start,
            ));
        }

        let old_after = block.old_start + block.length;
        let new_after = block.new_start + block.length;
        if old_after < old_range.end && new_after < new_range.end {
            pending.push((old_after..old_range.end, new_after..new_range.end));
        }
    }

    result.sort();
    result
}

pub fn change_blocks<Item: PartialEq>(old_items: &[Item], new_items: &[Item]) -> Vec<ChangeBlock> {
    let mut result = Vec::new();
    let mut old_cursor = 0;
    let mut new_cursor = 0;

    for matched in matching_blocks(old_items, new_items) {
        if old_cursor < matched.old_start || new_cursor < matched.new_start {
            result.push(ChangeBlock {
                old_start: old_cursor,
                old_end: matched.old_start,
                new_start: new_cursor,
                new_end: matched.new_start,
            });
        }
        old_cursor = matched.old_start + matched.length;
        new_cursor = matched.new_start + matched.length;
    }

    if old_cursor < old_items.len() || new_cursor < new_items.len() {
        result.push(ChangeBlock {
            old_start: old_cursor,
            old_end: old_items.len(),
            new_start: new_cursor,
            new_end: new_items.len(),
        });
    }

    result
}

pub fn ratio(first: &str, second: &str) -> f64 {
    let first_chars: Vec<char> = first.chars().collect();
    let second_chars: Vec<char> = second.chars().collect();
    let total_length = first_chars.len() + second_chars.len();

    if total_length == 0 {
        return 1.0;
    }

    let shorter_length = first_chars.len().min(second_chars.len());
    if 2.0 * (shorter_length as f64) / (total_length as f64) < PAIR_THRESHOLD {
        return 0.0;
    }

    let matched_length: usize = matching_blocks(&first_chars, &second_chars)
        .iter()
        .map(|block| block.length)
        .sum();

    2.0 * matched_length as f64 / total_length as f64
}

#[derive(Debug, PartialEq)]
pub enum Pair {
    Both(usize, usize),
    Del(usize),
    Add(usize),
}

const SCORE_TOLERANCE: f64 = 1e-9;

struct PairingTable {
    old_count: usize,
    new_count: usize,
    similarity: Vec<f64>,
    best_score: Vec<f64>,
}

impl PairingTable {
    fn new(old_lines: &[&str], new_lines: &[&str]) -> PairingTable {
        let mut table = PairingTable {
            old_count: old_lines.len(),
            new_count: new_lines.len(),
            similarity: Vec::new(),
            best_score: Vec::new(),
        };
        table.fill_similarity(old_lines, new_lines);
        table.fill_best_scores();
        table
    }

    fn similarity_at(&self, old_index: usize, new_index: usize) -> f64 {
        self.similarity[old_index * self.new_count + new_index]
    }

    fn best_score_at(&self, old_index: usize, new_index: usize) -> f64 {
        self.best_score[old_index * (self.new_count + 1) + new_index]
    }

    fn is_pairable(&self, old_index: usize, new_index: usize) -> bool {
        self.similarity_at(old_index, new_index) >= PAIR_THRESHOLD
    }

    fn fill_similarity(&mut self, old_lines: &[&str], new_lines: &[&str]) {
        self.similarity = Vec::with_capacity(self.old_count * self.new_count);
        for old_line in old_lines {
            for new_line in new_lines {
                self.similarity.push(ratio(old_line, new_line));
            }
        }
    }

    fn fill_best_scores(&mut self) {
        self.best_score = vec![0.0; (self.old_count + 1) * (self.new_count + 1)];
        for old_index in (0..self.old_count).rev() {
            for new_index in (0..self.new_count).rev() {
                let score = self.best_score_from(old_index, new_index);
                self.best_score[old_index * (self.new_count + 1) + new_index] = score;
            }
        }
    }

    fn best_score_from(&self, old_index: usize, new_index: usize) -> f64 {
        let skip_old = self.best_score_at(old_index + 1, new_index);
        let skip_new = self.best_score_at(old_index, new_index + 1);
        let skipping = skip_old.max(skip_new);
        if self.is_pairable(old_index, new_index) {
            skipping.max(self.score_if_paired(old_index, new_index))
        } else {
            skipping
        }
    }

    fn score_if_paired(&self, old_index: usize, new_index: usize) -> f64 {
        self.best_score_at(old_index + 1, new_index + 1) + self.similarity_at(old_index, new_index)
    }

    fn pairing_is_best(&self, old_index: usize, new_index: usize) -> bool {
        old_index < self.old_count
            && new_index < self.new_count
            && self.is_pairable(old_index, new_index)
            && scores_match(
                self.best_score_at(old_index, new_index),
                self.score_if_paired(old_index, new_index),
            )
    }

    fn skipping_old_is_best(&self, old_index: usize, new_index: usize) -> bool {
        old_index < self.old_count
            && (new_index == self.new_count
                || scores_match(
                    self.best_score_at(old_index, new_index),
                    self.best_score_at(old_index + 1, new_index),
                ))
    }
}

fn scores_match(first: f64, second: f64) -> bool {
    (first - second).abs() < SCORE_TOLERANCE
}

fn trace_pairing(table: &PairingTable) -> Vec<Pair> {
    let mut result = Vec::new();
    let mut old_index = 0;
    let mut new_index = 0;

    while old_index < table.old_count || new_index < table.new_count {
        if table.pairing_is_best(old_index, new_index) {
            result.push(Pair::Both(old_index, new_index));
            old_index += 1;
            new_index += 1;
        } else if table.skipping_old_is_best(old_index, new_index) {
            result.push(Pair::Del(old_index));
            old_index += 1;
        } else {
            result.push(Pair::Add(new_index));
            new_index += 1;
        }
    }

    result
}

pub fn pair_lines(old_lines: &[&str], new_lines: &[&str]) -> Vec<Pair> {
    trace_pairing(&PairingTable::new(old_lines, new_lines))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn characters(text: &str) -> Vec<char> {
        text.chars().collect()
    }

    fn block(old_start: usize, old_end: usize, new_start: usize, new_end: usize) -> ChangeBlock {
        ChangeBlock {
            old_start,
            old_end,
            new_start,
            new_end,
        }
    }

    #[test]
    fn identical_sequences_have_no_change_blocks() {
        assert!(change_blocks(&characters("abc"), &characters("abc")).is_empty());
    }

    #[test]
    fn both_empty_have_no_change_blocks() {
        let empty: Vec<char> = Vec::new();
        assert!(change_blocks(&empty, &empty).is_empty());
    }

    #[test]
    fn replacement_is_one_block() {
        assert_eq!(
            change_blocks(&characters("abc"), &characters("aXc")),
            vec![block(1, 2, 1, 2)]
        );
    }

    #[test]
    fn insertion_has_empty_old_side() {
        assert_eq!(
            change_blocks(&characters("ac"), &characters("abc")),
            vec![block(1, 1, 1, 2)]
        );
    }

    #[test]
    fn deletion_has_empty_new_side() {
        assert_eq!(
            change_blocks(&characters("abc"), &characters("ac")),
            vec![block(1, 2, 1, 1)]
        );
    }

    #[test]
    fn completely_different_is_one_block() {
        assert_eq!(
            change_blocks(&characters("abc"), &characters("xyz")),
            vec![block(0, 3, 0, 3)]
        );
    }

    #[test]
    fn against_empty_is_one_block() {
        let empty: Vec<char> = Vec::new();
        assert_eq!(
            change_blocks(&characters("ab"), &empty),
            vec![block(0, 2, 0, 0)]
        );
        assert_eq!(
            change_blocks(&empty, &characters("ab")),
            vec![block(0, 0, 0, 2)]
        );
    }

    #[test]
    fn leading_and_trailing_changes() {
        assert_eq!(
            change_blocks(&characters("xabcy"), &characters("abc")),
            vec![block(0, 1, 0, 0), block(4, 5, 3, 3)]
        );
    }

    #[test]
    fn longest_block_wins_over_scattered_matches() {
        let old_words = ["the", " ", "quick", " ", "fox"];
        let new_words = ["a", " ", "slow", " ", "dog"];
        assert_eq!(
            change_blocks(&old_words, &new_words),
            vec![block(0, 1, 0, 1), block(2, 3, 2, 3), block(4, 5, 4, 5)]
        );
    }

    #[test]
    fn ties_go_to_the_earliest_match() {
        assert_eq!(
            change_blocks(&characters("abab"), &characters("ab")),
            vec![block(2, 4, 2, 2)]
        );
    }

    #[test]
    fn ratio_of_identical_strings_is_one() {
        assert_eq!(ratio("hello world", "hello world"), 1.0);
    }

    #[test]
    fn ratio_of_two_empty_strings_is_one() {
        assert_eq!(ratio("", ""), 1.0);
    }

    #[test]
    fn ratio_of_disjoint_strings_is_zero() {
        assert_eq!(ratio("aaaa", "zzzz"), 0.0);
    }

    #[test]
    fn ratio_short_circuits_on_very_different_lengths() {
        assert_eq!(ratio("a", "abcdefghijklmnop"), 0.0);
    }

    #[test]
    fn ratio_is_partial_for_similar_strings() {
        let similarity = ratio("kitten", "sitting");
        assert!(similarity > 0.5 && similarity < 1.0, "{similarity}");
    }

    #[test]
    fn ratio_counts_characters_not_bytes() {
        assert_eq!(ratio("héllo", "héllo"), 1.0);
        assert!(ratio("héllo", "hello") > 0.7);
    }

    #[test]
    fn pairs_empty_inputs() {
        assert!(pair_lines(&[], &[]).is_empty());
        assert_eq!(pair_lines(&["x"], &[]), vec![Pair::Del(0)]);
        assert_eq!(pair_lines(&[], &["x"]), vec![Pair::Add(0)]);
    }

    #[test]
    fn pairs_similar_lines() {
        assert_eq!(
            pair_lines(&["the quick brown fox"], &["the quick red fox"]),
            vec![Pair::Both(0, 0)]
        );
    }

    #[test]
    fn dissimilar_lines_are_not_paired() {
        assert_eq!(
            pair_lines(&["aaaaaaaa"], &["zzzzzzzz"]),
            vec![Pair::Del(0), Pair::Add(0)]
        );
    }

    #[test]
    fn inserted_line_does_not_shift_pairing() {
        let old_lines = [
            "alpha beta gamma delta epsilon",
            "zeta eta theta iota kappa",
        ];
        let new_lines = [
            "a completely new unrelated line",
            "alpha beta GAMMA delta epsilon",
            "zeta eta theta iota KAPPA",
        ];
        assert_eq!(
            pair_lines(&old_lines, &new_lines),
            vec![Pair::Add(0), Pair::Both(0, 1), Pair::Both(1, 2)]
        );
    }

    #[test]
    fn deleted_line_does_not_shift_pairing() {
        let old_lines = [
            "alpha beta gamma delta epsilon",
            "a completely old unrelated line",
            "zeta eta theta iota kappa",
        ];
        let new_lines = [
            "alpha beta GAMMA delta epsilon",
            "zeta eta theta iota KAPPA",
        ];
        assert_eq!(
            pair_lines(&old_lines, &new_lines),
            vec![Pair::Both(0, 0), Pair::Del(1), Pair::Both(2, 1)]
        );
    }

    #[test]
    fn pairing_preserves_order() {
        let old_lines = ["one two three four", "five six seven eight"];
        let new_lines = ["five six seven EIGHT", "one two three FOUR"];
        let operations = pair_lines(&old_lines, &new_lines);
        let mut last_old_index = 0;
        let mut last_new_index = 0;
        for operation in &operations {
            if let Pair::Both(old_index, new_index) = operation {
                assert!(
                    *old_index >= last_old_index && *new_index >= last_new_index,
                    "{operations:?}"
                );
                last_old_index = *old_index;
                last_new_index = *new_index;
            }
        }
    }
}
