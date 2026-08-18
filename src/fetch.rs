use std::time::Duration;

pub fn fetch_title(url: &str) -> Option<String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .build();
    let body = agent.get(url).call().ok()?.into_string().ok()?;
    extract_title(&body)
}

fn extract_title(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let open = lower.find("<title")?;
    let content_start = lower[open..].find('>')? + open + 1;
    let close = lower[content_start..].find("</title>")? + content_start;
    let raw = &html[content_start..close];

    let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let decoded = decode_basic_entities(&collapsed);
    let trimmed = decoded.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn decode_basic_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_simple_title() {
        assert_eq!(
            extract_title("<html><head><title>Hello</title></head></html>"),
            Some("Hello".to_string())
        );
    }

    #[test]
    fn is_case_insensitive_and_collapses_whitespace() {
        assert_eq!(
            extract_title("<TITLE>\n  Multi\n  Line  </TITLE>"),
            Some("Multi Line".to_string())
        );
    }

    #[test]
    fn decodes_basic_entities() {
        assert_eq!(
            extract_title("<title>Tom &amp; Jerry</title>"),
            Some("Tom & Jerry".to_string())
        );
    }

    #[test]
    fn no_title_element_is_none() {
        assert_eq!(extract_title("<html><body>hi</body></html>"), None);
    }

    #[test]
    fn empty_title_is_none() {
        assert_eq!(extract_title("<title>   </title>"), None);
    }
}
