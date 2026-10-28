//! Application state and tab management.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::model::Audit;
use crate::store::{SessionState, Settings, SiteEntry, Store};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Overview,
    Categories,
    Issues,
    History,
}

impl View {
    pub fn label(self) -> &'static str {
        match self {
            View::Overview => "Overview",
            View::Categories => "Categories",
            View::Issues => "Issues",
            View::History => "History",
        }
    }

    pub fn all() -> [View; 4] {
        [
            View::Overview,
            View::Categories,
            View::Issues,
            View::History,
        ]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            View::Overview => "overview",
            View::Categories => "categories",
            View::Issues => "issues",
            View::History => "history",
        }
    }

    pub fn parse(s: &str) -> View {
        match s {
            "categories" => View::Categories,
            "issues" => View::Issues,
            "history" => View::History,
            _ => View::Overview,
        }
    }
}

pub struct SiteTab {
    pub site_id: String,
    pub reports: Vec<Audit>,
    pub selected_report: usize,
    pub view: View,
    pub issue_filter_cat: Option<String>,
    pub issue_filter_severity: Option<crate::model::Severity>,
    pub issue_search: String,
}

impl SiteTab {
    pub fn latest(&self) -> Option<&Audit> {
        self.reports.last()
    }

    pub fn latest_mut(&mut self) -> Option<&mut Audit> {
        self.reports.last_mut()
    }

    pub fn select_date(&mut self, date: &str) {
        if let Some(pos) = self.reports.iter().position(|r| r.date == date) {
            self.selected_report = pos;
        }
    }

    pub fn selected(&self) -> Option<&Audit> {
        self.reports.get(self.selected_report)
    }
}

pub enum PendingAction {
    OpenSite(String),
    DeleteSite(String),
    ShowSettings,
    None,
}

pub struct App {
    pub store: Store,
    pub index: BTreeMap<String, SiteEntry>,
    pub tabs: Vec<SiteTab>,
    pub active_tab: Option<usize>,
    pub settings: Settings,
    pub show_settings: bool,
    pub status_message: Option<String>,
    pub pending_action: PendingAction,
    pub library_search: String,
    pub confirm_delete_site: Option<String>,
}

impl App {
    pub fn new(store: Store) -> anyhow::Result<Self> {
        let index = store.load_index().unwrap_or_default();
        let settings = store.load_settings();
        let session = store.load_session();

        let mut tabs = Vec::new();
        for site_id in &session.open_sites {
            if let Ok(tab) = load_tab(&store, site_id) {
                tabs.push(tab);
            }
        }
        let active_tab = session
            .active_site
            .as_deref()
            .and_then(|id| tabs.iter().position(|t| t.site_id == id))
            .or(if tabs.is_empty() { None } else { Some(0) });
        if let (Some(active), Some(view)) = (active_tab, &session.active_view) {
            tabs[active].view = View::parse(view);
        }

        Ok(Self {
            store,
            index,
            tabs,
            active_tab,
            settings,
            show_settings: false,
            status_message: None,
            pending_action: PendingAction::None,
            library_search: String::new(),
            confirm_delete_site: None,
        })
    }

    pub fn refresh_index(&mut self) {
        self.index = self.store.load_index().unwrap_or_default();
    }

    pub fn import_files(&mut self, paths: &[PathBuf]) {
        for path in paths {
            match self.store.import_report(path) {
                Ok((site_id, _audit)) => {
                    self.refresh_index();
                    self.status_message = Some(format!("Imported report for {}", path.display()));
                    if !self.tabs.iter().any(|t| t.site_id == site_id) {
                        if let Ok(tab) = load_tab(&self.store, &site_id) {
                            self.tabs.push(tab);
                            self.active_tab = Some(self.tabs.len() - 1);
                        }
                    } else if let Some(pos) = self.tabs.iter().position(|t| t.site_id == site_id) {
                        if let Ok(tab) = load_tab(&self.store, &site_id) {
                            self.tabs[pos] = tab;
                        }
                        self.active_tab = Some(pos);
                    }
                }
                Err(err) => {
                    self.status_message =
                        Some(format!("Failed to import {}: {err:#}", path.display()));
                }
            }
        }
        self.persist_session();
    }

    pub fn open_site(&mut self, site_id: &str) {
        if self.tabs.iter().any(|t| t.site_id == site_id) {
            self.active_tab = self.tabs.iter().position(|t| t.site_id == site_id);
            return;
        }
        match load_tab(&self.store, site_id) {
            Ok(tab) => {
                self.tabs.push(tab);
                self.active_tab = Some(self.tabs.len() - 1);
            }
            Err(err) => {
                self.status_message = Some(format!("Failed to open site: {err:#}"));
            }
        }
        self.persist_session();
    }

    pub fn close_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.tabs.remove(index);
            self.active_tab = match self.active_tab {
                Some(active) if active == index => {
                    if self.tabs.is_empty() {
                        None
                    } else {
                        Some(active.min(self.tabs.len() - 1))
                    }
                }
                Some(active) if active > index => Some(active - 1),
                other => other,
            };
            self.persist_session();
        }
    }

    pub fn delete_site(&mut self, site_id: &str) {
        if let Err(err) = self.store.delete_site(site_id) {
            self.status_message = Some(format!("Failed to delete site: {err:#}"));
            return;
        }
        self.tabs.retain(|t| t.site_id != site_id);
        self.active_tab = match self.active_tab {
            Some(_) if self.tabs.is_empty() => None,
            Some(active) if active >= self.tabs.len() => Some(self.tabs.len() - 1),
            other => other,
        };
        self.refresh_index();
        self.persist_session();
        self.status_message = Some(format!("Deleted dashboard for {site_id}"));
    }

    pub fn save_settings(&mut self) {
        if let Err(err) = self.store.save_settings(&self.settings) {
            self.status_message = Some(format!("Failed to save settings: {err:#}"));
        }
    }

    pub fn persist_session(&self) {
        let session = SessionState {
            open_sites: self.tabs.iter().map(|t| t.site_id.clone()).collect(),
            active_site: self
                .active_tab
                .and_then(|i| self.tabs.get(i))
                .map(|t| t.site_id.clone()),
            active_view: self
                .active_tab
                .and_then(|i| self.tabs.get(i))
                .map(|t| t.view.as_str().to_string()),
        };
        let _ = self.store.save_session(&session);
    }

    pub fn active_tab(&self) -> Option<&SiteTab> {
        self.active_tab.and_then(|i| self.tabs.get(i))
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut SiteTab> {
        self.active_tab.and_then(|i| self.tabs.get_mut(i))
    }
}

fn load_tab(store: &Store, site_id: &str) -> anyhow::Result<SiteTab> {
    let mut reports = Vec::new();
    for path in store.list_reports(site_id)? {
        reports.push(store.load_report(&path)?);
    }
    reports.sort_by(|a, b| a.date.cmp(&b.date));
    if reports.is_empty() {
        anyhow::bail!("no reports found for {site_id}");
    }
    let selected_report = reports.len() - 1;
    Ok(SiteTab {
        site_id: site_id.to_string(),
        reports,
        selected_report,
        view: View::Overview,
        issue_filter_cat: None,
        issue_filter_severity: None,
        issue_search: String::new(),
    })
}
