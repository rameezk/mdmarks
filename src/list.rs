use std::cmp::Ordering;

use chrono::{DateTime, FixedOffset};

use crate::store::{Store, StoreError, StoredBookmark};

pub fn list(store: &Store, space: Option<&str>) -> Result<Vec<StoredBookmark>, StoreError> {
    let mut bookmarks = bookmarks_in_space(store, space)?;
    bookmarks.sort_by(by_added_desc_then_path);
    Ok(bookmarks)
}

pub fn bookmarks_in_space(
    store: &Store,
    space: Option<&str>,
) -> Result<Vec<StoredBookmark>, StoreError> {
    let mut bookmarks = store.bookmarks()?;
    retain_space(&mut bookmarks, space);
    Ok(bookmarks)
}

fn retain_space(bookmarks: &mut Vec<StoredBookmark>, space: Option<&str>) {
    if let Some(space) = space {
        bookmarks.retain(|b| b.frontmatter.space.as_deref() == Some(space));
    }
}

pub fn render_line(bookmark: &StoredBookmark) -> String {
    let fm = &bookmark.frontmatter;
    match &fm.title {
        Some(title) => format!("{title}  {}", fm.url),
        None => fm.url.clone(),
    }
}

pub(crate) fn by_added_desc_then_path(a: &StoredBookmark, b: &StoredBookmark) -> Ordering {
    match (added_key(a), added_key(b)) {
        (Some(x), Some(y)) => y.cmp(&x).then_with(|| a.path.cmp(&b.path)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => a.path.cmp(&b.path),
    }
}

fn added_key(bookmark: &StoredBookmark) -> Option<DateTime<FixedOffset>> {
    let added = bookmark.frontmatter.added.as_deref()?;
    DateTime::parse_from_rfc3339(added).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontmatter::Frontmatter;
    use std::path::PathBuf;

    fn bm(path: &str, space: Option<&str>) -> StoredBookmark {
        StoredBookmark {
            path: PathBuf::from(path),
            frontmatter: Frontmatter {
                url: "https://example.com".to_string(),
                title: None,
                tags: None,
                added: None,
                description: None,
                space: space.map(str::to_string),
            },
        }
    }

    fn stems(bookmarks: &[StoredBookmark]) -> Vec<String> {
        bookmarks
            .iter()
            .map(|b| b.path.file_stem().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn none_keeps_every_bookmark() {
        let mut books = vec![bm("a", Some("work")), bm("b", None), bm("c", Some("home"))];
        retain_space(&mut books, None);
        assert_eq!(stems(&books), vec!["a", "b", "c"]);
    }

    #[test]
    fn keeps_only_exact_space_matches() {
        let mut books = vec![
            bm("a", Some("work")),
            bm("b", Some("home")),
            bm("c", Some("work")),
        ];
        retain_space(&mut books, Some("work"));
        assert_eq!(stems(&books), vec!["a", "c"]);
    }

    #[test]
    fn unset_space_is_excluded() {
        let mut books = vec![bm("a", Some("work")), bm("b", None)];
        retain_space(&mut books, Some("work"));
        assert_eq!(stems(&books), vec!["a"]);
    }

    #[test]
    fn filter_is_exact_not_fuzzy() {
        let mut books = vec![bm("a", Some("work")), bm("b", Some("work-stuff"))];
        retain_space(&mut books, Some("work"));
        assert_eq!(stems(&books), vec!["a"]);
    }

    #[test]
    fn no_match_yields_empty() {
        let mut books = vec![bm("a", Some("work")), bm("b", None)];
        retain_space(&mut books, Some("nonexistent"));
        assert!(books.is_empty());
    }
}
