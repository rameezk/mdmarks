use crate::select::by_exact_url;
use crate::store::{Store, StoreError, StoredBookmark};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSpec {
    pub browser: Option<String>,
    pub profile_arg: Option<String>,
    pub url: String,
}

pub fn build_argv(spec: &LaunchSpec) -> Vec<String> {
    vec!["open".to_string(), spec.url.clone()]
}

pub trait Launcher {
    fn launch(&self, spec: &LaunchSpec) -> Result<(), LaunchError>;
}

#[derive(Debug)]
pub enum LaunchError {
    Spawn(std::io::Error),
    Failed(std::process::ExitStatus),
}

impl std::fmt::Display for LaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LaunchError::Spawn(e) => write!(f, "could not launch: {e}"),
            LaunchError::Failed(status) => write!(f, "launch command failed: {status}"),
        }
    }
}

impl std::error::Error for LaunchError {}

pub struct SystemLauncher;

impl Launcher for SystemLauncher {
    fn launch(&self, spec: &LaunchSpec) -> Result<(), LaunchError> {
        let argv = build_argv(spec);
        let status = std::process::Command::new(&argv[0])
            .args(&argv[1..])
            .status()
            .map_err(LaunchError::Spawn)?;
        if status.success() {
            Ok(())
        } else {
            Err(LaunchError::Failed(status))
        }
    }
}

#[derive(Debug)]
pub enum OpenError {
    NotFound(String),
    Store(StoreError),
    Launch(LaunchError),
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenError::NotFound(url) => write!(f, "no bookmark with url: {url}"),
            OpenError::Store(e) => write!(f, "{e}"),
            OpenError::Launch(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for OpenError {}

pub fn open(
    store: &Store,
    url: &str,
    launcher: &dyn Launcher,
) -> Result<StoredBookmark, OpenError> {
    let target = by_exact_url(store, url)
        .map_err(OpenError::Store)?
        .ok_or_else(|| OpenError::NotFound(url.to_string()))?;
    let spec = LaunchSpec {
        browser: None,
        profile_arg: None,
        url: target.frontmatter.url.clone(),
    };
    launcher.launch(&spec).map_err(OpenError::Launch)?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use tempfile::TempDir;

    #[derive(Default)]
    struct RecordingLauncher {
        argvs: RefCell<Vec<Vec<String>>>,
    }

    impl Launcher for RecordingLauncher {
        fn launch(&self, spec: &LaunchSpec) -> Result<(), LaunchError> {
            self.argvs.borrow_mut().push(build_argv(spec));
            Ok(())
        }
    }

    fn seed(store: &Store, name: &str, url: &str, title: &str) {
        store.ensure_dir().unwrap();
        let content = format!("---\nurl: {url}\ntitle: {title}\n---\n\n");
        std::fs::write(store.root().join(name), content).unwrap();
    }

    #[test]
    fn spaceless_bookmark_records_launchservices_default_argv() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        let url = "https://example.com/page?utm_source=news&id=7";
        seed(&store, "a.md", url, "Page");

        let launcher = RecordingLauncher::default();
        let opened = open(&store, url, &launcher).unwrap();

        assert_eq!(opened.frontmatter.url, url);
        assert_eq!(
            *launcher.argvs.borrow(),
            vec![vec!["open".to_string(), url.to_string()]]
        );
    }

    #[test]
    fn launched_url_is_verbatim_with_tracking_params_intact() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        let url = "https://example.com/x?utm_campaign=spring&ref=twitter#frag";
        seed(&store, "a.md", url, "X");

        let launcher = RecordingLauncher::default();
        open(&store, url, &launcher).unwrap();

        assert_eq!(launcher.argvs.borrow()[0][1], url);
    }

    #[test]
    fn no_match_errors_and_launches_nothing() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        seed(&store, "a.md", "https://example.com/a", "A");

        let launcher = RecordingLauncher::default();
        let err = open(&store, "https://example.com/nope", &launcher).unwrap_err();

        assert!(matches!(err, OpenError::NotFound(_)));
        assert!(launcher.argvs.borrow().is_empty(), "nothing launched");
    }

    #[test]
    fn near_duplicate_url_does_not_match() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        seed(&store, "a.md", "https://example.com/page", "Page");

        let launcher = RecordingLauncher::default();
        for near in [
            "https://example.com/page/",
            "http://example.com/page",
            "https://example.com/page?utm_source=news",
        ] {
            let err = open(&store, near, &launcher).unwrap_err();
            assert!(matches!(err, OpenError::NotFound(_)), "{near}");
        }
        assert!(launcher.argvs.borrow().is_empty());
    }
}
