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
fn json_emits_ranked_results_as_records_in_ranked_order() {
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

    let assert = mdmarks(&store)
        .args(["search", "rust", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let records: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let titles: Vec<&str> = records
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["title"].as_str().unwrap())
        .collect();
    assert_eq!(
        titles,
        vec!["The Rust Programming Language", "Building Trust"]
    );
}

#[test]
fn json_no_match_emits_empty_array_and_exits_zero() {
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

    let assert = mdmarks(&store)
        .args(["search", "qqzzxx", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let records: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(records, serde_json::json!([]));
}

#[test]
fn json_and_human_cover_the_same_ranked_set_in_the_same_order() {
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

    let human = mdmarks(&store).args(["search", "rust"]).assert().success();
    let human_urls: Vec<String> = stdout_lines(&human)
        .iter()
        .map(|l| l.rsplit("  ").next().unwrap().to_string())
        .collect();

    let json = mdmarks(&store)
        .args(["search", "rust", "--json"])
        .assert()
        .success();
    let json_stdout = String::from_utf8(json.get_output().stdout.clone()).unwrap();
    let records: serde_json::Value = serde_json::from_str(&json_stdout).unwrap();
    let json_urls: Vec<String> = records
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["url"].as_str().unwrap().to_string())
        .collect();

    assert_eq!(human_urls, json_urls);
}

fn alfred_feed(assert: &assert_cmd::assert::Assert) -> serde_json::Value {
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    serde_json::from_str(&stdout).unwrap()
}

#[test]
fn alfred_emits_items_envelope_in_the_same_order_as_json() {
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

    let alfred = mdmarks(&store)
        .args(["search", "rust", "--format", "alfred"])
        .assert()
        .success();
    let feed = alfred_feed(&alfred);
    let alfred_urls: Vec<String> = feed["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["arg"].as_str().unwrap().to_string())
        .collect();

    let json = mdmarks(&store)
        .args(["search", "rust", "--json"])
        .assert()
        .success();
    let records: serde_json::Value =
        serde_json::from_str(&String::from_utf8(json.get_output().stdout.clone()).unwrap())
            .unwrap();
    let json_urls: Vec<String> = records
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["url"].as_str().unwrap().to_string())
        .collect();

    assert_eq!(alfred_urls, json_urls);
}

#[test]
fn alfred_no_match_emits_empty_items_and_exits_zero() {
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

    let assert = mdmarks(&store)
        .args(["search", "qqzzxx", "--format", "alfred"])
        .assert()
        .success();
    assert_eq!(alfred_feed(&assert), serde_json::json!({ "items": [] }));
}

#[test]
fn alfred_maps_each_bookmark_to_a_script_filter_item() {
    let store = TempDir::new().unwrap();
    seed(
        &store,
        "untitled.md",
        "---\nurl: https://example.com/only-url\nadded: 2026-01-01T00:00:00+00:00\nspace: work\n---\n\n",
    );

    let assert = mdmarks(&store)
        .args(["search", "", "--format", "alfred"])
        .assert()
        .success();
    let item = &alfred_feed(&assert)["items"][0];

    assert_eq!(item["title"], "https://example.com/only-url");
    assert_eq!(item["subtitle"], "work · https://example.com/only-url");
    assert_eq!(item["arg"], "https://example.com/only-url");
    assert_eq!(item["valid"], true);
    assert_eq!(item["action"]["url"], "https://example.com/only-url");
    assert_eq!(item["mods"]["cmd"]["arg"], "https://example.com/only-url");
    assert_eq!(item["mods"]["cmd"]["subtitle"], "Copy URL");
    assert!(item.as_object().unwrap().get("uid").is_none());
}

#[test]
fn alfred_substitutes_default_space_for_an_unset_space() {
    let store = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let config_dir = home.path().join(".config/mdmarks");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(
        config_dir.join("config.toml"),
        "default_space = \"personal\"\n",
    )
    .unwrap();
    seed(
        &store,
        "spaceless.md",
        &bookmark(
            "https://example.com/z",
            Some("Zed"),
            Some("2026-01-01T00:00:00+00:00"),
            &[],
            "",
        ),
    );

    let assert = mdmarks(&store)
        .env("HOME", home.path())
        .args(["search", "zed", "--format", "alfred"])
        .assert()
        .success();
    assert_eq!(
        alfred_feed(&assert)["items"][0]["subtitle"],
        "personal · https://example.com/z"
    );
}

#[test]
fn alfred_respects_the_space_filter() {
    let store = TempDir::new().unwrap();
    seed(
        &store,
        "work.md",
        "---\nurl: https://work.example.com\ntitle: Work\nadded: 2026-01-01T00:00:00+00:00\nspace: work\n---\n\n",
    );
    seed(
        &store,
        "home.md",
        "---\nurl: https://home.example.com\ntitle: Home\nadded: 2026-02-01T00:00:00+00:00\nspace: home\n---\n\n",
    );

    let assert = mdmarks(&store)
        .args(["search", "", "--format", "alfred", "--space", "work"])
        .assert()
        .success();
    let items = alfred_feed(&assert)["items"].as_array().unwrap().clone();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["arg"], "https://work.example.com");
}

fn spaces_config() -> TempDir {
    let home = TempDir::new().unwrap();
    let config_dir = home.path().join(".config/mdmarks");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(
        config_dir.join("config.toml"),
        "[spaces.work]\nbrowser = \"Google Chrome\"\n\n[spaces.home]\nbrowser = \"Firefox\"\n",
    )
    .unwrap();
    home
}

fn spaced_corpus(store: &TempDir) {
    seed(
        store,
        "work-rust.md",
        "---\nurl: https://work.example.com/rust\ntitle: Rust at Work\nadded: 2026-01-01T00:00:00+00:00\nspace: work\n---\n\n",
    );
    seed(
        store,
        "work-kube.md",
        "---\nurl: https://work.example.com/kube\ntitle: Kubernetes Guide\nadded: 2026-03-01T00:00:00+00:00\nspace: work\n---\n\n",
    );
    seed(
        store,
        "home-rust.md",
        "---\nurl: https://home.example.com/rust\ntitle: Rust at Home\nadded: 2026-02-01T00:00:00+00:00\nspace: home\n---\n\n",
    );
}

fn args_of(feed: &serde_json::Value) -> Vec<String> {
    feed["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["arg"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn alfred_space_prefix_scopes_and_fuzzy_matches_the_remainder() {
    let store = TempDir::new().unwrap();
    let home = spaces_config();
    spaced_corpus(&store);

    let assert = mdmarks(&store)
        .env("HOME", home.path())
        .args(["search", "work: rust", "--format", "alfred"])
        .assert()
        .success();

    assert_eq!(
        args_of(&alfred_feed(&assert)),
        vec!["https://work.example.com/rust"]
    );
}

#[test]
fn alfred_bare_space_prefix_returns_the_whole_space_feed_newest_first() {
    let store = TempDir::new().unwrap();
    let home = spaces_config();
    spaced_corpus(&store);

    let assert = mdmarks(&store)
        .env("HOME", home.path())
        .args(["search", "work:", "--format", "alfred"])
        .assert()
        .success();

    assert_eq!(
        args_of(&alfred_feed(&assert)),
        vec![
            "https://work.example.com/kube",
            "https://work.example.com/rust"
        ]
    );
}

#[test]
fn alfred_unknown_token_prefix_stays_the_query() {
    let store = TempDir::new().unwrap();
    let home = spaces_config();
    seed(
        &store,
        "colon.md",
        "---\nurl: http://example.com/rust\ntitle: Colon URL\nadded: 2026-01-01T00:00:00+00:00\nspace: work\n---\n\n",
    );

    let assert = mdmarks(&store)
        .env("HOME", home.path())
        .args(["search", "http://example.com", "--format", "alfred"])
        .assert()
        .success();

    assert_eq!(
        args_of(&alfred_feed(&assert)),
        vec!["http://example.com/rust"]
    );
}

#[test]
fn alfred_space_flag_takes_precedence_over_the_query_prefix() {
    let store = TempDir::new().unwrap();
    let home = spaces_config();
    spaced_corpus(&store);

    let assert = mdmarks(&store)
        .env("HOME", home.path())
        .args(["search", "home:", "--format", "alfred", "--space", "work"])
        .assert()
        .success();

    assert_eq!(
        args_of(&alfred_feed(&assert)),
        Vec::<String>::new(),
        "flag scopes to work; the literal `home:` query matches nothing there"
    );
}

#[test]
fn space_prefix_is_not_interpreted_without_alfred_format() {
    let store = TempDir::new().unwrap();
    let home = spaces_config();
    spaced_corpus(&store);

    let assert = mdmarks(&store)
        .env("HOME", home.path())
        .args(["search", "work:", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let records: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(
        records.as_array().unwrap().len(),
        0,
        "plain --json treats `work:` as a literal fuzzy query, matching nothing"
    );
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
