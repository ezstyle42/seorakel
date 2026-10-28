//! Disk persistence: report library, app settings, and session state.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::model::Audit;
use crate::parse::parse_audit;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteEntry {
    pub url: String,
    pub name: String,
    pub first_seen: String,
    pub last_report: String,
    pub report_count: usize,
    pub latest_score: u32,
    pub latest_grade: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    pub dark_mode: bool,
    #[serde(default = "default_true")]
    pub confirm_delete: bool,
    #[serde(default)]
    pub last_directory: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionState {
    pub open_sites: Vec<String>,
    pub active_site: Option<String>,
    pub active_view: Option<String>,
}

pub struct Store {
    pub root: PathBuf,
}

impl Store {
    pub fn new(app_id: &str) -> Result<Self> {
        let root = if let Some(dir) = directories::ProjectDirs::from("", "", app_id) {
            dir.data_dir().to_path_buf()
        } else if let Some(home) = std::env::var_os("HOME") {
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join(app_id)
        } else {
            PathBuf::from(".").join(app_id)
        };
        let store = Store { root };
        store.ensure_dirs()?;
        Ok(store)
    }

    fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(self.reports_dir())?;
        std::fs::create_dir_all(self.sites_dir())?;
        Ok(())
    }

    pub fn reports_dir(&self) -> PathBuf {
        self.root.join("reports")
    }

    fn sites_dir(&self) -> PathBuf {
        self.root.join("sites")
    }

    fn index_path(&self) -> PathBuf {
        self.root.join("library.json")
    }

    fn settings_path(&self) -> PathBuf {
        self.root.join("settings.json")
    }

    fn session_path(&self) -> PathBuf {
        self.root.join("session.json")
    }

    pub fn site_dir(&self, site_id: &str) -> PathBuf {
        self.sites_dir().join(site_id)
    }

    pub fn report_path(&self, site_id: &str, date: &str) -> PathBuf {
        self.site_dir(site_id).join(format!("{date}.xml"))
    }

    pub fn load_index(&self) -> Result<BTreeMap<String, SiteEntry>> {
        let path = self.index_path();
        if !path.exists() {
            return Ok(BTreeMap::new());
        }
        let bytes = std::fs::read(&path).context("reading library index")?;
        let index = serde_json::from_slice(&bytes).context("parsing library index")?;
        Ok(index)
    }

    fn save_index(&self, index: &BTreeMap<String, SiteEntry>) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(index).context("serializing library index")?;
        atomic_write(&self.index_path(), &bytes)
    }

    pub fn load_settings(&self) -> Settings {
        std::fs::read(self.settings_path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(settings).context("serializing settings")?;
        atomic_write(&self.settings_path(), &bytes)
    }

    pub fn load_session(&self) -> SessionState {
        std::fs::read(self.session_path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save_session(&self, session: &SessionState) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(session).context("serializing session")?;
        atomic_write(&self.session_path(), &bytes)
    }

    pub fn list_reports(&self, site_id: &str) -> Result<Vec<PathBuf>> {
        let dir = self.site_dir(site_id);
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut reports: Vec<PathBuf> = std::fs::read_dir(&dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "xml"))
            .collect();
        reports.sort();
        Ok(reports)
    }

    pub fn load_report(&self, path: &Path) -> Result<Audit> {
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        parse_audit(&bytes).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn import_report(&self, source: &Path) -> Result<(String, Audit)> {
        let audit = self.load_report(source)?;
        let site_id = site_id_for(&audit.url);
        let report_path = self.report_path(&site_id, &audit.date);
        std::fs::create_dir_all(self.site_dir(&site_id))?;
        if !report_path.exists() {
            std::fs::copy(source, &report_path)
                .with_context(|| format!("storing report at {}", report_path.display()))?;
        }
        self.upsert_index(&site_id, &audit)?;
        Ok((site_id, audit))
    }

    fn upsert_index(&self, site_id: &str, audit: &Audit) -> Result<()> {
        let mut index = self.load_index()?;
        let entry = index
            .entry(site_id.to_string())
            .or_insert_with(|| SiteEntry {
                url: audit.url.clone(),
                name: site_name(&audit.url),
                first_seen: audit.date.clone(),
                last_report: audit.date.clone(),
                report_count: 0,
                latest_score: 0,
                latest_grade: String::new(),
            });
        entry.url = audit.url.clone();
        entry.last_report = audit.date.clone();
        entry.latest_score = audit.score;
        entry.latest_grade = audit.grade.clone();
        entry.report_count = self.list_reports(site_id)?.len();
        if audit.date < entry.first_seen {
            entry.first_seen = audit.date.clone();
        }
        self.save_index(&index)
    }

    pub fn delete_site(&self, site_id: &str) -> Result<()> {
        let mut index = self.load_index()?;
        index.remove(site_id);
        self.save_index(&index)?;
        let dir = self.site_dir(site_id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).with_context(|| format!("deleting {}", dir.display()))?;
        }
        Ok(())
    }
}

pub fn site_id_for(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    let sanitized: String = without_scheme
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let sanitized = sanitized.trim_matches('_').to_string();
    if sanitized.is_empty() {
        "site".to_string()
    } else {
        sanitized
    }
}

pub fn site_name(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    without_scheme
        .split('/')
        .next()
        .unwrap_or(without_scheme)
        .to_string()
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_id_sanitizes_urls() {
        assert_eq!(
            site_id_for("https://wonenindebeemden.nl"),
            "wonenindebeemden.nl"
        );
        assert_eq!(site_id_for("https://example.com/"), "example.com");
        assert_eq!(site_id_for("http://a.b/c/d"), "a.b_c_d");
        assert_eq!(site_id_for(""), "site");
    }

    #[test]
    fn site_name_extracts_host() {
        assert_eq!(
            site_name("https://wonenindebeemden.nl"),
            "wonenindebeemden.nl"
        );
        assert_eq!(site_name("https://example.com/path/page"), "example.com");
    }

    #[test]
    fn import_and_list_roundtrip() {
        let dir = std::env::temp_dir().join(format!("seorakel-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store { root: dir.clone() };
        store.ensure_dirs().unwrap();

        let sample = r#"<seo-audit schema="2" url="https://example.com" score="80" grade="B" pages="5" date="2026-10-24" nonce="x">
<summary passed="10" warnings="2" failures="1" notMeasured="0"/>
<categories><cat id="core" score="90" p="8" w="1" f="1"/></categories>
<issues count="1" total="1" omitted="0"><issue severity="warning" rule="core-x" cat="core" pages="1" of="1"><on>https://example.com/a</on><msg><u>m</u></msg><fix>f</fix><details>d</details></issue></issues>
<not-measured></not-measured><passed></passed></seo-audit>"#;
        let src = dir.join("incoming.xml");
        std::fs::write(&src, sample).unwrap();

        let (site_id, audit) = store.import_report(&src).unwrap();
        assert_eq!(site_id, "example.com");
        assert_eq!(audit.score, 80);

        let reports = store.list_reports(&site_id).unwrap();
        assert_eq!(reports.len(), 1);

        let index = store.load_index().unwrap();
        let entry = index.get(&site_id).unwrap();
        assert_eq!(entry.name, "example.com");
        assert_eq!(entry.report_count, 1);
        assert_eq!(entry.latest_score, 80);

        store.delete_site(&site_id).unwrap();
        assert!(store.load_index().unwrap().is_empty());
        assert!(store.list_reports(&site_id).unwrap().is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
