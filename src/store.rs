use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

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
        Ok(scan(paths))
    }

    pub fn write_bookmark(&self, slug: &str, content: &str) -> Result<PathBuf, StoreError> {
        self.ensure_dir()?;
        let path = self.free_path_for(slug);
        std::fs::write(&path, content)?;
        Ok(path)
    }

    pub fn remove_bookmark(&self, path: &Path) -> Result<(), StoreError> {
        std::fs::remove_file(path)?;
        Ok(())
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

fn scan(paths: Vec<PathBuf>) -> Vec<StoredBookmark> {
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(paths.len().max(1));

    if workers <= 1 {
        return paths.into_iter().filter_map(read_bookmark).collect();
    }

    let cursor = AtomicUsize::new(0);
    let paths = &paths;
    let cursor = &cursor;
    let mut results = Vec::with_capacity(paths.len());
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(move || {
                    let mut local = Vec::new();
                    loop {
                        let i = cursor.fetch_add(1, Ordering::Relaxed);
                        match paths.get(i) {
                            Some(path) => {
                                if let Some(b) = read_bookmark(path.clone()) {
                                    local.push(b);
                                }
                            }
                            None => break,
                        }
                    }
                    local
                })
            })
            .collect();
        for handle in handles {
            results.extend(handle.join().expect("scan worker panicked"));
        }
    });
    results
}

fn read_bookmark(path: PathBuf) -> Option<StoredBookmark> {
    let content = std::fs::read_to_string(&path).ok()?;
    let (frontmatter, _body) = frontmatter::parse(&content).ok()?;
    Some(StoredBookmark { path, frontmatter })
}
