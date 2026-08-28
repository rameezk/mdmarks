use std::path::Path;

use crate::frontmatter::{self, Frontmatter};
use crate::slug::slug;
use crate::store::{Store, StoreError};

pub struct ParsedBookmark {
    pub url: String,
    pub title: String,
}

pub struct ImportSummary {
    pub imported: usize,
}

#[derive(Debug)]
pub enum ImportError {
    Read(std::io::Error),
    Store(StoreError),
    Frontmatter(frontmatter::FrontmatterError),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::Read(e) => write!(f, "reading export: {e}"),
            ImportError::Store(e) => write!(f, "{e}"),
            ImportError::Frontmatter(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ImportError {}

pub fn import(store: &Store, file: &Path) -> Result<ImportSummary, ImportError> {
    let html = std::fs::read_to_string(file).map_err(ImportError::Read)?;
    let parsed = parse_netscape(&html);

    for bookmark in &parsed {
        let fm = Frontmatter::imported(bookmark.url.clone(), bookmark.title.clone());
        let content = frontmatter::serialize(&fm, "").map_err(ImportError::Frontmatter)?;
        store
            .write_bookmark(&slug(&bookmark.title), &content)
            .map_err(ImportError::Store)?;
    }

    Ok(ImportSummary {
        imported: parsed.len(),
    })
}

pub fn parse_netscape(html: &str) -> Vec<ParsedBookmark> {
    let lower = html.to_ascii_lowercase();
    let mut bookmarks = Vec::new();
    let mut cursor = 0;

    while let Some(rel) = lower[cursor..].find("<a ") {
        let tag_start = cursor + rel;
        let Some(close_rel) = lower[tag_start..].find('>') else {
            break;
        };
        let tag_end = tag_start + close_rel;
        let open_tag = &html[tag_start..tag_end];

        let text_start = tag_end + 1;
        let Some(text_rel) = lower[text_start..].find("</a>") else {
            break;
        };
        let text_end = text_start + text_rel;

        if let Some(url) = href_value(open_tag) {
            let title = html[text_start..text_end].trim().to_string();
            bookmarks.push(ParsedBookmark { url, title });
        }

        cursor = text_end + "</a>".len();
    }

    bookmarks
}

fn href_value(open_tag: &str) -> Option<String> {
    let lower = open_tag.to_ascii_lowercase();
    let attr = lower.find("href=")?;
    let after = attr + "href=".len();
    let value = open_tag[after..].strip_prefix('"')?;
    let end = value.find('"')?;
    Some(value[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/flat-export.html");

    #[test]
    fn parses_one_bookmark_per_link() {
        let bookmarks = parse_netscape(FIXTURE);
        assert_eq!(bookmarks.len(), 3);
    }

    #[test]
    fn keeps_href_verbatim_with_tracking_params() {
        let bookmarks = parse_netscape(FIXTURE);
        assert_eq!(
            bookmarks[0].url,
            "https://example.com/a?utm_source=news&id=7"
        );
    }

    #[test]
    fn title_is_the_link_text() {
        let bookmarks = parse_netscape(FIXTURE);
        assert_eq!(bookmarks[0].title, "Example Page");
        assert_eq!(bookmarks[1].title, "The Rust Programming Language");
    }

    #[test]
    fn ignores_add_date_and_other_attributes() {
        let bookmarks = parse_netscape(FIXTURE);
        assert_eq!(bookmarks[1].url, "https://www.rust-lang.org/");
    }

    #[test]
    fn empty_input_yields_no_bookmarks() {
        assert!(parse_netscape("").is_empty());
    }
}
