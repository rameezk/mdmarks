use crate::list::by_added_desc_then_path;
use crate::store::StoredBookmark;

const MULTIPLIER_TITLE: i32 = 3;
const MULTIPLIER_TAGS: i32 = 2;
const MULTIPLIER_URL: i32 = 1;

const SCORE_MATCH: i32 = 16;
const BONUS_BOUNDARY: i32 = 8;
const BONUS_CONTIGUOUS: i32 = 8;
const EARLINESS_SPAN: i32 = 8;
const GAP_PENALTY_CAP: i32 = 6;

pub fn rank<'a>(bookmarks: &'a [StoredBookmark], query: &str) -> Vec<&'a StoredBookmark> {
    let query = query.trim();

    if query.is_empty() {
        let mut all: Vec<&StoredBookmark> = bookmarks.iter().collect();
        all.sort_by(|a, b| by_added_desc_then_path(a, b));
        return all;
    }

    let mut scored: Vec<(i32, &StoredBookmark)> = bookmarks
        .iter()
        .filter_map(|b| best_field_score(query, b).map(|s| (s, b)))
        .collect();
    scored.sort_by(|(sa, a), (sb, b)| sb.cmp(sa).then_with(|| by_added_desc_then_path(a, b)));
    scored.into_iter().map(|(_, b)| b).collect()
}

fn best_field_score(query: &str, bookmark: &StoredBookmark) -> Option<i32> {
    let fm = &bookmark.frontmatter;
    let mut best: Option<i32> = None;
    let mut consider = |value: Option<i32>| {
        if let Some(v) = value {
            best = Some(best.map_or(v, |b| b.max(v)));
        }
    };

    if let Some(title) = &fm.title {
        consider(fuzzy_score(query, title).map(|s| s * MULTIPLIER_TITLE));
    }
    if let Some(tags) = &fm.tags {
        let best_tag = tags.iter().filter_map(|tag| fuzzy_score(query, tag)).max();
        consider(best_tag.map(|s| s * MULTIPLIER_TAGS));
    }
    consider(fuzzy_score(query, &fm.url).map(|s| s * MULTIPLIER_URL));

    best
}

fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    let q: Vec<char> = query.chars().map(|c| c.to_ascii_lowercase()).collect();
    if q.is_empty() {
        return None;
    }
    let lc: Vec<char> = candidate.chars().map(|c| c.to_ascii_lowercase()).collect();
    let (m, n) = (q.len(), lc.len());
    if m > n {
        return None;
    }

    let mut prev_row = vec![i32::MIN; n];
    for (j, &ch) in lc.iter().enumerate() {
        if ch == q[0] {
            prev_row[j] = SCORE_MATCH + boundary_bonus(&lc, j) + earliness_bonus(j);
        }
    }

    for (k, &qk) in q.iter().enumerate().skip(1) {
        let mut row = vec![i32::MIN; n];
        for j in k..n {
            if lc[j] != qk {
                continue;
            }
            let mut cell = i32::MIN;
            for (prev, &prior) in prev_row.iter().enumerate().take(j).skip(k - 1) {
                if prior == i32::MIN {
                    continue;
                }
                let contiguous = if j == prev + 1 { BONUS_CONTIGUOUS } else { 0 };
                let gap = if j > prev + 1 {
                    ((j - prev - 1) as i32).min(GAP_PENALTY_CAP)
                } else {
                    0
                };
                let value = prior + SCORE_MATCH + boundary_bonus(&lc, j) + contiguous - gap;
                cell = cell.max(value);
            }
            row[j] = cell;
        }
        prev_row = row;
    }

    prev_row
        .iter()
        .copied()
        .filter(|&v| v != i32::MIN)
        .max()
        .map(|v| v.max(1))
}

fn boundary_bonus(lc: &[char], j: usize) -> i32 {
    if is_boundary(lc, j) {
        BONUS_BOUNDARY
    } else {
        0
    }
}

fn is_boundary(lc: &[char], j: usize) -> bool {
    j == 0 || !lc[j - 1].is_alphanumeric()
}

fn earliness_bonus(first_match: usize) -> i32 {
    (EARLINESS_SPAN - first_match as i32).max(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontmatter::Frontmatter;
    use std::path::PathBuf;

    fn bm(
        path: &str,
        title: Option<&str>,
        tags: Option<&[&str]>,
        url: &str,
        added: Option<&str>,
    ) -> StoredBookmark {
        StoredBookmark {
            path: PathBuf::from(path),
            frontmatter: Frontmatter {
                url: url.to_string(),
                title: title.map(str::to_string),
                tags: tags.map(|t| t.iter().map(|s| s.to_string()).collect()),
                added: added.map(str::to_string),
                description: None,
                space: None,
            },
        }
    }

    fn order(ranked: &[&StoredBookmark]) -> Vec<String> {
        ranked
            .iter()
            .map(|b| b.path.file_stem().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn contiguity_beats_scattered() {
        assert!(fuzzy_score("ab", "cab").unwrap() > fuzzy_score("ab", "axxb").unwrap());
    }

    #[test]
    fn start_of_word_beats_mid_word() {
        assert!(fuzzy_score("b", "a b").unwrap() > fuzzy_score("b", "ab").unwrap());
    }

    #[test]
    fn earlier_match_beats_later() {
        assert!(fuzzy_score("x", "xaaaaaa").unwrap() > fuzzy_score("x", "aaaaaax").unwrap());
    }

    #[test]
    fn case_insensitive() {
        assert!(fuzzy_score("RUST", "rust lang").is_some());
        assert!(fuzzy_score("rust", "RUST LANG").is_some());
    }

    #[test]
    fn non_subsequence_does_not_match() {
        assert!(fuzzy_score("ba", "ab").is_none());
        assert!(fuzzy_score("longer", "lng").is_none());
    }

    fn corpus() -> Vec<StoredBookmark> {
        vec![
            bm(
                "rustbook",
                Some("The Rust Programming Language"),
                Some(&["rust", "lang"]),
                "https://doc.rust-lang.org/book/",
                Some("2026-01-01T00:00:00+00:00"),
            ),
            bm(
                "trust",
                Some("Building Trust in Teams"),
                Some(&["management"]),
                "https://example.com/trust",
                Some("2026-02-01T00:00:00+00:00"),
            ),
            bm(
                "python",
                Some("Python Docs"),
                Some(&["python"]),
                "https://python.org",
                Some("2026-03-01T00:00:00+00:00"),
            ),
            bm(
                "weekend",
                Some("Weekend Plans"),
                None,
                "https://example.com/weekend",
                Some("2026-04-01T00:00:00+00:00"),
            ),
        ]
    }

    #[test]
    fn ranks_table() {
        let books = corpus();
        let cases: &[(&str, &[&str])] = &[
            ("rust", &["rustbook", "trust"]),
            ("py", &["python"]),
            ("plan", &["weekend", "rustbook"]),
            ("", &["weekend", "python", "trust", "rustbook"]),
            ("   ", &["weekend", "python", "trust", "rustbook"]),
            ("zzznomatch", &[]),
        ];
        for (query, expected) in cases {
            let got = order(&rank(&books, query));
            assert_eq!(&got, expected, "query {query:?}");
        }
    }

    #[test]
    fn empty_query_returns_all_added_desc() {
        let books = vec![
            bm(
                "old",
                Some("Old"),
                None,
                "https://a",
                Some("2020-01-01T00:00:00+00:00"),
            ),
            bm(
                "new",
                Some("New"),
                None,
                "https://b",
                Some("2026-01-01T00:00:00+00:00"),
            ),
            bm(
                "mid",
                Some("Mid"),
                None,
                "https://c",
                Some("2023-01-01T00:00:00+00:00"),
            ),
        ];
        assert_eq!(order(&rank(&books, "")), vec!["new", "mid", "old"]);
        assert_eq!(order(&rank(&books, "   ")), vec!["new", "mid", "old"]);
    }

    #[test]
    fn title_outranks_url_at_equal_quality() {
        let by_title = bm(
            "titled",
            Some("alpha"),
            None,
            "zzzzzzzz",
            Some("2020-01-01T00:00:00+00:00"),
        );
        let by_url = bm(
            "urled",
            Some("zzzzzzzz"),
            None,
            "alpha",
            Some("2026-06-01T00:00:00+00:00"),
        );
        let books = vec![by_url, by_title];
        assert_eq!(
            order(&rank(&books, "alpha")),
            vec!["titled", "urled"],
            "identical match string in both fields, so identical quality; the title multiplier alone orders it first, though the url bookmark is newer"
        );
    }

    #[test]
    fn best_field_wins_not_summed() {
        let strong_single = bm(
            "strong",
            Some("kubernetes"),
            None,
            "https://example.com/k8s",
            Some("2026-01-01T00:00:00+00:00"),
        );
        let weak_many = bm(
            "weak",
            Some("k projects u b e archive"),
            Some(&["k", "u"]),
            "https://k.example.com/u/b/e",
            Some("2026-06-01T00:00:00+00:00"),
        );
        let books = vec![weak_many, strong_single];
        assert_eq!(
            order(&rank(&books, "kube"))[0],
            "strong",
            "one strong field beats several weak ones; scores are not summed"
        );
    }

    #[test]
    fn added_desc_breaks_score_ties() {
        let older = bm(
            "older",
            Some("rust"),
            None,
            "https://a",
            Some("2020-01-01T00:00:00+00:00"),
        );
        let newer = bm(
            "newer",
            Some("rust"),
            None,
            "https://b",
            Some("2026-01-01T00:00:00+00:00"),
        );
        let books = vec![older, newer];
        assert_eq!(order(&rank(&books, "rust")), vec!["newer", "older"]);
    }
}
