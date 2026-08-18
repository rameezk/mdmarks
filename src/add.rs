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
) -> Result<AddOutcome, AddError> {
    validate_http_url(url_input)?;

    let target = normalize(url_input).map_err(|_| AddError::InvalidUrl(url_input.to_string()))?;

    for bookmark in store.bookmarks().map_err(AddError::Store)? {
        if normalize(&bookmark.frontmatter.url).is_ok_and(|other| other == target) {
            let url = bookmark.frontmatter.url;
            let title = bookmark.frontmatter.title.unwrap_or_else(|| url.clone());
            return Ok(AddOutcome::Matched(BookmarkRef {
                path: bookmark.path,
                url,
                title,
            }));
        }
    }

    let title = resolve_title(override_title, url_input);
    let added = Utc::now().to_rfc3339();

    let fm = Frontmatter::new(url_input.to_string(), title.clone(), added);
    let content = frontmatter::serialize(&fm, "").map_err(AddError::Frontmatter)?;
    let path = store
        .write_bookmark(&slug(&title), &content)
        .map_err(AddError::Store)?;

    Ok(AddOutcome::Created(BookmarkRef {
        path,
        url: url_input.to_string(),
        title,
    }))
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
