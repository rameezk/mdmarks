use assert_cmd::Command;
use tempfile::TempDir;

fn mdmarks(store: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("mdmarks").unwrap();
    cmd.env("MDMARKS_STORE", store.path());
    cmd
}

fn seed(store: &TempDir, name: &str, contents: &str) {
    std::fs::write(store.path().join(name), contents).unwrap();
}

fn bookmark(url: &str, title: &str, added: &str, space: Option<&str>) -> String {
    let mut fm = format!("url: {url}\ntitle: {title}\nadded: {added}\n");
    if let Some(s) = space {
        fm.push_str(&format!("space: {s}\n"));
    }
    format!("---\n{fm}---\n\n")
}

fn stdout_lines(assert: &assert_cmd::assert::Assert) -> Vec<String> {
    String::from_utf8(assert.get_output().stdout.clone())
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

fn corpus(store: &TempDir) {
    seed(
        store,
        "work-one.md",
        &bookmark(
            "https://work.example.com/rust",
            "Rust at Work",
            "2026-01-01T00:00:00+00:00",
            Some("work"),
        ),
    );
    seed(
        store,
        "work-two.md",
        &bookmark(
            "https://work.example.com/kube",
            "Kubernetes Guide",
            "2026-03-01T00:00:00+00:00",
            Some("work"),
        ),
    );
    seed(
        store,
        "home-one.md",
        &bookmark(
            "https://home.example.com/rust",
            "Rust at Home",
            "2026-02-01T00:00:00+00:00",
            Some("home"),
        ),
    );
    seed(
        store,
        "spaceless.md",
        &bookmark(
            "https://none.example.com/rust",
            "Rust Nowhere",
            "2026-04-01T00:00:00+00:00",
            None,
        ),
    );
}

#[test]
fn list_space_returns_only_that_space_added_descending() {
    let store = TempDir::new().unwrap();
    corpus(&store);

    let assert = mdmarks(&store)
        .args(["list", "--space", "work"])
        .assert()
        .success();
    let lines = stdout_lines(&assert);

    assert_eq!(lines.len(), 2, "only work bookmarks: {lines:?}");
    assert!(
        lines[0].contains("Kubernetes Guide"),
        "newest first: {lines:?}"
    );
    assert!(lines[1].contains("Rust at Work"), "{lines:?}");
}

#[test]
fn list_space_excludes_unset_space() {
    let store = TempDir::new().unwrap();
    corpus(&store);

    let assert = mdmarks(&store)
        .args(["list", "--space", "work"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(
        !stdout.contains("Rust Nowhere"),
        "unset space excluded: {stdout}"
    );
    assert!(
        !stdout.contains("Rust at Home"),
        "other space excluded: {stdout}"
    );
}

#[test]
fn list_space_is_exact_not_fuzzy() {
    let store = TempDir::new().unwrap();
    seed(
        &store,
        "a.md",
        &bookmark(
            "https://a.example.com/",
            "Exact",
            "2026-01-01T00:00:00+00:00",
            Some("work"),
        ),
    );
    seed(
        &store,
        "b.md",
        &bookmark(
            "https://b.example.com/",
            "Prefixed",
            "2026-02-01T00:00:00+00:00",
            Some("work-stuff"),
        ),
    );

    let assert = mdmarks(&store)
        .args(["list", "--space", "work"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("Exact"), "{stdout}");
    assert!(
        !stdout.contains("Prefixed"),
        "no fuzzy match on space: {stdout}"
    );
}

#[test]
fn search_space_with_query_applies_both_filter_and_fuzzy_match() {
    let store = TempDir::new().unwrap();
    corpus(&store);

    let assert = mdmarks(&store)
        .args(["search", "rust", "--space", "work"])
        .assert()
        .success();
    let lines = stdout_lines(&assert);

    assert_eq!(lines.len(), 1, "only work + rust match: {lines:?}");
    assert!(lines[0].contains("Rust at Work"), "{lines:?}");
}

#[test]
fn search_space_with_empty_query_returns_that_space_added_descending() {
    let store = TempDir::new().unwrap();
    corpus(&store);

    let assert = mdmarks(&store)
        .args(["search", "", "--space", "work"])
        .assert()
        .success();
    let lines = stdout_lines(&assert);

    assert_eq!(lines.len(), 2, "the space's feed: {lines:?}");
    assert!(
        lines[0].contains("Kubernetes Guide"),
        "newest first: {lines:?}"
    );
    assert!(lines[1].contains("Rust at Work"), "{lines:?}");
}

#[test]
fn search_space_excludes_unset_space() {
    let store = TempDir::new().unwrap();
    corpus(&store);

    let assert = mdmarks(&store)
        .args(["search", "rust", "--space", "home"])
        .assert()
        .success();
    let lines = stdout_lines(&assert);

    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("Rust at Home"), "{lines:?}");
}

#[test]
fn list_space_composes_with_json() {
    let store = TempDir::new().unwrap();
    corpus(&store);

    let assert = mdmarks(&store)
        .args(["list", "--space", "work", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let records: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let records = records.as_array().unwrap();

    let titles: Vec<&str> = records
        .iter()
        .map(|r| r["title"].as_str().unwrap())
        .collect();
    assert_eq!(titles, vec!["Kubernetes Guide", "Rust at Work"]);
    for r in records {
        assert_eq!(r["space"], "work");
    }
}

#[test]
fn search_space_composes_with_json() {
    let store = TempDir::new().unwrap();
    corpus(&store);

    let assert = mdmarks(&store)
        .args(["search", "rust", "--space", "work", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let records: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let records = records.as_array().unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["title"], "Rust at Work");
    assert_eq!(records[0]["space"], "work");
}

#[test]
fn space_with_no_members_returns_nothing_and_exits_zero() {
    let store = TempDir::new().unwrap();
    corpus(&store);

    mdmarks(&store)
        .args(["list", "--space", "nonexistent"])
        .assert()
        .success()
        .stdout("");
    mdmarks(&store)
        .args(["search", "rust", "--space", "nonexistent"])
        .assert()
        .success()
        .stdout("");
}
