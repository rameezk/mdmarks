use assert_cmd::Command;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

struct Harness {
    store: TempDir,
    bin_dir: TempDir,
    log: std::path::PathBuf,
}

fn harness() -> Harness {
    let store = TempDir::new().unwrap();
    let bin_dir = TempDir::new().unwrap();
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
        log,
    }
}

impl Harness {
    fn mdmarks(&self) -> Command {
        let mut cmd = Command::cargo_bin("mdmarks").unwrap();
        cmd.env("MDMARKS_STORE", self.store.path());
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
