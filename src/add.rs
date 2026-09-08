use std::path::PathBuf;

use chrono::Utc;
use url::Url;

use crate::fetch::fetch_title;
use crate::frontmatter::{self, Frontmatter};
use crate::normalize::normalize;
use crate::slug::slug;
use crate::store::Store;

pub struct BookmarkRef {
    pub path: PathBuf,
    pub url: String,
    pub title: String,
    pub space: Option<String>,
}

pub enum AddOutcome {
    Created(BookmarkRef),
    Matched(BookmarkRef),
}

#[derive(Debug)]
pub enum AddError {
    InvalidUrl(String),
    Store(crate::store::StoreError),
    Frontmatter(crate::frontmatter::FrontmatterError),
}

impl std::fmt::Display for AddError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddError::InvalidUrl(u) => write!(f, "not a valid http(s) url: {u}"),
            AddError::Store(e) => write!(f, "{e}"),
            AddError::Frontmatter(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for AddError {}

pub fn add(
    store: &Store,
    url_input: &str,
    override_title: Option<&str>,
    space: Option<&str>,
) -> Result<AddOutcome, AddError> {
    validate_http_url(url_input)?;

    let target = normalize(url_input).map_err(|_| AddError::InvalidUrl(url_input.to_string()))?;

    for bookmark in store.bookmarks().map_err(AddError::Store)? {
        if normalize(&bookmark.frontmatter.url).is_ok_and(|other| other == target) {
            let title = bookmark.frontmatter.display_title().to_string();
            return Ok(AddOutcome::Matched(BookmarkRef {
                path: bookmark.path,
                title,
                space: bookmark.frontmatter.space,
                url: bookmark.frontmatter.url,
            }));
        }
    }

    let space = normalize_space(space);
    let title = resolve_title(override_title, url_input);
    let added = Utc::now().to_rfc3339();

    let mut fm = Frontmatter::new(url_input.to_string(), title.clone(), added);
    fm.space = space.clone();
    let content = frontmatter::serialize(&fm, "").map_err(AddError::Frontmatter)?;
    let path = store
        .write_bookmark(&slug(&title), &content)
        .map_err(AddError::Store)?;

    Ok(AddOutcome::Created(BookmarkRef {
        path,
        url: url_input.to_string(),
        title,
        space,
    }))
}

fn normalize_space(space: Option<&str>) -> Option<String> {
    space
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn resolve_title(override_title: Option<&str>, url: &str) -> String {
    match override_title {
        Some(t) => t.to_string(),
        None => fetch_title(url).unwrap_or_else(|| url.to_string()),
    }
}

fn validate_http_url(input: &str) -> Result<(), AddError> {
    let parsed = Url::parse(input).map_err(|_| AddError::InvalidUrl(input.to_string()))?;
    match parsed.scheme() {
        "http" | "https" if parsed.host_str().is_some() => Ok(()),
        _ => Err(AddError::InvalidUrl(input.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn matched_outcome_reflects_existing_bookmark_space() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        let url = "https://example.com/page";

        add(&store, url, Some("Page"), Some("work")).unwrap();

        let outcome = add(&store, url, Some("Page"), None).unwrap();
        match outcome {
            AddOutcome::Matched(b) => assert_eq!(b.space.as_deref(), Some("work")),
            AddOutcome::Created(_) => panic!("expected a dedup match, got a fresh create"),
        }
    }
}
