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

fn read_only_file(store: &TempDir) -> String {
    let files = md_files(store);
    assert_eq!(files.len(), 1, "expected exactly one bookmark file");
    std::fs::read_to_string(&files[0]).unwrap()
}

#[test]
fn add_creates_store_dir_and_one_file() {
    let parent = TempDir::new().unwrap();
    let store_path = parent.path().join("does-not-exist-yet");
    let mut cmd = Command::cargo_bin("mdmarks").unwrap();
    cmd.env("MDMARKS_STORE", &store_path);
    cmd.args(["add", "https://example.com/a", "--title", "Example Page"]);
    cmd.assert().success();

    assert!(store_path.is_dir());
    let files: Vec<_> = std::fs::read_dir(&store_path)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
        .collect();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].file_name().unwrap(), "example-page.md");
}

#[test]
fn written_frontmatter_has_verbatim_url_title_added_and_empty_body() {
    let store = TempDir::new().unwrap();
    let url = "https://example.com/a?utm_source=news&id=7";
    mdmarks(&store)
        .args(["add", url, "--title", "My Title"])
        .assert()
        .success();

    let content = read_only_file(&store);
    assert!(content.starts_with("---\n"));
    assert!(
        content.contains(&format!("url: {url}")) || content.contains(&format!("url: \"{url}\""))
    );
    assert!(content.contains("title: My Title"));
    assert!(content.contains("added:"));
    assert!(!content.contains("tags"));
    assert!(!content.contains("description"));
    assert!(!content.contains("space"));

    let (_, body) = content.split_once("---\n\n").unwrap();
    assert_eq!(body, "");
}

#[test]
fn filename_is_slug_of_title() {
    let store = TempDir::new().unwrap();
    mdmarks(&store)
        .args(["add", "https://example.com/a", "--title", "Hello, World!"])
        .assert()
        .success();
    let files = md_files(&store);
    assert_eq!(files[0].file_name().unwrap(), "hello-world.md");
}

#[test]
fn two_urls_same_title_suffix_and_do_not_overwrite() {
    let store = TempDir::new().unwrap();
    for url in [
        "https://a.example.com/x",
        "https://b.example.com/y",
        "https://c.example.com/z",
    ] {
        mdmarks(&store)
            .args(["add", url, "--title", "Shared Title"])
            .assert()
            .success();
    }
    let files = md_files(&store);
    let names: Vec<_> = files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert_eq!(
        names,
        vec![
            "shared-title-2.md".to_string(),
            "shared-title-3.md".to_string(),
            "shared-title.md".to_string(),
        ]
    );

    let mut stored_urls: Vec<String> = files
        .iter()
        .map(|p| {
            let c = std::fs::read_to_string(p).unwrap();
            c.lines()
                .find(|l| l.starts_with("url:"))
                .unwrap()
                .to_string()
        })
        .collect();
    stored_urls.sort();
    assert_eq!(
        stored_urls,
        vec![
            "url: https://a.example.com/x".to_string(),
            "url: https://b.example.com/y".to_string(),
            "url: https://c.example.com/z".to_string(),
        ]
    );
}

#[test]
fn re_adding_exact_url_is_a_dedup_noop() {
    let store = TempDir::new().unwrap();
    let url = "https://example.com/page";
    mdmarks(&store)
        .args(["add", url, "--title", "Page"])
        .assert()
        .success();
    mdmarks(&store)
        .args(["add", url, "--title", "Page"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Already saved"));
    assert_eq!(md_files(&store).len(), 1);
}

#[test]
fn near_duplicate_variants_all_dedup() {
    let store = TempDir::new().unwrap();
    let base = "https://example.com/path?a=1&b=2";
    mdmarks(&store)
        .args(["add", base, "--title", "Base"])
        .assert()
        .success();

    let variants = [
        "https://example.com/path?a=1&b=2&utm_source=news",
        "http://example.com/path?a=1&b=2",
        "https://www.example.com/path?a=1&b=2",
        "https://EXAMPLE.com/path?a=1&b=2",
        "https://example.com/path?b=2&a=1",
        "https://example.com/path/?a=1&b=2",
        "https://example.com/path?a=1&b=2#section",
    ];
    for v in variants {
        mdmarks(&store)
            .args(["add", v, "--title", "Variant"])
            .assert()
            .success()
            .stdout(predicates::str::contains("Already saved"));
    }
    assert_eq!(md_files(&store).len(), 1);
}

#[test]
fn path_case_difference_creates_second_bookmark() {
    let store = TempDir::new().unwrap();
    mdmarks(&store)
        .args(["add", "https://example.com/Path", "--title", "Upper"])
        .assert()
        .success();
    mdmarks(&store)
        .args(["add", "https://example.com/path", "--title", "Lower"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Added"));
    assert_eq!(md_files(&store).len(), 2);
}

#[test]
fn invalid_url_is_rejected_with_nonzero_exit_and_no_file() {
    let store = TempDir::new().unwrap();
    mdmarks(&store)
        .args(["add", "not a url", "--title", "X"])
        .assert()
        .failure();
    assert_eq!(md_files(&store).len(), 0);
}

#[test]
fn non_http_scheme_is_rejected() {
    let store = TempDir::new().unwrap();
    mdmarks(&store)
        .args(["add", "ftp://example.com/a", "--title", "X"])
        .assert()
        .failure();
    assert_eq!(md_files(&store).len(), 0);
}

#[test]
fn dedup_noop_exits_zero() {
    let store = TempDir::new().unwrap();
    let url = "https://example.com/z";
    mdmarks(&store)
        .args(["add", url, "--title", "Z"])
        .assert()
        .success();
    mdmarks(&store)
        .args(["add", url, "--title", "Z"])
        .assert()
        .code(0);
}
