use std::cmp::Ordering;

use chrono::{DateTime, FixedOffset};

use crate::store::{Store, StoreError, StoredBookmark};

pub fn list(store: &Store) -> Result<Vec<StoredBookmark>, StoreError> {
    let mut bookmarks = store.bookmarks()?;
    bookmarks.sort_by(by_added_desc_then_path);
    Ok(bookmarks)
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
