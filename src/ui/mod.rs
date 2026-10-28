//! egui user interface.

mod dashboard;
mod library;
mod settings;
mod widgets;

pub use dashboard::show_dashboard;
pub use library::show_library;
pub use settings::show_settings_dialog;

use std::path::PathBuf;

use crate::app::{App, PendingAction, View};

pub fn draw(app: &mut App, ui: &mut egui::Ui) {
    egui::Panel::top("seorakel_top_bar").show(ui, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.heading("🔮 Seorakel");
            ui.separator();
            let library_btn = ui.selectable_label(app.active_tab.is_none(), "📚 Library");
            if library_btn.clicked() {
                app.active_tab = None;
                app.persist_session();
            }
            if let Some(active) = app.active_tab {
                ui.separator();
                let mut close_tab = false;
                egui::ScrollArea::horizontal()
                    .id_salt("tabs")
                    .max_height(40.0)
                    .show(ui, |ui| {
                        for (i, tab) in app.tabs.iter().enumerate() {
                            let name = app
                                .index
                                .get(&tab.site_id)
                                .map(|e| e.name.clone())
                                .unwrap_or_else(|| tab.site_id.clone());
                            let selected = i == active;
                            let label = if selected {
                                format!("● {name} ✕")
                            } else {
                                format!("{name} ✕")
                            };
                            let btn = ui.selectable_label(selected, label);
                            if btn.clicked() {
                                app.active_tab = Some(i);
                                app.persist_session();
                            }
                            if btn.hovered() && ui.input(|i| i.pointer.secondary_clicked()) {
                                close_tab = true;
                            }
                            if selected
                                && ui.input(|i| i.key_pressed(egui::Key::W) && i.modifiers.ctrl)
                            {
                                close_tab = true;
                            }
                            if close_tab {
                                app.close_tab(i);
                                break;
                            }
                        }
                    });
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⚙ Settings").clicked() {
                    app.show_settings = true;
                }
                if ui.button("📂 Import XML…").clicked() {
                    pick_and_import(app);
                }
            });
        });
        ui.add_space(4.0);
    });

    if let Some(msg) = app.status_message.clone() {
        egui::Panel::bottom("seorakel_status").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(&msg);
                if ui.button("✕").clicked() {
                    app.status_message = None;
                }
            });
        });
    }

    egui::CentralPanel::default().show(ui, |ui| {
        handle_dropped_files(app, ui.ctx());
        if app.show_settings {
            settings::show_settings_dialog(app, ui.ctx());
        }
        if let Some(site_id) = app.confirm_delete_site.clone() {
            confirm_delete_dialog(app, ui.ctx(), &site_id);
        }
        match app.active_tab {
            Some(i) => {
                if i < app.tabs.len() {
                    let mut view = app.tabs[i].view;
                    draw_tab_bar(app, ui, i, &mut view);
                    app.tabs[i].view = view;
                    ui.separator();
                    dashboard::show_dashboard(app, ui, i);
                } else {
                    app.active_tab = None;
                    app.persist_session();
                }
            }
            None => {
                library::show_library(app, ui);
            }
        }
        app.persist_session();
    });

    if let PendingAction::ShowSettings = app.pending_action {
        app.show_settings = true;
        app.pending_action = PendingAction::None;
    }
}

fn draw_tab_bar(app: &mut App, ui: &mut egui::Ui, tab_idx: usize, view: &mut View) {
    ui.horizontal(|ui| {
        for v in View::all() {
            if ui.selectable_label(*view == v, v.label()).clicked() {
                *view = v;
                app.persist_session();
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(tab) = app.tabs.get_mut(tab_idx) {
                let current = tab
                    .selected()
                    .map(|a| format!("📅 {} (score {})", a.date, a.score))
                    .unwrap_or_default();
                let mut selected = tab.selected_report;
                egui::ComboBox::from_id_salt("report_picker")
                    .selected_text(current)
                    .show_ui(ui, |ui| {
                        for (i, report) in tab.reports.iter().enumerate() {
                            let label = format!(
                                "{} — score {} ({})",
                                report.date, report.score, report.grade
                            );
                            ui.selectable_value(&mut selected, i, label);
                        }
                    });
                if selected != tab.selected_report {
                    tab.selected_report = selected;
                }
            }
        });
    });
}

pub(crate) fn pick_and_import(app: &mut App) {
    let start_dir = app
        .settings
        .last_directory
        .as_ref()
        .map(std::path::PathBuf::from);
    let dialog = rfd::FileDialog::new()
        .add_filter("SEO audit XML", &["xml"])
        .set_title("Import SEO audit report");
    let dialog = match start_dir {
        Some(dir) => dialog.set_directory(dir),
        None => dialog,
    };
    if let Some(paths) = dialog.pick_files() {
        if let Some(dir) = paths.first().and_then(|p| p.parent()) {
            app.settings.last_directory = Some(dir.to_string_lossy().into_owned());
            app.save_settings();
        }
        app.import_files(&paths);
    }
}

fn handle_dropped_files(app: &mut App, ctx: &egui::Context) {
    let dropped: Vec<PathBuf> = ctx.input(|i| {
        i.raw
            .dropped_files
            .iter()
            .map(|f| f.path().to_path_buf())
            .collect()
    });
    if !dropped.is_empty() {
        app.import_files(&dropped);
    }

    let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
    if hovering {
        ctx.set_cursor_icon(egui::CursorIcon::Copy);
        egui::Area::new(egui::Id::unique("drop_overlay"))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let rect = ui.ctx().content_rect();
                ui.painter()
                    .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(120));
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Drop XML report files to import",
                    egui::FontId::proportional(28.0),
                    egui::Color32::WHITE,
                );
            });
    }
}

fn confirm_delete_dialog(app: &mut App, ctx: &egui::Context, site_id: &str) {
    let mut open = true;
    egui::Window::new("Delete dashboard")
        .id(egui::Id::unique("confirm_delete"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .open(&mut open)
        .show(ctx, |ui| {
            let name = app
                .index
                .get(site_id)
                .map(|e| e.name.clone())
                .unwrap_or_else(|| site_id.to_string());
            ui.label(format!(
                "Delete the dashboard for {name} and all its stored reports?"
            ));
            ui.label("This cannot be undone.");
            ui.horizontal(|ui| {
                if ui.button("Delete").clicked() {
                    app.delete_site(site_id);
                    app.confirm_delete_site = None;
                }
                if ui.button("Cancel").clicked() {
                    app.confirm_delete_site = None;
                }
            });
        });
    if !open {
        app.confirm_delete_site = None;
    }
}
