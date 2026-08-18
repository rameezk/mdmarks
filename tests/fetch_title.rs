use std::sync::mpsc;
use std::thread;

use assert_cmd::Command;
use mdmarks::fetch::fetch_title;
use tempfile::TempDir;
use tiny_http::{Response, Server, StatusCode};

struct TestServer {
    base: String,
    handle: Option<thread::JoinHandle<()>>,
    server: std::sync::Arc<Server>,
}

impl TestServer {
    fn start(body: &'static str) -> Self {
        Self::start_with_status(body, 200)
    }

    fn start_with_status(body: &'static str, status: u16) -> Self {
        let server = std::sync::Arc::new(Server::http("127.0.0.1:0").unwrap());
        let base = format!("http://{}", server.server_addr());
        let (ready_tx, ready_rx) = mpsc::channel();
        let srv = server.clone();
        let handle = thread::spawn(move || {
            ready_tx.send(()).unwrap();
            for request in srv.incoming_requests() {
                let response = Response::from_string(body).with_status_code(StatusCode(status));
                let _ = request.respond(response);
            }
        });
        ready_rx.recv().unwrap();
        TestServer {
            base,
            handle: Some(handle),
            server,
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

#[test]
fn fetch_title_returns_some_for_page_with_title() {
    let server = TestServer::start("<html><head><title>Served Title</title></head></html>");
    assert_eq!(fetch_title(&server.base), Some("Served Title".to_string()));
}

#[test]
fn fetch_title_returns_none_without_title_element() {
    let server = TestServer::start("<html><body>no title here</body></html>");
    assert_eq!(fetch_title(&server.base), None);
}

#[test]
fn fetch_title_returns_none_on_non_2xx_status() {
    let server =
        TestServer::start_with_status("<html><head><title>Not Found</title></head></html>", 404);
    assert_eq!(fetch_title(&server.base), None);
}

#[test]
fn fetch_title_returns_none_on_connection_failure() {
    assert_eq!(fetch_title("http://127.0.0.1:1"), None);
}

#[test]
fn add_without_title_uses_fetched_title() {
    let server = TestServer::start("<html><head><title>Fetched Heading</title></head></html>");
    let store = TempDir::new().unwrap();
    Command::cargo_bin("mdmarks")
        .unwrap()
        .env("MDMARKS_STORE", store.path())
        .args(["add", &server.base])
        .assert()
        .success();

    let file = std::fs::read_dir(store.path())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
        .unwrap();
    assert_eq!(file.file_name().unwrap(), "fetched-heading.md");
    let content = std::fs::read_to_string(&file).unwrap();
    assert!(content.contains("title: Fetched Heading"));
}

#[test]
fn add_falls_back_to_url_when_fetch_fails() {
    let store = TempDir::new().unwrap();
    let url = "http://127.0.0.1:1/unreachable";
    Command::cargo_bin("mdmarks")
        .unwrap()
        .env("MDMARKS_STORE", store.path())
        .args(["add", url])
        .assert()
        .success();

    let file = std::fs::read_dir(store.path())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
        .unwrap();
    let content = std::fs::read_to_string(&file).unwrap();
    assert!(content.contains(&format!("url: {url}")));
    assert!(content.contains(&format!("title: {url}")));
}
