use std::fmt::Write as _;

use url::{form_urlencoded, Url};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalizedUrl(String);

#[derive(Debug)]
pub struct NormalizeError(url::ParseError);

impl std::fmt::Display for NormalizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "could not parse url: {}", self.0)
    }
}

impl std::error::Error for NormalizeError {}

pub fn normalize(input: &str) -> Result<NormalizedUrl, NormalizeError> {
    let parsed = Url::parse(input).map_err(NormalizeError)?;

    let host = parsed.host_str().map(|h| {
        let lower = h.to_ascii_lowercase();
        lower
            .strip_prefix("www.")
            .map(str::to_string)
            .unwrap_or(lower)
    });

    let trimmed = parsed.path().trim_end_matches('/');
    let path = if trimmed.is_empty() { "/" } else { trimmed };

    let mut pairs: Vec<(String, String)> = parsed
        .query_pairs()
        .filter(|(k, _)| !is_tracker(k))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    pairs.sort();

    let mut out = String::from("https://");
    if let Some(host) = &host {
        out.push_str(host);
    }
    if let Some(port) = parsed.port() {
        let _ = write!(out, ":{port}");
    }
    out.push_str(path);
    if !pairs.is_empty() {
        let query = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(&pairs)
            .finish();
        out.push('?');
        out.push_str(&query);
    }

    Ok(NormalizedUrl(out))
}

fn is_tracker(key: &str) -> bool {
    key.starts_with("utm_") || matches!(key, "fbclid" | "gclid" | "mc_eid" | "ref")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn merges(a: &str, b: &str) {
        assert_eq!(
            normalize(a).unwrap(),
            normalize(b).unwrap(),
            "expected {a} and {b} to normalize equal"
        );
    }

    fn distinct(a: &str, b: &str) {
        assert_ne!(
            normalize(a).unwrap(),
            normalize(b).unwrap(),
            "expected {a} and {b} to normalize distinct"
        );
    }

    #[test]
    fn scheme_http_and_https_merge() {
        merges("http://example.com/a", "https://example.com/a");
    }

    #[test]
    fn host_case_merges() {
        merges("https://Example.COM/a", "https://example.com/a");
    }

    #[test]
    fn www_prefix_merges() {
        merges("https://www.example.com/a", "https://example.com/a");
    }

    #[test]
    fn trailing_slash_merges_except_root() {
        merges("https://example.com/a/", "https://example.com/a");
    }

    #[test]
    fn root_slash_is_kept_consistent() {
        merges("https://example.com", "https://example.com/");
    }

    #[test]
    fn fragment_dropped() {
        merges("https://example.com/a#section", "https://example.com/a");
    }

    #[test]
    fn tracker_params_stripped() {
        merges(
            "https://example.com/a?utm_source=x&fbclid=y&gclid=z&mc_eid=w&ref=q",
            "https://example.com/a",
        );
    }

    #[test]
    fn tracker_strip_keeps_real_params() {
        merges(
            "https://example.com/a?id=7&utm_source=x",
            "https://example.com/a?id=7",
        );
    }

    #[test]
    fn param_order_insensitive() {
        merges(
            "https://example.com/a?b=2&a=1",
            "https://example.com/a?a=1&b=2",
        );
    }

    #[test]
    fn combined_cosmetic_differences_merge() {
        merges(
            "http://WWW.Example.com/path/?b=2&a=1&utm_medium=email#frag",
            "https://example.com/path?a=1&b=2",
        );
    }

    #[test]
    fn path_case_is_distinct() {
        distinct("https://example.com/Path", "https://example.com/path");
    }

    #[test]
    fn different_path_is_distinct() {
        distinct("https://example.com/a", "https://example.com/b");
    }

    #[test]
    fn different_param_value_is_distinct() {
        distinct("https://example.com/a?id=7", "https://example.com/a?id=8");
    }

    #[test]
    fn userinfo_dropped_from_identity() {
        merges("https://user:pass@example.com/a", "https://example.com/a");
    }

    #[test]
    fn non_default_port_is_distinct() {
        distinct("https://example.com:8443/a", "https://example.com/a");
    }

    #[test]
    fn query_percent_encoding_merges() {
        merges(
            "https://example.com/a?q=hello%20world",
            "https://example.com/a?q=hello+world",
        );
    }

    #[test]
    fn encoded_delimiter_in_value_stays_distinct() {
        distinct(
            "https://example.com/a?x=1%26y%3D2",
            "https://example.com/a?x=1&y=2",
        );
    }
}
