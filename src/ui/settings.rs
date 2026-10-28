//! Settings dialog.

use crate::app::App;

pub fn show_settings_dialog(app: &mut App, ctx: &egui::Context) {
    let mut open = app.show_settings;
    let mut dark = app.settings.dark_mode;
    let mut confirm = app.settings.confirm_delete;
    let mut close_clicked = false;
    egui::Window::new("Settings")
        .id(egui::Id::unique("seorakel_settings"))
        .collapsible(false)
        .resizable(false)
        .default_width(360.0)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.checkbox(&mut dark, "Dark mode");
            ui.checkbox(
                &mut confirm,
                "Ask for confirmation before deleting dashboards",
            );
            ui.separator();
            ui.label(format!("Data folder: {}", app.store.root.display()));
            ui.horizontal(|ui| {
                if ui.button("Open data folder").clicked() {
                    let _ = open::that(&app.store.root);
                }
            });
            ui.separator();
            if ui.button("Close").clicked() {
                close_clicked = true;
            }
        });
    if close_clicked {
        open = false;
    }
    app.show_settings = open;
    if dark != app.settings.dark_mode {
        app.settings.dark_mode = dark;
        app.save_settings();
        ctx.set_theme(if dark {
            egui::ThemePreference::Dark
        } else {
            egui::ThemePreference::Light
        });
    }
    if confirm != app.settings.confirm_delete {
        app.settings.confirm_delete = confirm;
        app.save_settings();
    }
}
