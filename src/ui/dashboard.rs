//! Per-site dashboard: overview, categories, issues, history.

use egui::{RichText, Ui};

use crate::app::{App, SiteTab, View};
use crate::model::{category_label, score_color, Audit, Severity};
use crate::ui::widgets;

pub fn show_dashboard(app: &mut App, ui: &mut Ui, tab_idx: usize) {
    let view = app.tabs[tab_idx].view;
    match view {
        View::Overview => {
            let tab = &mut app.tabs[tab_idx];
            show_overview(ui, tab)
        }
        View::Categories => {
            let tab = &mut app.tabs[tab_idx];
            show_categories(ui, tab)
        }
        View::Issues => show_issues(ui, tab_idx, &mut app.tabs[tab_idx]),
        View::History => {
            let tab = &mut app.tabs[tab_idx];
            show_history(ui, tab)
        }
    }
}

fn show_overview(ui: &mut Ui, tab: &mut SiteTab) {
    let Some(audit) = tab.selected().cloned() else {
        ui.label("No report selected.");
        return;
    };
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&audit.url).heading());
                    widgets::grade_badge(ui, &audit.grade);
                });
                ui.label(
                    RichText::new(format!(
                        "Audit of {} · {} pages crawled",
                        audit.date, audit.pages
                    ))
                    .weak(),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                widgets::score_badge(ui, audit.score);
            });
        });
        ui.add_space(12.0);

        ui.horizontal_wrapped(|ui| {
            widgets::stat_card(
                ui,
                "Passed",
                audit.summary.passed,
                egui::Color32::from_rgb(46, 160, 67),
            );
            widgets::stat_card(
                ui,
                "Warnings",
                audit.summary.warnings,
                egui::Color32::from_rgb(187, 128, 9),
            );
            widgets::stat_card(
                ui,
                "Failures",
                audit.summary.failures,
                egui::Color32::from_rgb(218, 54, 51),
            );
            widgets::stat_card(
                ui,
                "Not measured",
                audit.summary.not_measured,
                ui.visuals().weak_text_color(),
            );
            let crit = audit
                .issues
                .iter()
                .filter(|i| i.severity == Severity::Critical)
                .count() as u32;
            let warn = audit.issues.len() as u32 - crit;
            widgets::stat_card(
                ui,
                "Critical issues",
                crit,
                egui::Color32::from_rgb(218, 54, 51),
            );
            widgets::stat_card(ui, "Warnings", warn, egui::Color32::from_rgb(187, 128, 9));
        });
        ui.add_space(12.0);

        ui.label(RichText::new("Category scores").heading());
        ui.add_space(4.0);
        let max_bar = ui.available_width() * 0.6;
        let mut cats = audit.categories.clone();
        cats.sort_by_key(|c| std::cmp::Reverse(c.score));
        egui::Grid::new("overview_cats")
            .num_columns(2)
            .spacing([24.0, 6.0])
            .show(ui, |ui| {
                for cat in &cats {
                    widgets::category_bar(ui, category_label(&cat.id), cat.score, max_bar);
                    ui.end_row();
                }
            });
        ui.add_space(12.0);

        if tab.reports.len() > 1 {
            ui.label(RichText::new("Trend").heading());
            ui.add_space(4.0);
            show_score_chart(ui, &tab.reports, 220.0);
            ui.label(
                RichText::new("See the History tab for trends per category and per issue.")
                    .small()
                    .weak(),
            );
        }
        ui.add_space(12.0);

        ui.label(RichText::new("Top issues by severity").heading());
        ui.add_space(4.0);
        let top: Vec<_> = audit
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Critical)
            .take(5)
            .collect();
        if top.is_empty() {
            ui.label("No critical issues 🎉");
        }
        for issue in top {
            issue_row(ui, issue);
        }
    });
}

fn show_categories(ui: &mut Ui, tab: &mut SiteTab) {
    let Some(audit) = tab.selected().cloned() else {
        ui.label("No report selected.");
        return;
    };
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(8.0);
        ui.label(RichText::new(format!("Categories — {}", audit.date)).heading());
        ui.add_space(8.0);
        let max_bar = ui.available_width() - 120.0;
        let mut cats = audit.categories.clone();
        cats.sort_by(|a, b| a.id.cmp(&b.id));
        for cat in &cats {
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::same(10))
                .corner_radius(egui::CornerRadius::same(8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.set_min_width(170.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(category_label(&cat.id)).strong());
                            ui.label(
                                RichText::new(cat.id.clone())
                                    .small()
                                    .color(ui.visuals().weak_text_color()),
                            );
                        });
                        ui.vertical(|ui| {
                            widgets::category_bar(ui, "", cat.score, max_bar - 180.0);
                            ui.label(
                                RichText::new(format!(
                                    "{} passed · {} warnings · {} failures{}{}",
                                    cat.passed,
                                    cat.warnings,
                                    cat.failures,
                                    if cat.not_measured > 0 {
                                        format!(" · {} not measured", cat.not_measured)
                                    } else {
                                        String::new()
                                    },
                                    cat.issues_note(&audit)
                                ))
                                .small()
                                .weak(),
                            );
                        });
                    });
                });
            ui.add_space(6.0);
        }
    });
}

trait CatExt {
    fn issues_note(&self, audit: &Audit) -> String;
}

impl CatExt for crate::model::Category {
    fn issues_note(&self, audit: &Audit) -> String {
        let crit = audit
            .issues
            .iter()
            .filter(|i| i.category == self.id && i.severity == Severity::Critical)
            .count();
        let warn = audit
            .issues
            .iter()
            .filter(|i| i.category == self.id && i.severity == Severity::Warning)
            .count();
        format!(" · issues: {crit} critical, {warn} warning")
    }
}

fn show_issues(ui: &mut Ui, _tab_idx: usize, tab: &mut SiteTab) {
    let Some(audit) = tab.selected().cloned() else {
        ui.label("No report selected.");
        return;
    };
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Issues — {}", audit.date)).heading());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut tab.issue_search)
                    .hint_text("Search issues…")
                    .desired_width(200.0),
            );
            let cats: Vec<String> = {
                let mut v: Vec<String> = audit
                    .issues
                    .iter()
                    .map(|i| i.category.clone())
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect();
                v.sort();
                v
            };
            egui::ComboBox::from_id_salt("issue_cat_filter")
                .selected_text(
                    tab.issue_filter_cat
                        .as_deref()
                        .map(category_label)
                        .unwrap_or("All categories"),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut tab.issue_filter_cat, None, "All categories");
                    for cat in cats {
                        ui.selectable_value(
                            &mut tab.issue_filter_cat,
                            Some(cat.clone()),
                            category_label(&cat),
                        );
                    }
                });
            egui::ComboBox::from_id_salt("issue_sev_filter")
                .selected_text(match tab.issue_filter_severity {
                    None => "All severities",
                    Some(Severity::Critical) => "Critical",
                    Some(Severity::Warning) => "Warning",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut tab.issue_filter_severity, None, "All severities");
                    ui.selectable_value(
                        &mut tab.issue_filter_severity,
                        Some(Severity::Critical),
                        "Critical",
                    );
                    ui.selectable_value(
                        &mut tab.issue_filter_severity,
                        Some(Severity::Warning),
                        "Warning",
                    );
                });
        });
    });
    if audit.issues_total > audit.issues.len() as u32 {
        ui.label(
            RichText::new(format!(
                "Showing {} of {} issues ({} lowest-impact omitted by the audit)",
                audit.issues.len(),
                audit.issues_total,
                audit.issues_omitted
            ))
            .small()
            .weak(),
        );
    }
    ui.add_space(6.0);

    let search = tab.issue_search.to_lowercase();
    let filtered: Vec<&crate::model::Issue> = audit
        .issues
        .iter()
        .filter(|i| {
            tab.issue_filter_cat
                .as_deref()
                .is_none_or(|c| i.category == c)
        })
        .filter(|i| tab.issue_filter_severity.is_none_or(|s| i.severity == s))
        .filter(|i| {
            search.is_empty()
                || i.message.to_lowercase().contains(&search)
                || i.rule.to_lowercase().contains(&search)
                || i.fix.to_lowercase().contains(&search)
        })
        .collect();

    ui.label(
        RichText::new(format!("{} issues", filtered.len()))
            .small()
            .weak(),
    );
    ui.add_space(4.0);

    let expanded_id: Option<String> = ui.data(|d| d.get_temp(egui::Id::unique("expanded_issue")));
    egui::ScrollArea::vertical().show(ui, |ui| {
        for issue in filtered {
            let id = format!("{}:{}", issue.category, issue.rule);
            let expanded = expanded_id.as_deref() == Some(id.as_str());
            let _ = &expanded_id;
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::same(10))
                .corner_radius(egui::CornerRadius::same(8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        widgets::severity_chip(ui, issue.severity);
                        ui.label(
                            RichText::new(category_label(&issue.category))
                                .small()
                                .weak(),
                        );
                        ui.label(RichText::new(&issue.rule).monospace().small());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("pages: {}", issue.pages_display()))
                                    .small()
                                    .weak(),
                            );
                            let btn =
                                ui.selectable_label(expanded, if expanded { "▲" } else { "▼" });
                            if btn.clicked() {
                                let new = if expanded { None } else { Some(id.clone()) };
                                ui.data_mut(|d| {
                                    d.insert_temp(egui::Id::unique("expanded_issue"), new)
                                });
                            }
                        });
                    });
                    ui.label(RichText::new(&issue.message).strong());
                    if expanded {
                        ui.add_space(4.0);
                        ui.label(RichText::new("How to fix").small().color(score_color(90)));
                        ui.label(&issue.fix);
                        if !issue.details.is_empty() {
                            ui.add_space(4.0);
                            ui.label(RichText::new("Details").small().weak());
                            ui.label(
                                RichText::new(&issue.details)
                                    .small()
                                    .color(ui.visuals().weak_text_color()),
                            );
                        }
                        if !issue.pages.is_empty() {
                            ui.add_space(4.0);
                            ui.label(RichText::new("Affected pages").small().weak());
                            egui::ScrollArea::vertical()
                                .id_salt("issue_pages")
                                .max_height(140.0)
                                .show(ui, |ui| {
                                    for page in &issue.pages {
                                        ui.label(RichText::new(page).small().monospace());
                                    }
                                });
                        }
                    }
                });
            ui.add_space(6.0);
        }
    });
}

fn show_history(ui: &mut Ui, tab: &mut SiteTab) {
    if tab.reports.len() < 2 {
        ui.add_space(20.0);
        ui.label(RichText::new("History needs at least two weekly reports for this site.").weak());
        ui.label("Import next week's XML file to start tracking changes over time.");
        return;
    }
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(8.0);
        ui.label(RichText::new("Score over time").heading());
        show_score_chart(ui, &tab.reports, 260.0);
        ui.add_space(16.0);

        ui.label(RichText::new("Checks over time (passed / warnings / failures)").heading());
        show_summary_chart(ui, &tab.reports, 260.0);
        ui.add_space(16.0);

        ui.label(RichText::new("Category scores over time").heading());
        let mut cat_ids: Vec<String> = tab
            .reports
            .last()
            .map(|a| a.categories.iter().map(|c| c.id.clone()).collect())
            .unwrap_or_default();
        cat_ids.sort();
        let palette = palette();
        for cat_id in &cat_ids {
            let points: Vec<[f64; 2]> = tab
                .reports
                .iter()
                .enumerate()
                .filter_map(|(i, a)| {
                    a.categories
                        .iter()
                        .find(|c| c.id == *cat_id)
                        .map(|c| [i as f64, c.score as f64])
                })
                .collect();
            if points.len() < 2 {
                continue;
            }
            ui.label(RichText::new(category_label(cat_id)).strong());
            let line = egui_plot::Line::new(
                category_label(cat_id),
                egui_plot::PlotPoints::from(points.clone()),
            )
            .color(palette[cat_ids.iter().position(|c| c == cat_id).unwrap_or(0) % palette.len()]);
            egui_plot::Plot::new(format!("cat_{cat_id}"))
                .height(140.0)
                .include_y(0)
                .include_y(100)
                .x_axis_label("report")
                .y_axis_label("score")
                .legend(egui_plot::Legend::default().position(egui_plot::Corner::RightTop))
                .show(ui, |plot_ui| {
                    plot_ui.line(line);
                    for (i, report) in tab.reports.iter().enumerate() {
                        if let Some(c) = report.categories.iter().find(|c| c.id == *cat_id) {
                            plot_ui.points(egui_plot::Points::new(
                                &report.date,
                                vec![[i as f64, c.score as f64]],
                            ));
                        }
                    }
                });
            ui.add_space(8.0);
        }

        ui.add_space(8.0);
        ui.label(RichText::new("Issue count over time").heading());
        show_issue_count_chart(ui, &tab.reports, 220.0);
    });
}

fn x_labels(reports: &[Audit]) -> Vec<String> {
    reports.iter().map(|r| r.date.clone()).collect()
}

fn show_score_chart(ui: &mut Ui, reports: &[Audit], height: f32) {
    let labels = x_labels(reports);
    let points: Vec<[f64; 2]> = reports
        .iter()
        .enumerate()
        .map(|(i, a)| [i as f64, a.score as f64])
        .collect();
    let line = egui_plot::Line::new("Overall score", egui_plot::PlotPoints::from(points.clone()))
        .color(egui::Color32::from_rgb(91, 141, 239))
        .width(2.5);
    egui_plot::Plot::new("score_chart")
        .height(height)
        .include_y(0)
        .include_y(100)
        .x_axis_formatter(move |mark, _range| {
            let idx = mark.value.round() as usize;
            labels.get(idx).cloned().unwrap_or_default()
        })
        .y_axis_label("score")
        .legend(egui_plot::Legend::default().position(egui_plot::Corner::LeftTop))
        .show(ui, |plot_ui| {
            plot_ui.line(line);
            plot_ui.points(
                egui_plot::Points::new("score", points)
                    .color(egui::Color32::from_rgb(91, 141, 239))
                    .radius(5.0),
            );
        });
}

fn show_summary_chart(ui: &mut Ui, reports: &[Audit], height: f32) {
    let labels = x_labels(reports);
    type SeriesDef = (&'static str, egui::Color32, fn(&Audit) -> u32);
    let series: [SeriesDef; 4] = [
        (
            "Passed",
            egui::Color32::from_rgb(46, 160, 67),
            |a: &Audit| a.summary.passed,
        ),
        (
            "Warnings",
            egui::Color32::from_rgb(187, 128, 9),
            |a: &Audit| a.summary.warnings,
        ),
        (
            "Failures",
            egui::Color32::from_rgb(218, 54, 51),
            |a: &Audit| a.summary.failures,
        ),
        (
            "Not measured",
            egui::Color32::from_rgb(150, 150, 150),
            |a: &Audit| a.summary.not_measured,
        ),
    ];
    egui_plot::Plot::new("summary_chart")
        .height(height)
        .x_axis_formatter(move |mark, _range| {
            let idx = mark.value.round() as usize;
            labels.get(idx).cloned().unwrap_or_default()
        })
        .legend(egui_plot::Legend::default().position(egui_plot::Corner::RightTop))
        .show(ui, |plot_ui| {
            for (name, color, f) in &series {
                let points: Vec<[f64; 2]> = reports
                    .iter()
                    .enumerate()
                    .map(|(i, a)| [i as f64, f(a) as f64])
                    .collect();
                plot_ui.line(
                    egui_plot::Line::new(*name, egui_plot::PlotPoints::from(points)).color(*color),
                );
            }
        });
}

fn show_issue_count_chart(ui: &mut Ui, reports: &[Audit], height: f32) {
    let labels = x_labels(reports);
    let crit: Vec<[f64; 2]> = reports
        .iter()
        .enumerate()
        .map(|(i, a)| {
            [
                i as f64,
                a.issues
                    .iter()
                    .filter(|x| x.severity == Severity::Critical)
                    .count() as f64,
            ]
        })
        .collect();
    let warn: Vec<[f64; 2]> = reports
        .iter()
        .enumerate()
        .map(|(i, a)| {
            [
                i as f64,
                a.issues
                    .iter()
                    .filter(|x| x.severity == Severity::Warning)
                    .count() as f64,
            ]
        })
        .collect();
    egui_plot::Plot::new("issue_chart")
        .height(height)
        .x_axis_formatter(move |mark, _range| {
            let idx = mark.value.round() as usize;
            labels.get(idx).cloned().unwrap_or_default()
        })
        .legend(egui_plot::Legend::default().position(egui_plot::Corner::LeftTop))
        .show(ui, |plot_ui| {
            plot_ui.line(
                egui_plot::Line::new("Critical", egui_plot::PlotPoints::from(crit))
                    .color(egui::Color32::from_rgb(218, 54, 51))
                    .width(2.0),
            );
            plot_ui.line(
                egui_plot::Line::new("Warning", egui_plot::PlotPoints::from(warn))
                    .color(egui::Color32::from_rgb(187, 128, 9))
                    .width(2.0),
            );
        });
}

fn palette() -> Vec<egui::Color32> {
    vec![
        egui::Color32::from_rgb(91, 141, 239),
        egui::Color32::from_rgb(46, 160, 67),
        egui::Color32::from_rgb(218, 54, 51),
        egui::Color32::from_rgb(187, 128, 9),
        egui::Color32::from_rgb(156, 101, 200),
        egui::Color32::from_rgb(0, 150, 160),
        egui::Color32::from_rgb(230, 126, 34),
        egui::Color32::from_rgb(120, 120, 120),
        egui::Color32::from_rgb(219, 68, 155),
        egui::Color32::from_rgb(75, 192, 130),
    ]
}

fn issue_row(ui: &mut Ui, issue: &crate::model::Issue) {
    ui.horizontal(|ui| {
        widgets::severity_chip(ui, issue.severity);
        ui.label(
            RichText::new(category_label(&issue.category))
                .small()
                .weak(),
        );
        ui.label(&issue.message);
    });
}
