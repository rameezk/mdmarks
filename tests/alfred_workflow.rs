use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;

fn script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("alfred/workflow/script_filter.sh")
}

fn binary() -> PathBuf {
    assert_cmd::cargo::cargo_bin("mdmarks")
}

fn seed(store: &TempDir, name: &str, url: &str, title: &str, added: &str) {
    let contents = format!("---\nurl: {url}\ntitle: {title}\nadded: {added}\n---\n\n");
    std::fs::write(store.path().join(name), contents).unwrap();
}

fn seeded_store(home: &TempDir) -> TempDir {
    let store = TempDir::new_in(home.path()).unwrap();
    seed(
        &store,
        "rust.md",
        "https://doc.rust-lang.org/book/",
        "The Rust Programming Language",
        "2026-01-01T00:00:00+00:00",
    );
    seed(
        &store,
        "python.md",
        "https://python.org",
        "Python Docs",
        "2026-02-01T00:00:00+00:00",
    );
    store
}

fn direct(home: &TempDir, store: &TempDir, query: &str) -> String {
    let out = Command::new(binary())
        .args(["search", query, "--format", "alfred"])
        .env("HOME", home.path())
        .env("MDMARKS_STORE", store.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "direct run failed: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

fn via_script(
    home: &TempDir,
    store: &TempDir,
    query: &str,
    path: &str,
    bin: Option<&PathBuf>,
) -> String {
    let mut cmd = Command::new("bash");
    cmd.arg(script_path())
        .arg(query)
        .env("HOME", home.path())
        .env("MDMARKS_STORE", store.path())
        .env("PATH", path);
    match bin {
        Some(bin) => {
            cmd.env("MDMARKS_BIN", bin);
        }
        None => {
            cmd.env_remove("MDMARKS_BIN");
        }
    }
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "script run failed: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn script_matches_the_cli_alfred_feed_for_a_query() {
    let home = TempDir::new().unwrap();
    let store = seeded_store(&home);
    let bin = binary();
    let path = std::env::var("PATH").unwrap();

    let expected = direct(&home, &store, "rust");
    let actual = via_script(&home, &store, "rust", &path, Some(&bin));

    assert_eq!(actual, expected);
}

#[test]
fn script_matches_the_cli_alfred_feed_for_the_empty_feed() {
    let home = TempDir::new().unwrap();
    let store = seeded_store(&home);
    let bin = binary();
    let path = std::env::var("PATH").unwrap();

    let expected = direct(&home, &store, "");
    let actual = via_script(&home, &store, "", &path, Some(&bin));

    assert_eq!(actual, expected);
    assert!(actual.contains("Rust Programming"), "full feed: {actual}");
    assert!(actual.contains("Python Docs"), "full feed: {actual}");
}

#[test]
fn binary_defaults_to_mdmarks_on_path_when_unset() {
    let home = TempDir::new().unwrap();
    let store = seeded_store(&home);
    let bin = binary();
    let bin_dir = bin.parent().unwrap().to_str().unwrap().to_string();
    let path = format!("{bin_dir}:{}", std::env::var("PATH").unwrap());

    let expected = direct(&home, &store, "rust");
    let actual = via_script(&home, &store, "rust", &path, None);

    assert_eq!(actual, expected);
}
