use assert_cmd::Command;
use tempfile::TempDir;

const FIXTURE: &str = include_str!("fixtures/flat-export.html");

fn write_fixture(dir: &TempDir) -> std::path::PathBuf {
    let path = dir.path().join("export.html");
    std::fs::write(&path, FIXTURE).unwrap();
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
fn imported_bookmarks_have_no_space_tags_added_or_description() {
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
        for key in ["space:", "tags:", "added:", "description:"] {
            assert!(
                !frontmatter.lines().any(|line| line.starts_with(key)),
                "frontmatter must not carry a {key} field: {frontmatter}"
            );
        }
        assert_eq!(body, "");
    }
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
