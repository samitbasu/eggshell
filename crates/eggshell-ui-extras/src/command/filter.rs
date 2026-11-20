//! TODO(prose): the one filter Combobox, Command and Autosuggest share: a query's words each the start of a word of an item's label or keywords.

use unicode_normalization::UnicodeNormalization as _;
use unicode_normalization::char::is_combining_mark;

/// TODO(prose): how well an item matches a query; the lower ranks first, and items of equal score keep the app's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Score(u32);

impl Score {
    /// TODO(prose): the best score: what an empty query gives every item.
    pub const BEST: Self = Self(0);

    /// TODO(prose): a score of `rank`, for a filter of the app's own; lower ranks first.
    pub fn new(rank: u32) -> Self {
        Self(rank)
    }
}

impl From<Score> for u32 {
    fn from(score: Score) -> Self {
        score.0
    }
}

/// TODO(prose): what a filter sees of an item: its label and its hidden keywords.
#[derive(Clone, Copy, Debug)]
pub struct Candidate<'a> {
    /// TODO(prose): the text the row shows.
    pub label: &'a str,
    /// TODO(prose): more words the item is found by, not shown.
    pub keywords: &'a [String],
}

/// The rank of a match: the label's first word, a later word of the label,
/// or a keyword.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Place {
    LabelStart,
    LabelWord,
    Keyword,
}

/// TODO(prose): the library's matcher: each word of `query` must start a word of the label or of a keyword, case and accents aside; a match at the start of the label ranks first, then one at a later word of it, then one in a keyword. An empty query matches everything at [`Score::BEST`].
pub fn word_prefix(query: &str, candidate: Candidate<'_>) -> Option<Score> {
    let wanted = words(query);
    let label = words(candidate.label);
    let keywords: Vec<String> = candidate.keywords.iter().flat_map(|k| words(k)).collect();
    let mut worst = Place::LabelStart;
    for word in &wanted {
        let place = label
            .iter()
            .position(|have| have.starts_with(word.as_str()))
            .map(|at| {
                if at == 0 {
                    Place::LabelStart
                } else {
                    Place::LabelWord
                }
            })
            .or_else(|| {
                keywords
                    .iter()
                    .any(|have| have.starts_with(word.as_str()))
                    .then_some(Place::Keyword)
            })?;
        worst = worst.max(place);
    }
    Some(Score(worst as u32))
}

/// The words of `text`, lower-cased and without accents: split at anything
/// that is not a letter or a digit.
fn words(text: &str) -> Vec<String> {
    let folded: String = text
        .nfd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect();
    folded
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The items of `scores` that matched, best first, ties in the given order.
pub(crate) fn ranked<T>(scored: impl IntoIterator<Item = (Option<Score>, T)>) -> Vec<T> {
    let mut kept: Vec<(Score, usize, T)> = scored
        .into_iter()
        .enumerate()
        .filter_map(|(at, (score, item))| Some((score?, at, item)))
        .collect();
    kept.sort_by_key(|&(score, at, _)| (score, at));
    kept.into_iter().map(|(_, _, item)| item).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score(query: &str, label: &str) -> Option<Score> {
        word_prefix(
            query,
            Candidate {
                label,
                keywords: &[],
            },
        )
    }

    #[test]
    fn every_word_of_the_query_must_start_a_word() {
        assert!(score("com pal", "Command palette").is_some());
        assert!(score("pal com", "Command palette").is_some());
        assert_eq!(
            score("cmdpl", "Command palette"),
            None,
            "no fuzzy subsequences"
        );
        assert_eq!(score("mand", "Command palette"), None, "not inside a word");
        assert_eq!(score("com xyz", "Command palette"), None);
    }

    #[test]
    fn case_and_accents_are_ignored() {
        assert!(score("ZUR", "zürich").is_some());
        assert!(score("zür", "Zurich").is_some());
        assert!(score("ecole", "École normale").is_some());
    }

    #[test]
    fn the_start_of_the_label_ranks_before_a_later_word_before_a_keyword() {
        let keywords = ["remove".to_owned()];
        let keyword = word_prefix(
            "rem",
            Candidate {
                label: "Delete",
                keywords: &keywords,
            },
        );
        let start = score("rem", "Remove line");
        let later = score("rem", "Line remove");
        assert!(
            start < later && later < keyword,
            "{start:?} {later:?} {keyword:?}"
        );
        assert!(keyword.is_some(), "a hidden keyword finds the item");
    }

    #[test]
    fn an_empty_query_matches_everything_at_the_best_score() {
        assert_eq!(score("", "Anything"), Some(Score::BEST));
        assert_eq!(score("  ", "Anything"), Some(Score::BEST));
    }

    #[test]
    fn ties_keep_the_given_order() {
        let ranked = ranked([
            (Some(Score::new(1)), "b"),
            (Some(Score::new(0)), "a"),
            (None, "gone"),
            (Some(Score::new(1)), "c"),
            (Some(Score::new(0)), "d"),
        ]);
        assert_eq!(ranked, ["a", "d", "b", "c"]);
    }
}
