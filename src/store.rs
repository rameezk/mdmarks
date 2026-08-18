use std::path::{Path, PathBuf};

use crate::frontmatter::{self, Frontmatter};

pub struct Store {
    root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct StoredBookmark {
    pub path: PathBuf,
    pub frontmatter: Frontmatter,
}

#[derive(Debug)]
pub enum StoreError {
    Io(std::io::Error),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Io(e) => write!(f, "store i/o error: {e}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        StoreError::Io(e)
    }
}

impl Store {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Store { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn ensure_dir(&self) -> Result<(), StoreError> {
        std::fs::create_dir_all(&self.root)?;
        Ok(())
    }

    pub fn bookmarks(&self) -> Result<Vec<StoredBookmark>, StoreError> {
        let paths = self.bookmark_paths()?;
        Ok(paths
            .into_iter()
            .filter_map(|path| match std::fs::read_to_string(&path) {
                Ok(content) => {
                    frontmatter::parse(&content)
                        .ok()
                        .map(|(fm, _body)| StoredBookmark {
                            path,
                            frontmatter: fm,
                        })
                }
                Err(_) => None,
            })
            .collect())
    }

    pub fn write_bookmark(&self, slug: &str, content: &str) -> Result<PathBuf, StoreError> {
        self.ensure_dir()?;
        let path = self.free_path_for(slug);
        std::fs::write(&path, content)?;
        Ok(path)
    }

    fn bookmark_paths(&self) -> Result<Vec<PathBuf>, StoreError> {
        let entries = match std::fs::read_dir(&self.root) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(StoreError::Io(e)),
        };
        let mut paths = Vec::new();
        for entry in entries {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                paths.push(path);
            }
        }
        Ok(paths)
    }

    fn free_path_for(&self, slug: &str) -> PathBuf {
        let first = self.root.join(format!("{slug}.md"));
        if !first.exists() {
            return first;
        }
        let mut n = 2;
        loop {
            let candidate = self.root.join(format!("{slug}-{n}.md"));
            if !candidate.exists() {
                return candidate;
            }
            n += 1;
        }
    }
}
