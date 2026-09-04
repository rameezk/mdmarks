use std::collections::HashMap;

use crate::config::SpaceConfig;
use crate::select::by_exact_url;
use crate::store::{Store, StoreError, StoredBookmark};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSpec {
    pub browser: Option<String>,
    pub profile_arg: Option<String>,
    pub url: String,
}

pub struct SpaceResolver<'a> {
    pub override_space: Option<&'a str>,
    pub default_space: Option<&'a str>,
    pub spaces: &'a HashMap<String, SpaceConfig>,
}

pub fn build_argv(spec: &LaunchSpec) -> Vec<String> {
    match &spec.browser {
        None => vec!["open".to_string(), spec.url.clone()],
        Some(browser) => vec![
            "open".to_string(),
            "-a".to_string(),
            browser.clone(),
            spec.url.clone(),
        ],
    }
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
    UnknownSpace(String),
    ProfileUnsupported { space: String, browser: String },
    Store(StoreError),
    Launch(LaunchError),
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenError::NotFound(url) => write!(f, "no bookmark with url: {url}"),
            OpenError::UnknownSpace(name) => {
                write!(f, "space \"{name}\" is not defined in config")
            }
            OpenError::ProfileUnsupported { space, browser } => write!(
                f,
                "space \"{space}\" sets a {browser} profile, which is not supported yet"
            ),
            OpenError::Store(e) => write!(f, "{e}"),
            OpenError::Launch(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for OpenError {}

pub fn open(
    store: &Store,
    url: &str,
    resolver: &SpaceResolver,
    launcher: &dyn Launcher,
) -> Result<StoredBookmark, OpenError> {
    let target = by_exact_url(store, url)
        .map_err(OpenError::Store)?
        .ok_or_else(|| OpenError::NotFound(url.to_string()))?;
    let spec = resolve_launch_spec(resolver, &target)?;
    launcher.launch(&spec).map_err(OpenError::Launch)?;
    Ok(target)
}

fn resolve_launch_spec(
    resolver: &SpaceResolver,
    target: &StoredBookmark,
) -> Result<LaunchSpec, OpenError> {
    let url = target.frontmatter.url.clone();
    let name = resolver
        .override_space
        .or(target.frontmatter.space.as_deref())
        .or(resolver.default_space);

    let name = match name {
        None => {
            return Ok(LaunchSpec {
                browser: None,
                profile_arg: None,
                url,
            })
        }
        Some(name) => name,
    };

    let space = resolver
        .spaces
        .get(name)
        .ok_or_else(|| OpenError::UnknownSpace(name.to_string()))?;

    if space.profile.is_some() {
        return Err(OpenError::ProfileUnsupported {
            space: name.to_string(),
            browser: space.browser.clone(),
        });
    }

    Ok(LaunchSpec {
        browser: Some(space.browser.clone()),
        profile_arg: None,
        url,
    })
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

    fn seed_with_space(store: &Store, name: &str, url: &str, title: &str, space: &str) {
        store.ensure_dir().unwrap();
        let content = format!("---\nurl: {url}\ntitle: {title}\nspace: {space}\n---\n\n");
        std::fs::write(store.root().join(name), content).unwrap();
    }

    fn spaces(pairs: &[(&str, &str, Option<&str>)]) -> HashMap<String, SpaceConfig> {
        pairs
            .iter()
            .map(|(name, browser, profile)| {
                (
                    name.to_string(),
                    SpaceConfig {
                        browser: browser.to_string(),
                        profile: profile.map(str::to_string),
                    },
                )
            })
            .collect()
    }

    fn resolver<'a>(
        override_space: Option<&'a str>,
        default_space: Option<&'a str>,
        spaces: &'a HashMap<String, SpaceConfig>,
    ) -> SpaceResolver<'a> {
        SpaceResolver {
            override_space,
            default_space,
            spaces,
        }
    }

    #[test]
    fn spaceless_bookmark_records_launchservices_default_argv() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        let url = "https://example.com/page?utm_source=news&id=7";
        seed(&store, "a.md", url, "Page");

        let no_spaces = spaces(&[]);
        let launcher = RecordingLauncher::default();
        let opened = open(&store, url, &resolver(None, None, &no_spaces), &launcher).unwrap();

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

        let no_spaces = spaces(&[]);
        let launcher = RecordingLauncher::default();
        open(&store, url, &resolver(None, None, &no_spaces), &launcher).unwrap();

        assert_eq!(launcher.argvs.borrow()[0][1], url);
    }

    #[test]
    fn no_match_errors_and_launches_nothing() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        seed(&store, "a.md", "https://example.com/a", "A");

        let no_spaces = spaces(&[]);
        let launcher = RecordingLauncher::default();
        let err = open(
            &store,
            "https://example.com/nope",
            &resolver(None, None, &no_spaces),
            &launcher,
        )
        .unwrap_err();

        assert!(matches!(err, OpenError::NotFound(_)));
        assert!(launcher.argvs.borrow().is_empty(), "nothing launched");
    }

    #[test]
    fn near_duplicate_url_does_not_match() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        seed(&store, "a.md", "https://example.com/page", "Page");

        let no_spaces = spaces(&[]);
        let launcher = RecordingLauncher::default();
        for near in [
            "https://example.com/page/",
            "http://example.com/page",
            "https://example.com/page?utm_source=news",
        ] {
            let err = open(&store, near, &resolver(None, None, &no_spaces), &launcher).unwrap_err();
            assert!(matches!(err, OpenError::NotFound(_)), "{near}");
        }
        assert!(launcher.argvs.borrow().is_empty());
    }

    #[test]
    fn bookmark_space_without_profile_records_open_a_browser() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        let url = "https://work.example.com/x?ref=t";
        seed_with_space(&store, "a.md", url, "Work", "work");
        let spaces = spaces(&[("work", "Safari", None)]);

        let launcher = RecordingLauncher::default();
        open(&store, url, &resolver(None, None, &spaces), &launcher).unwrap();

        assert_eq!(
            *launcher.argvs.borrow(),
            vec![vec![
                "open".to_string(),
                "-a".to_string(),
                "Safari".to_string(),
                url.to_string(),
            ]]
        );
    }

    #[test]
    fn flag_overrides_bookmark_space() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        let url = "https://example.com/x";
        seed_with_space(&store, "a.md", url, "X", "work");
        let spaces = spaces(&[("work", "Safari", None), ("home", "Firefox", None)]);

        let launcher = RecordingLauncher::default();
        open(
            &store,
            url,
            &resolver(Some("home"), None, &spaces),
            &launcher,
        )
        .unwrap();

        assert_eq!(launcher.argvs.borrow()[0][2], "Firefox");
    }

    #[test]
    fn default_space_used_when_no_flag_and_no_bookmark_space() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        let url = "https://example.com/x";
        seed(&store, "a.md", url, "X");
        let spaces = spaces(&[("personal", "Safari", None)]);

        let launcher = RecordingLauncher::default();
        open(
            &store,
            url,
            &resolver(None, Some("personal"), &spaces),
            &launcher,
        )
        .unwrap();

        assert_eq!(launcher.argvs.borrow()[0][2], "Safari");
    }

    #[test]
    fn bookmark_space_beats_default_space() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        let url = "https://example.com/x";
        seed_with_space(&store, "a.md", url, "X", "work");
        let spaces = spaces(&[("work", "Safari", None), ("personal", "Firefox", None)]);

        let launcher = RecordingLauncher::default();
        open(
            &store,
            url,
            &resolver(None, Some("personal"), &spaces),
            &launcher,
        )
        .unwrap();

        assert_eq!(launcher.argvs.borrow()[0][2], "Safari");
    }

    #[test]
    fn override_beats_default_space() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        let url = "https://example.com/x";
        seed(&store, "a.md", url, "X");
        let spaces = spaces(&[("work", "Safari", None), ("personal", "Firefox", None)]);

        let launcher = RecordingLauncher::default();
        open(
            &store,
            url,
            &resolver(Some("work"), Some("personal"), &spaces),
            &launcher,
        )
        .unwrap();

        assert_eq!(launcher.argvs.borrow()[0][2], "Safari");
    }

    #[test]
    fn default_space_absent_from_config_errors_and_launches_nothing() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        let url = "https://example.com/x";
        seed(&store, "a.md", url, "X");
        let no_spaces = spaces(&[]);

        let launcher = RecordingLauncher::default();
        let err = open(
            &store,
            url,
            &resolver(None, Some("ghost"), &no_spaces),
            &launcher,
        )
        .unwrap_err();

        assert!(matches!(err, OpenError::UnknownSpace(name) if name == "ghost"));
        assert!(launcher.argvs.borrow().is_empty());
    }

    #[test]
    fn named_space_absent_from_config_errors_and_launches_nothing() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        let url = "https://example.com/x";
        seed_with_space(&store, "a.md", url, "X", "ghost");
        let no_spaces = spaces(&[]);

        let launcher = RecordingLauncher::default();
        let err = open(&store, url, &resolver(None, None, &no_spaces), &launcher).unwrap_err();

        assert!(matches!(err, OpenError::UnknownSpace(name) if name == "ghost"));
        assert!(launcher.argvs.borrow().is_empty());
    }

    #[test]
    fn flag_space_absent_from_config_errors_and_launches_nothing() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        let url = "https://example.com/x";
        seed(&store, "a.md", url, "X");
        let spaces = spaces(&[("work", "Safari", None)]);

        let launcher = RecordingLauncher::default();
        let err = open(
            &store,
            url,
            &resolver(Some("ghost"), None, &spaces),
            &launcher,
        )
        .unwrap_err();

        assert!(matches!(err, OpenError::UnknownSpace(name) if name == "ghost"));
        assert!(launcher.argvs.borrow().is_empty());
    }

    #[test]
    fn profiled_space_errors_and_launches_nothing() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        let url = "https://example.com/x";
        seed_with_space(&store, "a.md", url, "X", "work");
        let spaces = spaces(&[("work", "Google Chrome", Some("Work"))]);

        let launcher = RecordingLauncher::default();
        let err = open(&store, url, &resolver(None, None, &spaces), &launcher).unwrap_err();

        assert!(matches!(err, OpenError::ProfileUnsupported { .. }));
        assert!(launcher.argvs.borrow().is_empty());
    }
}
