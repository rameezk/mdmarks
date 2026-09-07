use std::path::PathBuf;
use std::process::Command;

use plist::Value;
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

fn info_plist() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("alfred/workflow/info.plist");
    Value::from_file(path).expect("info.plist parses as a plist")
}

fn objects(plist: &Value) -> &Vec<Value> {
    plist
        .as_dictionary()
        .unwrap()
        .get("objects")
        .and_then(Value::as_array)
        .expect("objects array")
}

fn object_of_type<'a>(plist: &'a Value, kind: &str) -> &'a plist::Dictionary {
    objects(plist)
        .iter()
        .map(|o| o.as_dictionary().unwrap())
        .find(|o| o.get("type").and_then(Value::as_string) == Some(kind))
        .unwrap_or_else(|| panic!("no object of type {kind}"))
}

fn uid(obj: &plist::Dictionary) -> &str {
    obj.get("uid").and_then(Value::as_string).unwrap()
}

fn config(obj: &plist::Dictionary) -> &plist::Dictionary {
    obj.get("config").and_then(Value::as_dictionary).unwrap()
}

#[test]
fn info_plist_ships_a_wired_node_graph_not_an_empty_stub() {
    let plist = info_plist();

    assert!(
        !objects(&plist).is_empty(),
        "objects must not regress to an empty stub"
    );

    let filter = object_of_type(&plist, "alfred.workflow.input.scriptfilter");
    let filter_cfg = config(filter);
    assert_eq!(
        filter_cfg.get("keyword").and_then(Value::as_string),
        Some("bm")
    );
    assert_eq!(
        filter_cfg.get("scriptfile").and_then(Value::as_string),
        Some("./script_filter.sh")
    );

    let action = object_of_type(&plist, "alfred.workflow.action.script");
    let action_cfg = config(action);
    let script = action_cfg.get("script").and_then(Value::as_string).unwrap();
    assert!(
        script.contains("open \"$1\""),
        "the action must invoke `mdmarks open`: {script}"
    );
    assert_eq!(
        action_cfg.get("scriptargtype").and_then(Value::as_signed_integer),
        Some(1),
        "the action reads $1, so Alfred must pass input as argv (scriptargtype 1), not {{query}} (0)"
    );

    let connections = plist
        .as_dictionary()
        .unwrap()
        .get("connections")
        .and_then(Value::as_dictionary)
        .expect("connections dict");
    let from_filter = connections
        .get(uid(filter))
        .and_then(Value::as_array)
        .expect("the Script Filter must have outgoing connections");
    let wired_to_action = from_filter.iter().any(|c| {
        let c = c.as_dictionary().unwrap();
        c.get("destinationuid").and_then(Value::as_string) == Some(uid(action))
            && c.get("modifiers").and_then(Value::as_signed_integer) == Some(0)
    });
    assert!(
        wired_to_action,
        "Enter (modifiers 0) on the Script Filter must run the open action"
    );
}
