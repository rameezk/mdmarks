use assert_cmd::Command;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

struct Harness {
    store: TempDir,
    bin_dir: TempDir,
    home: TempDir,
    log: std::path::PathBuf,
}

fn harness() -> Harness {
    let store = TempDir::new().unwrap();
    let bin_dir = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let log = bin_dir.path().join("open.log");
    let fake_open = bin_dir.path().join("open");
    std::fs::write(
        &fake_open,
        format!(
            "#!/bin/sh\n: > \"{log}\"\nfor a in \"$@\"; do printf '%s\\n' \"$a\" >> \"{log}\"; done\n",
            log = log.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&fake_open, std::fs::Permissions::from_mode(0o755)).unwrap();
    Harness {
        store,
        bin_dir,
        home,
        log,
    }
}

impl Harness {
    fn mdmarks(&self) -> Command {
        let mut cmd = Command::cargo_bin("mdmarks").unwrap();
        cmd.env("MDMARKS_STORE", self.store.path());
        cmd.env("HOME", self.home.path());
        let path = format!(
            "{}:{}",
            self.bin_dir.path().display(),
            std::env::var("PATH").unwrap_or_default()
        );
        cmd.env("PATH", path);
        cmd
    }

    fn add(&self, url: &str, title: &str) {
        self.mdmarks()
            .args(["add", url, "--title", title])
            .assert()
            .success();
    }

    fn seed(&self, name: &str, url: &str, title: &str, space: &str) {
        let content = format!("---\nurl: {url}\ntitle: {title}\nspace: {space}\n---\n\nnotes\n");
        std::fs::write(self.store.path().join(name), content).unwrap();
    }

    fn write_config(&self, toml: &str) {
        let dir = self.home.path().join(".config/mdmarks");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("config.toml"), toml).unwrap();
    }

    fn write_local_state(&self, support_dir: &str, contents: &str) {
        let dir = self
            .home
            .path()
            .join("Library/Application Support")
            .join(support_dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Local State"), contents).unwrap();
    }

    fn launched(&self) -> Vec<String> {
        match std::fs::read_to_string(&self.log) {
            Ok(s) => s.lines().map(str::to_string).collect(),
            Err(_) => Vec::new(),
        }
    }
}

#[test]
fn open_launches_the_verbatim_url_via_launchservices_default() {
    let h = harness();
    let url = "https://example.com/page?utm_source=news&id=7";
    h.add(url, "Page");

    h.mdmarks()
        .args(["open", url])
        .assert()
        .success()
        .stdout(predicates::str::contains("Page"));

    assert_eq!(h.launched(), vec![url.to_string()]);
}

#[test]
fn open_launches_url_verbatim_with_fragment_and_trackers_intact() {
    let h = harness();
    let url = "https://console.example.com/logs#/stream/abc?utm_campaign=x";
    h.add(url, "Console");

    h.mdmarks().args(["open", url]).assert().success();

    assert_eq!(h.launched(), vec![url.to_string()]);
}

#[test]
fn open_with_no_match_errors_and_launches_nothing() {
    let h = harness();
    h.add("https://example.com/a", "A");

    h.mdmarks()
        .args(["open", "https://example.com/nope"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("https://example.com/nope"));

    assert!(h.launched().is_empty(), "nothing should have launched");
}

#[test]
fn open_is_verbatim_not_normalized() {
    for near_duplicate in [
        "https://example.com/page/",
        "https://example.com/page?utm_source=news",
        "http://example.com/page",
    ] {
        let h = harness();
        h.add("https://example.com/page", "Page");

        h.mdmarks()
            .args(["open", near_duplicate])
            .assert()
            .failure();

        assert!(
            h.launched().is_empty(),
            "near-duplicate {near_duplicate} must not launch"
        );
    }
}

#[test]
fn open_on_empty_store_errors_and_launches_nothing() {
    let h = harness();

    h.mdmarks()
        .args(["open", "https://example.com/whatever"])
        .assert()
        .failure();

    assert!(h.launched().is_empty());
}

#[test]
fn open_space_without_profile_launches_open_a_browser() {
    let h = harness();
    let url = "https://work.example.com/x?utm_source=news";
    h.seed("work.md", url, "Work", "work");
    h.write_config("[spaces.work]\nbrowser = \"Google Chrome\"\n");

    h.mdmarks().args(["open", url]).assert().success();

    assert_eq!(
        h.launched(),
        vec![
            "-a".to_string(),
            "Google Chrome".to_string(),
            url.to_string()
        ]
    );
}

#[test]
fn open_flag_overrides_bookmark_space() {
    let h = harness();
    let url = "https://example.com/x";
    h.seed("x.md", url, "X", "work");
    h.write_config(
        "[spaces.work]\nbrowser = \"Google Chrome\"\n\n[spaces.home]\nbrowser = \"Firefox\"\n",
    );

    h.mdmarks()
        .args(["open", url, "--space", "home"])
        .assert()
        .success();

    assert_eq!(
        h.launched(),
        vec!["-a".to_string(), "Firefox".to_string(), url.to_string()]
    );
}

#[test]
fn open_uses_default_space_when_bookmark_has_none() {
    let h = harness();
    let url = "https://example.com/x";
    h.add(url, "X");
    h.write_config("default_space = \"personal\"\n\n[spaces.personal]\nbrowser = \"Safari\"\n");

    h.mdmarks().args(["open", url]).assert().success();

    assert_eq!(
        h.launched(),
        vec!["-a".to_string(), "Safari".to_string(), url.to_string()]
    );
}

#[test]
fn open_unknown_space_errors_and_launches_nothing() {
    let h = harness();
    let url = "https://example.com/x";
    h.seed("x.md", url, "X", "ghost");
    h.write_config("[spaces.work]\nbrowser = \"Google Chrome\"\n");

    h.mdmarks()
        .args(["open", url])
        .assert()
        .failure()
        .stderr(predicates::str::contains("ghost"));

    assert!(h.launched().is_empty());
}

#[test]
fn open_flag_space_absent_from_config_errors_and_launches_nothing() {
    let h = harness();
    let url = "https://example.com/x";
    h.add(url, "X");
    h.write_config("[spaces.work]\nbrowser = \"Google Chrome\"\n");

    h.mdmarks()
        .args(["open", url, "--space", "ghost"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("ghost"));

    assert!(h.launched().is_empty());
}

const LOCAL_STATE: &str =
    r#"{"profile":{"info_cache":{"Default":{"name":"Personal"},"Profile 1":{"name":"Work"}}}}"#;

#[test]
fn open_chrome_profile_launches_profile_directory_argv() {
    let h = harness();
    let url = "https://work.example.com/x?utm_source=news#frag";
    h.seed("x.md", url, "X", "work");
    h.write_config("[spaces.work]\nbrowser = \"Google Chrome\"\nprofile = \"Work\"\n");
    h.write_local_state("Google/Chrome", LOCAL_STATE);

    h.mdmarks().args(["open", url]).assert().success();

    assert_eq!(
        h.launched(),
        vec![
            "-na".to_string(),
            "Google Chrome".to_string(),
            "--args".to_string(),
            "--profile-directory=Profile 1".to_string(),
            url.to_string(),
        ]
    );
}

#[test]
fn open_chromium_fork_profile_uses_configured_support_dir() {
    let h = harness();
    let url = "https://example.com/x";
    h.seed("x.md", url, "X", "work");
    h.write_config(
        "[spaces.work]\nbrowser = \"Helium\"\nprofile = \"Work\"\nchromium_support_dir = \"net.imput.helium\"\n",
    );
    h.write_local_state("net.imput.helium", LOCAL_STATE);

    h.mdmarks().args(["open", url]).assert().success();

    assert_eq!(
        h.launched(),
        vec![
            "-na".to_string(),
            "Helium".to_string(),
            "--args".to_string(),
            "--profile-directory=Profile 1".to_string(),
            url.to_string(),
        ]
    );
}

#[test]
fn open_profile_absent_from_local_state_errors_and_launches_nothing() {
    let h = harness();
    let url = "https://example.com/x";
    h.seed("x.md", url, "X", "work");
    h.write_config("[spaces.work]\nbrowser = \"Google Chrome\"\nprofile = \"Ghost\"\n");
    h.write_local_state("Google/Chrome", LOCAL_STATE);

    h.mdmarks()
        .args(["open", url])
        .assert()
        .failure()
        .stderr(predicates::str::contains("Ghost"));

    assert!(h.launched().is_empty());
}

#[test]
fn open_profile_with_absent_local_state_errors_and_launches_nothing() {
    let h = harness();
    let url = "https://example.com/x";
    h.seed("x.md", url, "X", "work");
    h.write_config("[spaces.work]\nbrowser = \"Google Chrome\"\nprofile = \"Work\"\n");

    h.mdmarks().args(["open", url]).assert().failure();

    assert!(h.launched().is_empty());
}

#[test]
fn open_profile_with_malformed_local_state_errors_and_launches_nothing() {
    let h = harness();
    let url = "https://example.com/x";
    h.seed("x.md", url, "X", "work");
    h.write_config("[spaces.work]\nbrowser = \"Google Chrome\"\nprofile = \"Work\"\n");
    h.write_local_state("Google/Chrome", "{ not valid json");

    h.mdmarks().args(["open", url]).assert().failure();

    assert!(h.launched().is_empty());
}
