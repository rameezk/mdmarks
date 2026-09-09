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

fn quick_add_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("alfred/workflow/quick_add.sh")
}

fn quick_add_action_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("alfred/workflow/quick_add_action.sh")
}

fn objects_of_type<'a>(plist: &'a Value, kind: &str) -> Vec<&'a plist::Dictionary> {
    objects(plist)
        .iter()
        .map(|o| o.as_dictionary().unwrap())
        .filter(|o| o.get("type").and_then(Value::as_string) == Some(kind))
        .collect()
}

fn find_by_config<'a>(
    plist: &'a Value,
    kind: &str,
    key: &str,
    value: &str,
) -> &'a plist::Dictionary {
    objects_of_type(plist, kind)
        .into_iter()
        .find(|o| config(o).get(key).and_then(Value::as_string) == Some(value))
        .unwrap_or_else(|| panic!("no {kind} with config.{key} == {value}"))
}

fn wired(plist: &Value, from: &plist::Dictionary, to: &plist::Dictionary) -> bool {
    plist
        .as_dictionary()
        .unwrap()
        .get("connections")
        .and_then(Value::as_dictionary)
        .and_then(|c| c.get(uid(from)))
        .and_then(Value::as_array)
        .is_some_and(|edges| {
            edges.iter().any(|c| {
                let c = c.as_dictionary().unwrap();
                c.get("destinationuid").and_then(Value::as_string) == Some(uid(to))
                    && c.get("modifiers").and_then(Value::as_signed_integer) == Some(0)
            })
        })
}

fn make_shim(dir: &TempDir, name: &str, body: &str) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, body).unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    use std::os::unix::fs::PermissionsExt;
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).unwrap();
    path
}

fn run_quick_add(clipboard: &str, query: &str) -> String {
    let home = TempDir::new().unwrap();
    let config_dir = home.path().join(".config/mdmarks");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(
        config_dir.join("config.toml"),
        "default_space = \"personal\"\n\n\
         [spaces.work]\nbrowser = \"Google Chrome\"\nprofile = \"Work\"\n\n\
         [spaces.personal]\nbrowser = \"Safari\"\n",
    )
    .unwrap();
    let store = home.path().join("store");
    std::fs::create_dir_all(&store).unwrap();

    let shim = TempDir::new().unwrap();
    make_shim(
        &shim,
        "pbpaste",
        &format!(
            "#!/usr/bin/env bash\nprintf '%s' {}\n",
            shell_quote(clipboard)
        ),
    );
    let path = format!(
        "{}:{}",
        shim.path().display(),
        std::env::var("PATH").unwrap()
    );

    let out = Command::new("bash")
        .arg(quick_add_script())
        .arg(query)
        .env("HOME", home.path())
        .env("MDMARKS_STORE", &store)
        .env("MDMARKS_BIN", binary())
        .env("PATH", path)
        .output()
        .unwrap();
    assert!(out.status.success(), "quick_add.sh failed: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn feed(stdout: &str) -> serde_json::Value {
    serde_json::from_str(stdout).unwrap()
}

#[test]
fn quick_add_offers_the_space_picker_default_first_with_url_and_title_variables() {
    let out = run_quick_add("https://example.com/page", "My Title");
    let json = feed(&out);

    let items = json["items"].as_array().unwrap();
    assert_eq!(items[0]["title"], "(default)");
    assert_eq!(items[0]["arg"], "");
    assert_eq!(items[0]["valid"], true);
    assert!(items.iter().any(|i| i["arg"] == "work"));
    assert!(items.iter().any(|i| i["arg"] == "personal"));

    for item in items {
        assert_eq!(item["variables"]["url"], "https://example.com/page");
        assert_eq!(item["variables"]["title"], "My Title");
    }
}

#[test]
fn quick_add_empty_query_carries_an_empty_title_variable() {
    let out = run_quick_add("https://example.com/page", "");
    let json = feed(&out);
    let items = json["items"].as_array().unwrap();
    assert_eq!(items[0]["variables"]["title"], "");
    assert_eq!(items[0]["variables"]["url"], "https://example.com/page");
}

#[test]
fn quick_add_trims_surrounding_whitespace_from_the_clipboard_url() {
    let out = run_quick_add("  https://example.com/page  ", "");
    let json = feed(&out);
    let items = json["items"].as_array().unwrap();
    assert_eq!(items[0]["variables"]["url"], "https://example.com/page");
}

#[test]
fn quick_add_rejects_a_non_url_clipboard_with_a_single_non_actionable_row() {
    let out = run_quick_add("just some text", "");
    let json = feed(&out);
    let items = json["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["valid"], false);
    assert!(items[0]["arg"].is_null());
    assert!(json.get("variables").is_none());
}

fn run_action(space_arg: &str, url: &str, title: &str) -> String {
    let shim = TempDir::new().unwrap();
    let recorded = shim.path().join("argv");
    make_shim(
        &shim,
        "mdmarks",
        &format!(
            "#!/usr/bin/env bash\nprintf '%s\\0' \"$@\" > {}\nprintf 'Saved \\u2713 ok\\n'\n",
            shell_quote(recorded.to_str().unwrap())
        ),
    );

    let out = Command::new("bash")
        .arg(quick_add_action_script())
        .arg(space_arg)
        .env("MDMARKS_BIN", shim.path().join("mdmarks"))
        .env("url", url)
        .env("title", title)
        .env("PATH", std::env::var("PATH").unwrap())
        .output()
        .unwrap();
    assert!(out.status.success(), "action failed: {out:?}");
    std::fs::read_to_string(recorded).unwrap()
}

fn argv(recorded: &str) -> Vec<&str> {
    recorded.split('\0').filter(|s| !s.is_empty()).collect()
}

#[test]
fn action_omits_space_and_title_for_the_default_row_and_empty_query() {
    let recorded = run_action("", "https://example.com/x", "");
    assert_eq!(argv(&recorded), vec!["add", "https://example.com/x"]);
}

#[test]
fn action_passes_space_and_title_when_present() {
    let recorded = run_action("work", "https://example.com/x", "Hello World");
    assert_eq!(
        argv(&recorded),
        vec![
            "add",
            "https://example.com/x",
            "--space",
            "work",
            "--title",
            "Hello World",
        ]
    );
}

struct Notified {
    success: bool,
    subtitle: String,
    body: String,
}

fn run_action_notification(space_arg: &str, url: &str, add_exit: u8, add_stdout: &str) -> Notified {
    let shim = TempDir::new().unwrap();
    make_shim(
        &shim,
        "mdmarks",
        &format!(
            "#!/usr/bin/env bash\nprintf '%s\\n' {}\nexit {add_exit}\n",
            shell_quote(add_stdout)
        ),
    );
    let recorded = shim.path().join("notification");
    make_shim(
        &shim,
        "osascript",
        &format!(
            "#!/usr/bin/env bash\ncat >/dev/null\nprintf '%s\\0' \"$@\" > {}\n",
            shell_quote(recorded.to_str().unwrap())
        ),
    );

    let out = Command::new("bash")
        .arg(quick_add_action_script())
        .arg(space_arg)
        .env("MDMARKS_BIN", shim.path().join("mdmarks"))
        .env("url", url)
        .env("title", "")
        .env(
            "PATH",
            format!(
                "{}:{}",
                shim.path().display(),
                std::env::var("PATH").unwrap()
            ),
        )
        .output()
        .unwrap();

    let recorded = std::fs::read_to_string(&recorded).unwrap();
    let parts: Vec<&str> = recorded.split('\0').filter(|s| !s.is_empty()).collect();
    Notified {
        success: out.status.success(),
        subtitle: parts.get(1).unwrap_or(&"").to_string(),
        body: parts.get(2).unwrap_or(&"").to_string(),
    }
}

#[test]
fn action_notifies_with_the_result_line_and_url_for_the_default_row() {
    let n = run_action_notification("", "https://example.com/x", 0, "Saved ✓ Example");
    assert!(
        n.success,
        "the action must exit 0 so Alfred does not flag an error"
    );
    assert_eq!(n.subtitle, "Saved ✓ Example");
    assert!(n.body.contains("https://example.com/x"), "body: {}", n.body);
    assert!(
        n.body.contains("(default)"),
        "body must name the Space: {}",
        n.body
    );
}

#[test]
fn action_notifies_the_chosen_space_and_survives_already_saved_exit() {
    let n = run_action_notification("work", "https://example.com/x", 3, "Already saved: Example");
    assert!(
        n.success,
        "a non-zero add (already saved) must not leave the action non-zero"
    );
    assert_eq!(n.subtitle, "Already saved: Example");
    assert!(n.body.contains("Space: work"), "body: {}", n.body);
}

#[test]
fn action_notifies_the_error_text_when_add_fails() {
    let n = run_action_notification(
        "",
        "https://example.com/x",
        1,
        "error: not a valid http(s) url",
    );
    assert!(n.success, "an add error must not leave the action non-zero");
    assert_eq!(n.subtitle, "error: not a valid http(s) url");
}

#[test]
fn info_plist_wires_bma_to_the_self_notifying_quick_add_action() {
    let plist = info_plist();

    let filter = find_by_config(
        &plist,
        "alfred.workflow.input.scriptfilter",
        "keyword",
        "bma",
    );
    assert_eq!(
        config(filter).get("scriptfile").and_then(Value::as_string),
        Some("./quick_add.sh")
    );
    assert_eq!(
        config(filter)
            .get("scriptargtype")
            .and_then(Value::as_signed_integer),
        Some(1)
    );

    let action = find_by_config(
        &plist,
        "alfred.workflow.action.script",
        "scriptfile",
        "./quick_add_action.sh",
    );

    assert!(
        wired(&plist, filter, action),
        "Enter on the bma Script Filter must run the quick-add action"
    );
    assert!(
        objects_of_type(&plist, "alfred.workflow.output.notification").is_empty(),
        "the action posts its own notification, so there must be no notification node to swallow the result"
    );

    let body = std::fs::read_to_string(quick_add_action_script()).unwrap();
    assert!(
        body.contains("display notification"),
        "the quick-add action must post its own notification"
    );
}
