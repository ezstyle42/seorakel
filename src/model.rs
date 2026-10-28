//! Domain model for a parsed SEO audit XML file.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Audit {
    pub url: String,
    pub date: String,
    pub score: u32,
    pub grade: String,
    pub pages: u32,
    pub summary: Summary,
    pub categories: Vec<Category>,
    pub issues: Vec<Issue>,
    pub issues_total: u32,
    pub issues_omitted: u32,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Summary {
    pub passed: u32,
    pub warnings: u32,
    pub failures: u32,
    pub not_measured: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub score: u32,
    pub passed: u32,
    pub warnings: u32,
    pub failures: u32,
    pub not_measured: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    Warning,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Severity::Critical => "Critical",
            Severity::Warning => "Warning",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub severity: Severity,
    pub rule: String,
    pub category: String,
    pub message: String,
    pub fix: String,
    pub details: String,
    pub pages: Vec<String>,
    pub pages_affected: u32,
    pub pages_total: u32,
}

impl Issue {
    pub fn pages_display(&self) -> String {
        if self.pages.is_empty() {
            format!("{}/{}", self.pages_affected, self.pages_total)
        } else {
            format!(
                "{} ({}/{})",
                self.pages.len(),
                self.pages_affected,
                self.pages_total
            )
        }
    }
}

pub fn category_label(id: &str) -> &str {
    match id {
        "core" => "Core SEO",
        "technical" => "Technical",
        "perf" => "Performance",
        "links" => "Links",
        "images" => "Images",
        "security" => "Security",
        "crawl" => "Crawlability",
        "schema" => "Structured Data",
        "a11y" => "Accessibility",
        "content" => "Content",
        "social" => "Social Media",
        "eeat" => "E-E-A-T",
        "url" => "URLs",
        "mobile" => "Mobile",
        "i18n" => "Internationalization",
        "legal" => "Legal",
        "js" => "JavaScript",
        "redirect" => "Redirects",
        "htmlval" => "HTML Validation",
        "geo" => "GEO / AI Search",
        other => other,
    }
}

pub fn score_color(score: u32) -> egui::Color32 {
    if score >= 90 {
        egui::Color32::from_rgb(46, 160, 67)
    } else if score >= 70 {
        egui::Color32::from_rgb(187, 128, 9)
    } else if score >= 50 {
        egui::Color32::from_rgb(207, 106, 30)
    } else {
        egui::Color32::from_rgb(218, 54, 51)
    }
}

pub fn grade_color(grade: &str) -> egui::Color32 {
    match grade {
        "A" | "A+" | "A-" => egui::Color32::from_rgb(46, 160, 67),
        "B" | "B+" | "B-" => egui::Color32::from_rgb(120, 180, 60),
        "C" | "C+" | "C-" => egui::Color32::from_rgb(187, 128, 9),
        "D" | "D+" | "D-" => egui::Color32::from_rgb(207, 106, 30),
        _ => egui::Color32::from_rgb(218, 54, 51),
    }
}
