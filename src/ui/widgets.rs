//! Small shared widgets.

use egui::{Color32, RichText, Ui};

use crate::model::{grade_color, score_color};

pub fn score_badge(ui: &mut Ui, score: u32) {
    let color = score_color(score);
    let text = RichText::new(format!("{score}"))
        .color(Color32::WHITE)
        .strong();
    let (rect, _response) = ui.allocate_exact_size(egui::vec2(44.0, 28.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 6.0, color);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("{score}"),
        egui::FontId::proportional(16.0),
        Color32::WHITE,
    );
    let _ = text;
}

pub fn grade_badge(ui: &mut Ui, grade: &str) {
    let color = grade_color(grade);
    let (rect, _response) = ui.allocate_exact_size(egui::vec2(36.0, 28.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 6.0, color);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        grade,
        egui::FontId::proportional(15.0),
        Color32::WHITE,
    );
}

pub fn severity_chip(ui: &mut Ui, severity: crate::model::Severity) {
    let (color, label) = match severity {
        crate::model::Severity::Critical => (Color32::from_rgb(218, 54, 51), "critical"),
        crate::model::Severity::Warning => (Color32::from_rgb(187, 128, 9), "warning"),
    };
    let (rect, _response) = ui.allocate_exact_size(egui::vec2(70.0, 20.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 4.0, color);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(11.0),
        Color32::WHITE,
    );
}

pub fn stat_card(ui: &mut Ui, label: &str, value: impl ToString, color: Color32) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(10))
        .corner_radius(egui::CornerRadius::same(8))
        .show(ui, |ui| {
            ui.set_min_width(110.0);
            ui.label(
                RichText::new(label)
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
            ui.label(RichText::new(value.to_string()).heading().color(color));
        });
}

pub fn category_bar(ui: &mut Ui, label: &str, score: u32, max_width: f32) {
    let color = score_color(score);
    let frac = (score as f32 / 100.0).clamp(0.0, 1.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).strong());
        let (rect, _response) =
            ui.allocate_exact_size(egui::vec2(max_width - 60.0, 16.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 4.0, ui.visuals().widgets.inactive.bg_fill);
        let fill = egui::Rect::from_min_size(
            rect.min,
            egui::vec2((rect.width() * frac).max(2.0), rect.height()),
        );
        ui.painter().rect_filled(fill, 4.0, color);
        ui.label(egui::RichText::new(format!("{score}")).monospace());
    });
}
