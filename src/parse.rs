//! Streaming parser for the `<seo-audit>` XML format using quick-xml.

use anyhow::{anyhow, Context, Result};
use quick_xml::events::Event;
use quick_xml::name::QName;
use quick_xml::reader::Reader;

use crate::model::{Audit, Category, Issue, Severity, Summary};

pub fn parse_audit(xml: &[u8]) -> Result<Audit> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut audit = Audit {
        url: String::new(),
        date: String::new(),
        score: 0,
        grade: String::new(),
        pages: 0,
        summary: Summary::default(),
        categories: Vec::new(),
        issues: Vec::new(),
        issues_total: 0,
        issues_omitted: 0,
    };

    let mut in_issues = false;
    let mut in_categories = false;
    let mut current_issue: Option<Issue> = None;
    let mut current_text_tag: Option<String> = None;
    let mut text_buf = String::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                let name = local_name(e.name());
                match name.as_str() {
                    "seo-audit" => {
                        audit.url = attr(&e, "url").unwrap_or_default();
                        audit.date = attr(&e, "date").unwrap_or_default();
                        audit.score = attr(&e, "score").and_then(|v| v.parse().ok()).unwrap_or(0);
                        audit.grade = attr(&e, "grade").unwrap_or_default();
                        audit.pages = attr(&e, "pages").and_then(|v| v.parse().ok()).unwrap_or(0);
                    }
                    "summary" => {
                        audit.summary = Summary {
                            passed: attr(&e, "passed").and_then(|v| v.parse().ok()).unwrap_or(0),
                            warnings: attr(&e, "warnings")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0),
                            failures: attr(&e, "failures")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0),
                            not_measured: attr(&e, "notMeasured")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0),
                        };
                    }
                    "categories" => in_categories = true,
                    "cat" if in_categories => {
                        audit.categories.push(Category {
                            id: attr(&e, "id").unwrap_or_default(),
                            score: attr(&e, "score").and_then(|v| v.parse().ok()).unwrap_or(0),
                            passed: attr(&e, "p").and_then(|v| v.parse().ok()).unwrap_or(0),
                            warnings: attr(&e, "w").and_then(|v| v.parse().ok()).unwrap_or(0),
                            failures: attr(&e, "f").and_then(|v| v.parse().ok()).unwrap_or(0),
                            not_measured: attr(&e, "nm").and_then(|v| v.parse().ok()).unwrap_or(0),
                        });
                    }
                    "issues" => {
                        in_issues = true;
                        audit.issues_total =
                            attr(&e, "total").and_then(|v| v.parse().ok()).unwrap_or(0);
                        audit.issues_omitted = attr(&e, "omitted")
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0);
                    }
                    "issue" if in_issues => {
                        current_issue = Some(Issue {
                            severity: match attr(&e, "severity").as_deref() {
                                Some("critical") => Severity::Critical,
                                _ => Severity::Warning,
                            },
                            rule: attr(&e, "rule").unwrap_or_default(),
                            category: attr(&e, "cat").unwrap_or_default(),
                            message: String::new(),
                            fix: String::new(),
                            details: String::new(),
                            pages: Vec::new(),
                            pages_affected: attr(&e, "pages")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0),
                            pages_total: attr(&e, "of").and_then(|v| v.parse().ok()).unwrap_or(0),
                        });
                    }
                    "on" if current_issue.is_some() => {
                        current_text_tag = Some("on".to_string());
                        text_buf.clear();
                    }
                    "msg" | "fix" | "details" if current_issue.is_some() => {
                        current_text_tag = Some(name.to_string());
                        text_buf.clear();
                    }
                    _ => {}
                }
            }
            Event::Empty(e) => {
                let name = local_name(e.name());
                if name == "summary" {
                    audit.summary = Summary {
                        passed: attr(&e, "passed").and_then(|v| v.parse().ok()).unwrap_or(0),
                        warnings: attr(&e, "warnings")
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0),
                        failures: attr(&e, "failures")
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0),
                        not_measured: attr(&e, "notMeasured")
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0),
                    };
                }
                if name == "cat" && in_categories {
                    audit.categories.push(Category {
                        id: attr(&e, "id").unwrap_or_default(),
                        score: attr(&e, "score").and_then(|v| v.parse().ok()).unwrap_or(0),
                        passed: attr(&e, "p").and_then(|v| v.parse().ok()).unwrap_or(0),
                        warnings: attr(&e, "w").and_then(|v| v.parse().ok()).unwrap_or(0),
                        failures: attr(&e, "f").and_then(|v| v.parse().ok()).unwrap_or(0),
                        not_measured: attr(&e, "nm").and_then(|v| v.parse().ok()).unwrap_or(0),
                    });
                }
            }
            Event::Text(e) => {
                if current_text_tag.is_some() {
                    let raw = e.xml_content(quick_xml::XmlVersion::Explicit1_0);
                    let text = quick_xml::escape::unescape(&raw)
                        .map(|c| c.into_owned())
                        .unwrap_or_else(|_| raw.into_owned());
                    push_text(&mut text_buf, &text);
                }
            }
            Event::GeneralRef(e) => {
                if current_text_tag.is_some() {
                    let name = e.into_inner();
                    let resolved = quick_xml::escape::resolve_predefined_entity(&name)
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("&{name};"));
                    push_text(&mut text_buf, &resolved);
                }
            }
            Event::End(e) => {
                let name = local_name(e.name());
                match name.as_str() {
                    "categories" => in_categories = false,
                    "issues" => in_issues = false,
                    "issue" => {
                        if let Some(issue) = current_issue.take() {
                            audit.issues.push(issue);
                        }
                    }
                    "msg" | "fix" | "details" | "on" => {
                        if let (Some(issue), Some(tag)) =
                            (&mut current_issue, current_text_tag.take())
                        {
                            let text = text_buf.trim().to_string();
                            match tag.as_str() {
                                "msg" => issue.message = text,
                                "fix" => issue.fix = text,
                                "details" => issue.details = text,
                                "on" => {
                                    issue.pages =
                                        text.split_whitespace().map(str::to_string).collect();
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    if audit.url.is_empty() {
        return Err(anyhow!("not a seo-audit file: missing url attribute"));
    }
    Ok(audit)
}

fn push_text(buf: &mut String, text: &str) {
    if buf.is_empty() {
        buf.push_str(text);
        return;
    }
    let need_space = !buf.ends_with(char::is_whitespace) && !text.starts_with(char::is_whitespace);
    if need_space {
        buf.push(' ');
    }
    buf.push_str(text);
}

fn local_name(qname: QName<'_>) -> String {
    qname.local_name().as_ref().to_string()
}

fn attr(e: &quick_xml::events::BytesStart<'_>, key: &str) -> Option<String> {
    e.attributes()
        .with_checks(false)
        .flatten()
        .find(|a| a.key.as_ref() == key)
        .map(|a| {
            quick_xml::escape::unescape(&a.value)
                .map(|c| c.into_owned())
                .unwrap_or_else(|_| a.value.into_owned())
        })
}

pub fn parse_audit_file(path: &std::path::Path) -> Result<Audit> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    parse_audit(&bytes).with_context(|| format!("parsing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0"?>
<seo-audit schema="2" url="https://example.com" score="73" grade="C" pages="12" date="2026-10-24" nonce="abc">
<summary passed="100" warnings="20" failures="5" notMeasured="3"/>
<categories>
<cat id="core" score="95" p="50" w="4" f="1"/>
<cat id="geo" score="59" p="10" w="20" f="5"/>
</categories>
<issues count="2" total="3" omitted="1" note="ranked by impact">
<issue severity="critical" rule="core-h1-present" cat="core" pages="2" of="2">
<on>https://example.com/a https://example.com/b</on>
<msg><x>Missing H1 &amp; heading</x></msg>
<fix>Add a single H1 tag</fix>
<details><x>found=false</x></details>
</issue>
<issue severity="warning" rule="geo-content-structure" cat="geo">
<on>https://example.com/</on>
<msg><x>Poor content structure</x></msg>
<fix>Structure content</fix>
<details><x>signals=1</x></details>
</issue>
</issues>
<not-measured></not-measured>
<passed></passed>
</seo-audit>"#;

    #[test]
    fn parses_full_audit() {
        let audit = parse_audit(SAMPLE.as_bytes()).unwrap();
        assert_eq!(audit.url, "https://example.com");
        assert_eq!(audit.date, "2026-10-24");
        assert_eq!(audit.score, 73);
        assert_eq!(audit.grade, "C");
        assert_eq!(audit.pages, 12);
        assert_eq!(audit.summary.passed, 100);
        assert_eq!(audit.summary.warnings, 20);
        assert_eq!(audit.summary.failures, 5);
        assert_eq!(audit.summary.not_measured, 3);
        assert_eq!(audit.categories.len(), 2);
        assert_eq!(audit.categories[0].id, "core");
        assert_eq!(audit.categories[0].score, 95);
        assert_eq!(audit.categories[1].id, "geo");
        assert_eq!(audit.issues.len(), 2);
        assert_eq!(audit.issues_total, 3);
        assert_eq!(audit.issues_omitted, 1);

        let first = &audit.issues[0];
        assert_eq!(first.severity, Severity::Critical);
        assert_eq!(first.rule, "core-h1-present");
        assert_eq!(first.category, "core");
        assert_eq!(first.message, "Missing H1 & heading");
        assert_eq!(first.fix, "Add a single H1 tag");
        assert_eq!(first.details, "found=false");
        assert_eq!(
            first.pages,
            vec!["https://example.com/a", "https://example.com/b"]
        );
        assert_eq!(first.pages_affected, 2);
        assert_eq!(first.pages_total, 2);

        let second = &audit.issues[1];
        assert_eq!(second.severity, Severity::Warning);
        assert!(second.message.contains("Poor content structure"));
    }

    #[test]
    fn rejects_non_audit_xml() {
        assert!(parse_audit(b"<html><body>hi</body></html>").is_err());
    }
}
