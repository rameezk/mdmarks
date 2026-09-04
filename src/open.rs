use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::SpaceConfig;
use crate::select::by_exact_url;
use crate::store::{Store, StoreError, StoredBookmark};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSpec {
    pub browser: Option<String>,
    pub profile_dir: Option<String>,
    pub url: String,
}

pub struct SpaceResolver<'a> {
    pub override_space: Option<&'a str>,
    pub default_space: Option<&'a str>,
    pub spaces: &'a HashMap<String, SpaceConfig>,
    pub app_support: &'a Path,
}

pub fn build_argv(spec: &LaunchSpec) -> Vec<String> {
    match (&spec.browser, &spec.profile_dir) {
        (None, _) => vec!["open".to_string(), spec.url.clone()],
        (Some(browser), None) => vec![
            "open".to_string(),
            "-a".to_string(),
            browser.clone(),
            spec.url.clone(),
        ],
        (Some(browser), Some(dir)) => vec![
            "open".to_string(),
            "-na".to_string(),
            browser.clone(),
            "--args".to_string(),
            format!("--profile-directory={dir}"),
            spec.url.clone(),
        ],
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProfileResolveError {
    LocalStateAbsent(PathBuf),
    LocalStateUnreadable(PathBuf),
    LocalStateMalformed(PathBuf),
    NoProfileNamed { display_name: String, path: PathBuf },
    AmbiguousProfileName { display_name: String, path: PathBuf },
}

impl std::fmt::Display for ProfileResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfileResolveError::LocalStateAbsent(path) => {
                write!(f, "browser profile state not found at {}", path.display())
            }
            ProfileResolveError::LocalStateUnreadable(path) => {
                write!(
                    f,
                    "browser profile state at {} is unreadable",
                    path.display()
                )
            }
            ProfileResolveError::LocalStateMalformed(path) => {
                write!(
                    f,
                    "browser profile state at {} is malformed",
                    path.display()
                )
            }
            ProfileResolveError::NoProfileNamed { display_name, path } => write!(
                f,
                "no browser profile named \"{display_name}\" in {}",
                path.display()
            ),
            ProfileResolveError::AmbiguousProfileName { display_name, path } => write!(
                f,
                "more than one browser profile named \"{display_name}\" in {}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ProfileResolveError {}

pub fn default_support_dir(browser: &str) -> Option<&'static str> {
    match browser {
        "Google Chrome" => Some("Google/Chrome"),
        _ => None,
    }
}

pub fn resolve_profile_dir(
    local_state_path: &Path,
    display_name: &str,
) -> Result<String, ProfileResolveError> {
    let contents = match std::fs::read_to_string(local_state_path) {
        Ok(contents) => contents,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(ProfileResolveError::LocalStateAbsent(
                local_state_path.to_path_buf(),
            ))
        }
        Err(_) => {
            return Err(ProfileResolveError::LocalStateUnreadable(
                local_state_path.to_path_buf(),
            ))
        }
    };

    let value: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|_| ProfileResolveError::LocalStateMalformed(local_state_path.to_path_buf()))?;

    let info_cache = value
        .get("profile")
        .and_then(|p| p.get("info_cache"))
        .and_then(|c| c.as_object())
        .ok_or_else(|| ProfileResolveError::LocalStateMalformed(local_state_path.to_path_buf()))?;

    let mut matches = info_cache
        .iter()
        .filter(|(_, entry)| entry.get("name").and_then(|n| n.as_str()) == Some(display_name))
        .map(|(dir, _)| dir.clone());

    let first = matches
        .next()
        .ok_or_else(|| ProfileResolveError::NoProfileNamed {
            display_name: display_name.to_string(),
            path: local_state_path.to_path_buf(),
        })?;

    if matches.next().is_some() {
        return Err(ProfileResolveError::AmbiguousProfileName {
            display_name: display_name.to_string(),
            path: local_state_path.to_path_buf(),
        });
    }

    Ok(first)
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
    ProfileUnsupported {
        space: String,
        browser: String,
    },
    ProfileResolve {
        space: String,
        source: ProfileResolveError,
    },
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
            OpenError::ProfileResolve { space, source } => {
                write!(f, "space \"{space}\": {source}")
            }
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
                profile_dir: None,
                url,
            })
        }
        Some(name) => name,
    };

    let space = resolver
        .spaces
        .get(name)
        .ok_or_else(|| OpenError::UnknownSpace(name.to_string()))?;

    let profile_dir = match &space.profile {
        None => None,
        Some(display_name) => {
            let support_dir = space
                .chromium_support_dir
                .as_deref()
                .or_else(|| default_support_dir(&space.browser))
                .ok_or_else(|| OpenError::ProfileUnsupported {
                    space: name.to_string(),
                    browser: space.browser.clone(),
                })?;
            let local_state = resolver.app_support.join(support_dir).join("Local State");
            let dir = resolve_profile_dir(&local_state, display_name).map_err(|source| {
                OpenError::ProfileResolve {
                    space: name.to_string(),
                    source,
                }
            })?;
            Some(dir)
        }
    };

    Ok(LaunchSpec {
        browser: Some(space.browser.clone()),
        profile_dir,
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
                        chromium_support_dir: None,
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
            app_support: Path::new("/nonexistent"),
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

    const LOCAL_STATE: &str = r#"{
        "profile": {
            "info_cache": {
                "Default": { "name": "Personal" },
                "Profile 1": { "name": "Work" }
            }
        }
    }"#;

    fn write_local_state(app_support: &Path, support_dir: &str, contents: &str) {
        let dir = app_support.join(support_dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Local State"), contents).unwrap();
    }

    fn resolver_with_support<'a>(
        spaces: &'a HashMap<String, SpaceConfig>,
        app_support: &'a Path,
    ) -> SpaceResolver<'a> {
        SpaceResolver {
            override_space: None,
            default_space: None,
            spaces,
            app_support,
        }
    }

    fn chromium_space(
        name: &str,
        browser: &str,
        profile: &str,
        support_dir: Option<&str>,
    ) -> HashMap<String, SpaceConfig> {
        let mut map = HashMap::new();
        map.insert(
            name.to_string(),
            SpaceConfig {
                browser: browser.to_string(),
                profile: Some(profile.to_string()),
                chromium_support_dir: support_dir.map(str::to_string),
            },
        );
        map
    }

    #[test]
    fn chrome_profile_records_profile_directory_argv_with_verbatim_url() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        let url = "https://work.example.com/x?utm_source=news#frag";
        seed_with_space(&store, "a.md", url, "X", "work");

        let app_support = TempDir::new().unwrap();
        write_local_state(app_support.path(), "Google/Chrome", LOCAL_STATE);
        let spaces = chromium_space("work", "Google Chrome", "Work", None);

        let launcher = RecordingLauncher::default();
        open(
            &store,
            url,
            &resolver_with_support(&spaces, app_support.path()),
            &launcher,
        )
        .unwrap();

        assert_eq!(
            *launcher.argvs.borrow(),
            vec![vec![
                "open".to_string(),
                "-na".to_string(),
                "Google Chrome".to_string(),
                "--args".to_string(),
                "--profile-directory=Profile 1".to_string(),
                url.to_string(),
            ]]
        );
    }

    #[test]
    fn chromium_fork_uses_configured_support_dir() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        let url = "https://example.com/x";
        seed_with_space(&store, "a.md", url, "X", "work");

        let app_support = TempDir::new().unwrap();
        write_local_state(app_support.path(), "net.imput.helium", LOCAL_STATE);
        let spaces = chromium_space("work", "Helium", "Work", Some("net.imput.helium"));

        let launcher = RecordingLauncher::default();
        open(
            &store,
            url,
            &resolver_with_support(&spaces, app_support.path()),
            &launcher,
        )
        .unwrap();

        assert_eq!(
            *launcher.argvs.borrow(),
            vec![vec![
                "open".to_string(),
                "-na".to_string(),
                "Helium".to_string(),
                "--args".to_string(),
                "--profile-directory=Profile 1".to_string(),
                url.to_string(),
            ]]
        );
    }

    #[test]
    fn profile_on_unknown_chromium_browser_errors_and_launches_nothing() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        let url = "https://example.com/x";
        seed_with_space(&store, "a.md", url, "X", "work");

        let app_support = TempDir::new().unwrap();
        let spaces = chromium_space("work", "Firefox", "Work", None);

        let launcher = RecordingLauncher::default();
        let err = open(
            &store,
            url,
            &resolver_with_support(&spaces, app_support.path()),
            &launcher,
        )
        .unwrap_err();

        assert!(matches!(err, OpenError::ProfileUnsupported { .. }));
        assert!(launcher.argvs.borrow().is_empty());
    }

    #[test]
    fn absent_profile_errors_and_launches_nothing() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        let url = "https://example.com/x";
        seed_with_space(&store, "a.md", url, "X", "work");

        let app_support = TempDir::new().unwrap();
        write_local_state(app_support.path(), "Google/Chrome", LOCAL_STATE);
        let spaces = chromium_space("work", "Google Chrome", "Nonexistent", None);

        let launcher = RecordingLauncher::default();
        let err = open(
            &store,
            url,
            &resolver_with_support(&spaces, app_support.path()),
            &launcher,
        )
        .unwrap_err();

        assert!(matches!(
            err,
            OpenError::ProfileResolve {
                source: ProfileResolveError::NoProfileNamed { .. },
                ..
            }
        ));
        assert!(launcher.argvs.borrow().is_empty());
    }

    #[test]
    fn missing_local_state_errors_and_launches_nothing() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        let url = "https://example.com/x";
        seed_with_space(&store, "a.md", url, "X", "work");

        let app_support = TempDir::new().unwrap();
        let spaces = chromium_space("work", "Google Chrome", "Work", None);

        let launcher = RecordingLauncher::default();
        let err = open(
            &store,
            url,
            &resolver_with_support(&spaces, app_support.path()),
            &launcher,
        )
        .unwrap_err();

        assert!(matches!(
            err,
            OpenError::ProfileResolve {
                source: ProfileResolveError::LocalStateAbsent(_),
                ..
            }
        ));
        assert!(launcher.argvs.borrow().is_empty());
    }

    #[test]
    fn resolve_profile_dir_table() {
        let app_support = TempDir::new().unwrap();
        let present = app_support.path().join("present");
        std::fs::create_dir_all(&present).unwrap();
        let present_path = present.join("Local State");
        std::fs::write(&present_path, LOCAL_STATE).unwrap();

        let malformed = app_support.path().join("malformed");
        std::fs::create_dir_all(&malformed).unwrap();
        let malformed_path = malformed.join("Local State");
        std::fs::write(&malformed_path, "{ not valid json").unwrap();

        let absent_path = app_support.path().join("absent").join("Local State");

        let duplicate = app_support.path().join("duplicate");
        std::fs::create_dir_all(&duplicate).unwrap();
        let duplicate_path = duplicate.join("Local State");
        std::fs::write(
            &duplicate_path,
            r#"{"profile":{"info_cache":{"Default":{"name":"Work"},"Profile 1":{"name":"Work"}}}}"#,
        )
        .unwrap();

        let unreadable_dir = app_support.path().join("unreadable");
        std::fs::create_dir_all(&unreadable_dir).unwrap();

        assert_eq!(
            resolve_profile_dir(&present_path, "Work"),
            Ok("Profile 1".to_string())
        );
        assert_eq!(
            resolve_profile_dir(&present_path, "Personal"),
            Ok("Default".to_string())
        );
        assert!(matches!(
            resolve_profile_dir(&present_path, "Ghost"),
            Err(ProfileResolveError::NoProfileNamed { .. })
        ));
        assert!(matches!(
            resolve_profile_dir(&duplicate_path, "Work"),
            Err(ProfileResolveError::AmbiguousProfileName { .. })
        ));
        assert!(matches!(
            resolve_profile_dir(&malformed_path, "Work"),
            Err(ProfileResolveError::LocalStateMalformed(_))
        ));
        assert!(matches!(
            resolve_profile_dir(&absent_path, "Work"),
            Err(ProfileResolveError::LocalStateAbsent(_))
        ));
        assert!(matches!(
            resolve_profile_dir(&unreadable_dir, "Work"),
            Err(ProfileResolveError::LocalStateUnreadable(_))
        ));
    }
}
