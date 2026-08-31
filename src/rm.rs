use crate::select::by_exact_url;
use crate::store::{Store, StoreError, StoredBookmark};

#[derive(Debug)]
pub enum RmError {
    NotFound(String),
    Store(StoreError),
}

impl std::fmt::Display for RmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RmError::NotFound(url) => write!(f, "no bookmark with url: {url}"),
            RmError::Store(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for RmError {}

pub fn rm(store: &Store, url: &str) -> Result<StoredBookmark, RmError> {
    let target = by_exact_url(store, url)
        .map_err(RmError::Store)?
        .ok_or_else(|| RmError::NotFound(url.to_string()))?;
    store
        .remove_bookmark(&target.path)
        .map_err(RmError::Store)?;
    Ok(target)
}
