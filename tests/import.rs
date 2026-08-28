use assert_cmd::Command;
use tempfile::TempDir;

const FIXTURE: &str = include_str!("fixtures/flat-export.html");
const NESTED: &str = include_str!("fixtures/nested-export.html");
const DATES_TITLES: &str = include_str!("fixtures/import-dates-titles.html");

fn write_fixture(dir: &TempDir) -> std::path::PathBuf {
    write_export(dir, FIXTURE)
}

fn write_export(dir: &TempDir, html: &str) -> std::path::PathBuf {
    let path = dir.path().join("export.html");
    std::fs::write(&path, html).unwrap();
    path
}

fn md_files(store_path: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files: Vec<_> = std::fs::read_dir(store_path)
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

fn read(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn imports_one_file_per_link_and_creates_store() {
    let work = TempDir::new().unwrap();
    let export = write_fixture(&work);
    let store_path = work.path().join("store-does-not-exist");

    let mut cmd = Command::cargo_bin("mdmarks").unwrap();
    cmd.env("MDMARKS_STORE", &store_path)
        .args(["import"])
        .arg(&export)
        .assert()
        .success()
        .stdout(predicates::str::contains("Imported 3 bookmarks"));

    assert!(store_path.is_dir());
    assert_eq!(md_files(&store_path).len(), 3);
}

#[test]
fn writes_verbatim_url_and_link_text_title() {
    let work = TempDir::new().unwrap();
    let export = write_fixture(&work);
    let store_path = work.path().join("store");

    Command::cargo_bin("mdmarks")
        .unwrap()
        .env("MDMARKS_STORE", &store_path)
        .args(["import"])
        .arg(&export)
        .assert()
        .success();

    let bodies: Vec<String> = md_files(&store_path).iter().map(|p| read(p)).collect();
    let joined = bodies.join("\n");
    assert!(joined.contains("url: https://example.com/a?utm_source=news&id=7"));
    assert!(joined.contains("title: The Rust Programming Language"));
}

#[test]
fn imported_bookmarks_have_no_space_tags_or_description() {
    let work = TempDir::new().unwrap();
    let export = write_fixture(&work);
    let store_path = work.path().join("store");

    Command::cargo_bin("mdmarks")
        .unwrap()
        .env("MDMARKS_STORE", &store_path)
        .args(["import"])
        .arg(&export)
        .assert()
        .success();

    for path in md_files(&store_path) {
        let content = read(&path);
        assert!(content.starts_with("---\n"));
        let (frontmatter, body) = content.split_once("---\n\n").unwrap();
        for key in ["space:", "tags:", "description:"] {
            assert!(
                !frontmatter.lines().any(|line| line.starts_with(key)),
                "frontmatter must not carry a {key} field: {frontmatter}"
            );
        }
        assert_eq!(body, "");
    }
}

#[test]
fn add_date_maps_to_rfc3339_added() {
    let work = TempDir::new().unwrap();
    let export = write_export(&work, DATES_TITLES);
    let store_path = work.path().join("store");

    Command::cargo_bin("mdmarks")
        .unwrap()
        .env("MDMARKS_STORE", &store_path)
        .args(["import"])
        .arg(&export)
        .assert()
        .success();

    let dated = frontmatter_for_url(&store_path, "https://dated.example.com/");
    assert!(
        dated
            .lines()
            .any(|l| l == "added: 2020-09-13T12:26:40+00:00"),
        "expected RFC 3339 added from ADD_DATE, got:\n{dated}"
    );
}

#[test]
fn missing_add_date_leaves_added_absent() {
    let work = TempDir::new().unwrap();
    let export = write_export(&work, DATES_TITLES);
    let store_path = work.path().join("store");

    Command::cargo_bin("mdmarks")
        .unwrap()
        .env("MDMARKS_STORE", &store_path)
        .args(["import"])
        .arg(&export)
        .assert()
        .success();

    let undated = frontmatter_for_url(&store_path, "https://undated.example.com/");
    assert!(
        !undated.lines().any(|l| l.starts_with("added:")),
        "bookmark without ADD_DATE must have no added field, got:\n{undated}"
    );
}

#[test]
fn empty_title_falls_back_to_url() {
    let work = TempDir::new().unwrap();
    let export = write_export(&work, DATES_TITLES);
    let store_path = work.path().join("store");

    Command::cargo_bin("mdmarks")
        .unwrap()
        .env("MDMARKS_STORE", &store_path)
        .args(["import"])
        .arg(&export)
        .assert()
        .success();

    let blank = frontmatter_for_url(&store_path, "https://blank.example.com/");
    assert!(
        blank
            .lines()
            .any(|l| l == "title: https://blank.example.com/"),
        "empty link text must fall back to url as title, got:\n{blank}"
    );
}

#[test]
fn entity_encoded_title_is_decoded() {
    let work = TempDir::new().unwrap();
    let export = write_export(&work, DATES_TITLES);
    let store_path = work.path().join("store");

    Command::cargo_bin("mdmarks")
        .unwrap()
        .env("MDMARKS_STORE", &store_path)
        .args(["import"])
        .arg(&export)
        .assert()
        .success();

    let entities = frontmatter_for_url(&store_path, "https://entities.example.com/");
    assert!(
        entities.lines().any(|l| l == "title: Ben & Jerry's"),
        "html entities in link text must be decoded, got:\n{entities}"
    );
}

#[test]
fn shared_titles_get_collision_suffix_without_overwrite() {
    let work = TempDir::new().unwrap();
    let export = write_fixture(&work);
    let store_path = work.path().join("store");

    Command::cargo_bin("mdmarks")
        .unwrap()
        .env("MDMARKS_STORE", &store_path)
        .args(["import"])
        .arg(&export)
        .assert()
        .success();

    let names: Vec<String> = md_files(&store_path)
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert!(names.contains(&"example-page.md".to_string()));
    assert!(names.contains(&"example-page-2.md".to_string()));

    let mut urls: Vec<String> = md_files(&store_path)
        .iter()
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("example-page")
        })
        .map(|p| {
            read(p)
                .lines()
                .find(|l| l.starts_with("url:"))
                .unwrap()
                .to_string()
        })
        .collect();
    urls.sort();
    assert_eq!(
        urls,
        vec![
            "url: https://blog.example.com/other".to_string(),
            "url: https://example.com/a?utm_source=news&id=7".to_string(),
        ]
    );
}

fn frontmatter_for_url(store_path: &std::path::Path, url: &str) -> String {
    let needle = format!("url: {url}");
    md_files(store_path)
        .iter()
        .map(|p| read(p))
        .find(|c| c.lines().any(|l| l == needle))
        .unwrap_or_else(|| panic!("no bookmark file for {url}"))
}

#[test]
fn nested_folders_map_to_tags_outer_to_inner() {
    let work = TempDir::new().unwrap();
    let export = write_export(&work, NESTED);
    let store_path = work.path().join("store");

    Command::cargo_bin("mdmarks")
        .unwrap()
        .env("MDMARKS_STORE", &store_path)
        .args(["import"])
        .arg(&export)
        .assert()
        .success();

    let reading = frontmatter_for_url(&store_path, "https://reading.example.com/");
    assert!(
        reading.contains("tags:\n- Work\n- Reading\n"),
        "expected outer-to-inner tags list, got:\n{reading}"
    );

    let papers = frontmatter_for_url(&store_path, "https://papers.example.com/");
    assert!(
        papers.contains("tags:\n- Work\n- Reading\n- Papers\n"),
        "expected one tag per segment, got:\n{papers}"
    );
}

#[test]
fn top_level_bookmark_has_no_tags_field() {
    let work = TempDir::new().unwrap();
    let export = write_export(&work, NESTED);
    let store_path = work.path().join("store");

    Command::cargo_bin("mdmarks")
        .unwrap()
        .env("MDMARKS_STORE", &store_path)
        .args(["import"])
        .arg(&export)
        .assert()
        .success();

    let top = frontmatter_for_url(&store_path, "https://top.example.com/");
    assert!(
        !top.lines().any(|l| l.starts_with("tags:")),
        "top-level bookmark must have no tags field, got:\n{top}"
    );
}
