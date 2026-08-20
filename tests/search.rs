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

fn bookmark(
    url: &str,
    title: Option<&str>,
    added: Option<&str>,
    tags: &[&str],
    body: &str,
) -> String {
    let mut fm = format!("url: {url}\n");
    if let Some(t) = title {
        fm.push_str(&format!("title: {t}\n"));
    }
    if let Some(a) = added {
        fm.push_str(&format!("added: {a}\n"));
    }
    if !tags.is_empty() {
        fm.push_str("tags:\n");
        for tag in tags {
            fm.push_str(&format!("  - {tag}\n"));
        }
    }
    format!("---\n{fm}---\n\n{body}\n")
}

fn stdout_lines(assert: &assert_cmd::assert::Assert) -> Vec<String> {
    String::from_utf8(assert.get_output().stdout.clone())
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn ranks_matching_bookmarks_and_excludes_the_rest() {
    let store = TempDir::new().unwrap();
    seed(
        &store,
        "rustbook.md",
        &bookmark(
            "https://doc.rust-lang.org/book/",
            Some("The Rust Programming Language"),
            Some("2026-01-01T00:00:00+00:00"),
            &["rust", "lang"],
            "",
        ),
    );
    seed(
        &store,
        "trust.md",
        &bookmark(
            "https://example.com/trust",
            Some("Building Trust"),
            Some("2026-02-01T00:00:00+00:00"),
            &["management"],
            "",
        ),
    );
    seed(
        &store,
        "python.md",
        &bookmark(
            "https://python.org",
            Some("Python Docs"),
            Some("2026-03-01T00:00:00+00:00"),
            &["python"],
            "",
        ),
    );

    let assert = mdmarks(&store).args(["search", "rust"]).assert().success();
    let lines = stdout_lines(&assert);
    assert_eq!(lines.len(), 2, "only matches: {lines:?}");
    assert!(
        lines[0].contains("Rust Programming"),
        "strongest first: {lines:?}"
    );
    assert!(lines[1].contains("Building Trust"), "{lines:?}");
}

#[test]
fn body_never_contributes_to_a_match() {
    let store = TempDir::new().unwrap();
    seed(
        &store,
        "one.md",
        &bookmark(
            "https://example.com/a",
            Some("Nothing Relevant"),
            Some("2026-01-01T00:00:00+00:00"),
            &["misc"],
            "this note body mentions zebracorn prominently",
        ),
    );

    mdmarks(&store)
        .args(["search", "zebracorn"])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn empty_query_returns_all_added_descending() {
    let store = TempDir::new().unwrap();
    seed(
        &store,
        "old.md",
        &bookmark(
            "https://a",
            Some("Old"),
            Some("2020-01-01T00:00:00+00:00"),
            &[],
            "",
        ),
    );
    seed(
        &store,
        "new.md",
        &bookmark(
            "https://b",
            Some("New"),
            Some("2026-01-01T00:00:00+00:00"),
            &[],
            "",
        ),
    );

    let assert = mdmarks(&store).args(["search", ""]).assert().success();
    let lines = stdout_lines(&assert);
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[0].contains("New"), "newest first: {lines:?}");
    assert!(lines[1].contains("Old"), "{lines:?}");
}

#[test]
fn no_match_query_prints_nothing_and_exits_zero() {
    let store = TempDir::new().unwrap();
    seed(
        &store,
        "one.md",
        &bookmark(
            "https://a",
            Some("Alpha"),
            Some("2026-01-01T00:00:00+00:00"),
            &[],
            "",
        ),
    );

    mdmarks(&store)
        .args(["search", "qqzzxx"])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn tags_are_matched() {
    let store = TempDir::new().unwrap();
    seed(
        &store,
        "tagged.md",
        &bookmark(
            "https://example.com/x",
            Some("Some Title"),
            Some("2026-01-01T00:00:00+00:00"),
            &["kubernetes"],
            "",
        ),
    );

    mdmarks(&store)
        .args(["search", "kube"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Some Title"));
}
