use serde::Serialize;

use crate::frontmatter::Frontmatter;
use crate::store::StoredBookmark;

#[derive(Serialize)]
struct BookmarkRecord<'a> {
    url: &'a str,
    title: Option<&'a str>,
    tags: &'a [String],
    added: Option<&'a str>,
    description: Option<&'a str>,
    space: Option<&'a str>,
}

const NO_TAGS: &[String] = &[];

impl<'a> From<&'a Frontmatter> for BookmarkRecord<'a> {
    fn from(fm: &'a Frontmatter) -> Self {
        BookmarkRecord {
            url: &fm.url,
            title: fm.title.as_deref(),
            tags: fm.tags.as_deref().unwrap_or(NO_TAGS),
            added: fm.added.as_deref(),
            description: fm.description.as_deref(),
            space: fm.space.as_deref(),
        }
    }
}

pub fn render(bookmarks: &[&StoredBookmark]) -> String {
    let records: Vec<BookmarkRecord> = bookmarks
        .iter()
        .map(|b| BookmarkRecord::from(&b.frontmatter))
        .collect();
    serde_json::to_string_pretty(&records).expect("bookmark records are always serializable")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn bookmark(fm: Frontmatter) -> StoredBookmark {
        StoredBookmark {
            path: PathBuf::from("x.md"),
            frontmatter: fm,
        }
    }

    #[test]
    fn empty_input_renders_an_empty_array() {
        assert_eq!(render(&[]), "[]");
    }

    #[test]
    fn every_field_is_present_and_named_to_schema() {
        let fm = Frontmatter {
            url: "https://example.com/a?utm=1".to_string(),
            title: Some("Example".to_string()),
            tags: Some(vec!["rust".to_string(), "cli".to_string()]),
            added: Some("2026-01-01T00:00:00+00:00".to_string()),
            description: Some("a note".to_string()),
            space: Some("work".to_string()),
        };
        let b = bookmark(fm);
        let value: serde_json::Value = serde_json::from_str(&render(&[&b])).unwrap();
        let record = &value.as_array().unwrap()[0];

        assert_eq!(record["url"], "https://example.com/a?utm=1");
        assert_eq!(record["title"], "Example");
        assert_eq!(record["tags"], serde_json::json!(["rust", "cli"]));
        assert_eq!(record["added"], "2026-01-01T00:00:00+00:00");
        assert_eq!(record["description"], "a note");
        assert_eq!(record["space"], "work");
    }

    #[test]
    fn unset_scalars_are_null_and_absent_tags_are_an_empty_array() {
        let fm = Frontmatter {
            url: "https://example.com".to_string(),
            title: None,
            tags: None,
            added: None,
            description: None,
            space: None,
        };
        let b = bookmark(fm);
        let value: serde_json::Value = serde_json::from_str(&render(&[&b])).unwrap();
        let record = &value.as_array().unwrap()[0];

        assert_eq!(record["url"], "https://example.com");
        assert!(record["title"].is_null());
        assert_eq!(record["tags"], serde_json::json!([]));
        assert!(record["added"].is_null());
        assert!(record["description"].is_null());
        assert!(record["space"].is_null());
    }
}
