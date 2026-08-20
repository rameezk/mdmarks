use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use tempfile::TempDir;

fn mdmarks(store: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("mdmarks").unwrap();
    cmd.env("MDMARKS_STORE", store.path());
    cmd
}

fn seed(store: &TempDir, name: &str, contents: &str) {
    std::fs::write(store.path().join(name), contents).unwrap();
}

fn bookmark(url: &str, title: Option<&str>, added: Option<&str>) -> String {
    let mut fm = format!("url: {url}\n");
    if let Some(t) = title {
        fm.push_str(&format!("title: {t}\n"));
    }
    if let Some(a) = added {
        fm.push_str(&format!("added: {a}\n"));
    }
    format!("---\n{fm}---\n\n")
}

#[test]
fn lists_every_bookmark_sorted_by_added_descending() {
    let store = TempDir::new().unwrap();
    seed(
        &store,
        "alpha.md",
        &bookmark(
            "https://a.example.com/?utm_source=x",
            Some("Alpha"),
            Some("2026-01-01T00:00:00+00:00"),
        ),
    );
    seed(
        &store,
        "bravo.md",
        &bookmark(
            "https://b.example.com/y",
            Some("Bravo"),
            Some("2026-03-01T00:00:00+00:00"),
        ),
    );
    seed(
        &store,
        "charlie.md",
        &bookmark(
            "https://c.example.com/z",
            Some("Charlie"),
            Some("2026-02-01T00:00:00+00:00"),
        ),
    );

    let assert = mdmarks(&store).arg("list").assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    let titles: Vec<&str> = stdout.lines().collect();
    assert_eq!(titles.len(), 3);
    assert!(titles[0].contains("Bravo"), "newest first: {stdout}");
    assert!(titles[1].contains("Charlie"), "{stdout}");
    assert!(titles[2].contains("Alpha"), "{stdout}");
}

#[test]
fn shows_verbatim_url() {
    let store = TempDir::new().unwrap();
    let url = "https://example.com/path?utm_source=news&id=7";
    seed(
        &store,
        "one.md",
        &bookmark(url, Some("Kept"), Some("2026-01-01T00:00:00+00:00")),
    );

    mdmarks(&store)
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains(url));
}

#[test]
fn titleless_bookmark_renders_the_url_once() {
    let store = TempDir::new().unwrap();
    seed(
        &store,
        "bare.md",
        &bookmark(
            "https://bare.example.com/x",
            None,
            Some("2026-01-01T00:00:00+00:00"),
        ),
    );

    let assert = mdmarks(&store).arg("list").assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(
        stdout, "https://bare.example.com/x\n",
        "url once, not doubled"
    );
}

#[test]
fn empty_store_prints_nothing_and_exits_zero() {
    let store = TempDir::new().unwrap();
    mdmarks(&store).arg("list").assert().success().stdout("");
}

#[test]
fn nonexistent_store_prints_nothing_and_exits_zero() {
    let parent = TempDir::new().unwrap();
    let store_path = parent.path().join("does-not-exist");
    let mut cmd = Command::cargo_bin("mdmarks").unwrap();
    cmd.env("MDMARKS_STORE", &store_path);
    cmd.arg("list").assert().success().stdout("");
}

#[test]
fn malformed_file_is_skipped_and_the_rest_are_listed() {
    let store = TempDir::new().unwrap();
    seed(
        &store,
        "good.md",
        &bookmark(
            "https://good.example.com/",
            Some("Good"),
            Some("2026-01-01T00:00:00+00:00"),
        ),
    );
    seed(&store, "bad.md", "this file has no frontmatter at all\n");

    mdmarks(&store)
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("Good"))
        .stdout(predicates::str::contains("bad.example").not());
}

#[test]
fn bookmarks_without_added_sort_last() {
    let store = TempDir::new().unwrap();
    seed(
        &store,
        "dated.md",
        &bookmark(
            "https://dated.example.com/",
            Some("Dated"),
            Some("2020-01-01T00:00:00+00:00"),
        ),
    );
    seed(
        &store,
        "undated.md",
        &bookmark("https://undated.example.com/", Some("Undated"), None),
    );

    let assert = mdmarks(&store).arg("list").assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    assert!(lines[0].contains("Dated"), "dated first: {stdout}");
    assert!(lines[1].contains("Undated"), "undated last: {stdout}");
}

#[test]
fn output_is_deterministic_across_runs() {
    let store = TempDir::new().unwrap();
    for i in 0..12 {
        seed(
            &store,
            &format!("b{i}.md"),
            &bookmark(
                &format!("https://example.com/{i}"),
                Some(&format!("Title {i}")),
                Some("2026-01-01T00:00:00+00:00"),
            ),
        );
    }

    let first = mdmarks(&store).arg("list").assert().success();
    let first = first.get_output().stdout.clone();
    for _ in 0..5 {
        let again = mdmarks(&store).arg("list").assert().success();
        assert_eq!(again.get_output().stdout, first, "output must be stable");
    }
}
