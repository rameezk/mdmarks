use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Frontmatter {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub added: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space: Option<String>,
}

#[derive(Debug)]
pub enum FrontmatterError {
    MissingDelimiter,
    Yaml(serde_yaml_ng::Error),
}

impl std::fmt::Display for FrontmatterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrontmatterError::MissingDelimiter => {
                write!(f, "file is missing its `---` frontmatter delimiters")
            }
            FrontmatterError::Yaml(e) => write!(f, "invalid frontmatter yaml: {e}"),
        }
    }
}

impl std::error::Error for FrontmatterError {}

impl Frontmatter {
    pub fn new(url: String, title: String, added: String) -> Self {
        Frontmatter {
            url,
            title: Some(title),
            tags: None,
            added: Some(added),
            description: None,
            space: None,
        }
    }
}

pub fn serialize(fm: &Frontmatter, body: &str) -> Result<String, FrontmatterError> {
    let yaml = serde_yaml_ng::to_string(fm).map_err(FrontmatterError::Yaml)?;
    Ok(format!("---\n{yaml}---\n\n{body}"))
}

pub fn parse(content: &str) -> Result<(Frontmatter, String), FrontmatterError> {
    let rest = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
        .ok_or(FrontmatterError::MissingDelimiter)?;

    let (yaml, body) =
        split_at_closing_delimiter(rest).ok_or(FrontmatterError::MissingDelimiter)?;

    let fm: Frontmatter = serde_yaml_ng::from_str(yaml).map_err(FrontmatterError::Yaml)?;
    Ok((fm, body.to_string()))
}

fn split_at_closing_delimiter(rest: &str) -> Option<(&str, &str)> {
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\n', '\r']);
        if trimmed == "---" {
            let yaml = &rest[..offset];
            let after = &rest[offset + line.len()..];
            let body = after
                .strip_prefix('\n')
                .or_else(|| after.strip_prefix("\r\n"))
                .unwrap_or(after);
            return Some((yaml, body));
        }
        offset += line.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_written_fields() {
        let fm = Frontmatter::new(
            "https://example.com/a?utm=1".to_string(),
            "Example".to_string(),
            "2026-08-18T00:00:00+00:00".to_string(),
        );
        let content = serialize(&fm, "").unwrap();
        let (parsed, body) = parse(&content).unwrap();
        assert_eq!(parsed, fm);
        assert_eq!(body, "");
    }

    #[test]
    fn parses_full_schema() {
        let content = "---\nurl: https://example.com\ntitle: T\ntags:\n  - a\n  - b\nadded: 2026-01-01T00:00:00+00:00\ndescription: d\nspace: work\n---\n\nsome notes\n";
        let (fm, body) = parse(content).unwrap();
        assert_eq!(fm.url, "https://example.com");
        assert_eq!(fm.tags, Some(vec!["a".to_string(), "b".to_string()]));
        assert_eq!(fm.space, Some("work".to_string()));
        assert_eq!(body, "some notes\n");
    }

    #[test]
    fn unset_optionals_are_absent_not_empty() {
        let fm = Frontmatter::new(
            "https://example.com".to_string(),
            "T".to_string(),
            "2026-01-01T00:00:00+00:00".to_string(),
        );
        let content = serialize(&fm, "").unwrap();
        assert!(!content.contains("tags"));
        assert!(!content.contains("description"));
        assert!(!content.contains("space"));
    }

    #[test]
    fn missing_delimiter_is_error() {
        assert!(parse("no frontmatter here").is_err());
    }
}
