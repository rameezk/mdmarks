use assert_cmd::Command;
use tempfile::TempDir;

fn mdmarks(home: &TempDir) -> Command {
    let store = home.path().join("store");
    std::fs::create_dir_all(&store).unwrap();
    let mut cmd = Command::cargo_bin("mdmarks").unwrap();
    cmd.env("HOME", home.path()).env("MDMARKS_STORE", store);
    cmd
}

fn with_config(contents: &str) -> TempDir {
    let home = TempDir::new().unwrap();
    let config_dir = home.path().join(".config/mdmarks");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.toml"), contents).unwrap();
    home
}

fn stdout_lines(assert: &assert_cmd::assert::Assert) -> Vec<String> {
    String::from_utf8(assert.get_output().stdout.clone())
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

fn feed(assert: &assert_cmd::assert::Assert) -> serde_json::Value {
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    serde_json::from_str(&stdout).unwrap()
}

const CONFIG: &str = "default_space = \"personal\"\n\n\
[spaces.work]\nbrowser = \"Google Chrome\"\nprofile = \"Work\"\n\n\
[spaces.personal]\nbrowser = \"Safari\"\n";

#[test]
fn human_prints_each_space_with_its_resolved_browser_and_profile() {
    let home = with_config(CONFIG);

    let assert = mdmarks(&home).arg("spaces").assert().success();
    let lines = stdout_lines(&assert);

    assert_eq!(
        lines,
        vec![
            "work  Google Chrome · Work".to_string(),
            "personal  Safari".to_string(),
        ]
    );
}

#[test]
fn human_with_no_spaces_prints_nothing_and_exits_zero() {
    let home = with_config("");
    mdmarks(&home).arg("spaces").assert().success().stdout("");
}

#[test]
fn alfred_leads_with_default_then_each_space_in_config_order() {
    let home = with_config(CONFIG);

    let assert = mdmarks(&home)
        .args(["spaces", "--format", "alfred"])
        .assert()
        .success();
    let items = feed(&assert)["items"].as_array().unwrap().clone();

    assert_eq!(items.len(), 3);

    assert_eq!(items[0]["title"], "(default)");
    assert_eq!(items[0]["arg"], "");
    assert_eq!(items[0]["subtitle"], "Safari");
    assert_eq!(items[0]["valid"], true);

    assert_eq!(items[1]["title"], "work");
    assert_eq!(items[1]["arg"], "work");
    assert_eq!(items[1]["subtitle"], "Google Chrome · Work");

    assert_eq!(items[2]["title"], "personal");
    assert_eq!(items[2]["arg"], "personal");
    assert_eq!(items[2]["subtitle"], "Safari");
}

#[test]
fn alfred_rows_advertise_no_action_or_reordering_metadata() {
    let home = with_config(CONFIG);

    let assert = mdmarks(&home)
        .args(["spaces", "--format", "alfred"])
        .assert()
        .success();
    for item in feed(&assert)["items"].as_array().unwrap() {
        let obj = item.as_object().unwrap();
        assert!(obj.get("action").is_none(), "{item}");
        assert!(obj.get("uid").is_none(), "{item}");
        assert!(obj.get("mods").is_none(), "{item}");
    }
}

#[test]
fn alfred_with_no_spaces_still_emits_a_working_default_row() {
    let home = with_config("");

    let assert = mdmarks(&home)
        .args(["spaces", "--format", "alfred"])
        .assert()
        .success();
    let items = feed(&assert)["items"].as_array().unwrap().clone();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["title"], "(default)");
    assert_eq!(items[0]["arg"], "");
    assert_eq!(items[0]["subtitle"], "system default browser");
}

#[test]
fn alfred_default_row_falls_back_to_system_browser_without_a_default_space() {
    let home = with_config("[spaces.work]\nbrowser = \"Google Chrome\"\n");

    let assert = mdmarks(&home)
        .args(["spaces", "--format", "alfred"])
        .assert()
        .success();
    assert_eq!(
        feed(&assert)["items"][0]["subtitle"],
        "system default browser"
    );
}

#[test]
fn spaces_takes_no_query_argument() {
    let home = with_config(CONFIG);
    mdmarks(&home).args(["spaces", "work"]).assert().failure();
}
