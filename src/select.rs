use crate::store::{Store, StoreError, StoredBookmark};

pub fn by_exact_url(store: &Store, url: &str) -> Result<Option<StoredBookmark>, StoreError> {
    Ok(pick_exact_url(store.bookmarks()?, url))
}

fn pick_exact_url(bookmarks: Vec<StoredBookmark>, url: &str) -> Option<StoredBookmark> {
    let mut matches: Vec<StoredBookmark> = bookmarks
        .into_iter()
        .filter(|b| b.frontmatter.url == url)
        .collect();
    matches.sort_by(|a, b| a.path.cmp(&b.path));
    matches.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontmatter::Frontmatter;
    use std::path::PathBuf;

    fn bm(path: &str, url: &str) -> StoredBookmark {
        StoredBookmark {
            path: PathBuf::from(path),
            frontmatter: Frontmatter {
                url: url.to_string(),
                title: None,
                tags: None,
                added: None,
                description: None,
                space: None,
            },
        }
    }

    #[test]
    fn exact_url_matches_verbatim() {
        let books = vec![
            bm("a.md", "https://example.com/page"),
            bm("b.md", "https://example.com/other"),
        ];
        let hit = pick_exact_url(books, "https://example.com/page").unwrap();
        assert_eq!(hit.path, PathBuf::from("a.md"));
    }

    #[test]
    fn near_duplicate_does_not_match() {
        let books = vec![bm("a.md", "https://example.com/page")];
        assert!(pick_exact_url(books, "https://example.com/page/").is_none());
    }

    #[test]
    fn duplicate_stored_urls_select_by_stable_path_order() {
        let books = vec![
            bm("z.md", "https://example.com/dup"),
            bm("a.md", "https://example.com/dup"),
        ];
        let hit = pick_exact_url(books, "https://example.com/dup").unwrap();
        assert_eq!(hit.path, PathBuf::from("a.md"));
    }
}
