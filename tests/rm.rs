use assert_cmd::Command;
use tempfile::TempDir;

fn mdmarks(store: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("mdmarks").unwrap();
    cmd.env("MDMARKS_STORE", store.path());
    cmd
}

fn md_files(store: &TempDir) -> Vec<std::path::PathBuf> {
    let mut files: Vec<_> = std::fs::read_dir(store.path())
        .map(|rd| {
            rd.filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

fn add(store: &TempDir, url: &str, title: &str) {
    mdmarks(store)
        .args(["add", url, "--title", title])
        .assert()
        .success();
}

#[test]
fn rm_deletes_the_matching_bookmark_and_reports_it() {
    let store = TempDir::new().unwrap();
    let url = "https://example.com/keep-me?utm_source=news&id=7";
    add(&store, url, "Keep Me");
    assert_eq!(md_files(&store).len(), 1);

    mdmarks(&store)
        .args(["rm", url])
        .assert()
        .success()
        .stdout(predicates::str::contains(url))
        .stdout(predicates::str::contains("Keep Me"));

    assert!(md_files(&store).is_empty(), "file should be deleted");
}

#[test]
fn rm_is_verbatim_not_normalized() {
    for near_duplicate in [
        "https://example.com/page/",
        "https://example.com/page?utm_source=news",
        "http://example.com/page",
    ] {
        let store = TempDir::new().unwrap();
        add(&store, "https://example.com/page", "Page");

        mdmarks(&store)
            .args(["rm", near_duplicate])
            .assert()
            .failure();

        assert_eq!(
            md_files(&store).len(),
            1,
            "near-duplicate {near_duplicate} must not match"
        );
    }
}

#[test]
fn rm_with_no_match_errors_and_deletes_nothing() {
    let store = TempDir::new().unwrap();
    add(&store, "https://example.com/a", "A");

    mdmarks(&store)
        .args(["rm", "https://example.com/nope"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("https://example.com/nope"));

    assert_eq!(md_files(&store).len(), 1);
}

#[test]
fn rm_on_empty_store_errors() {
    let store = TempDir::new().unwrap();
    mdmarks(&store)
        .args(["rm", "https://example.com/whatever"])
        .assert()
        .failure();
}

#[test]
fn rm_leaves_every_other_bookmark_untouched() {
    let store = TempDir::new().unwrap();
    add(&store, "https://example.com/one", "One");
    add(&store, "https://example.com/two", "Two");
    let before: Vec<_> = md_files(&store)
        .into_iter()
        .map(|p| (p.clone(), std::fs::read_to_string(&p).unwrap()))
        .collect();

    mdmarks(&store)
        .args(["rm", "https://example.com/one"])
        .assert()
        .success();

    let survivor = before
        .iter()
        .find(|(_, content)| content.contains("https://example.com/two"))
        .expect("two should be among the originals");
    assert!(survivor.0.exists(), "untouched bookmark must remain");
    assert_eq!(
        std::fs::read_to_string(&survivor.0).unwrap(),
        survivor.1,
        "untouched bookmark content must be identical"
    );
    assert_eq!(md_files(&store).len(), 1);
}
