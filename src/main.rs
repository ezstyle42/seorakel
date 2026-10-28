//! Seorakel — desktop dashboard for weekly SEO audit XML reports.

use seorakel::app::App;
use seorakel::store::Store;

const APP_ID: &str = "seorakel";

fn main() -> eframe::Result<()> {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id(APP_ID)
            .with_title("Seorakel — SEO Audit Dashboards")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0]),
        persist_window: true,
        ..Default::default()
    };
    eframe::run_native(
        APP_ID,
        options,
        Box::new(|cc| {
            let store = match Store::new(APP_ID) {
                Ok(store) => store,
                Err(err) => {
                    log::error!("failed to initialize data directory: {err:#}");
                    std::process::exit(1);
                }
            };
            let app = match App::new(store) {
                Ok(app) => app,
                Err(err) => {
                    log::error!("failed to load application state: {err:#}");
                    std::process::exit(1);
                }
            };
            let dark = app.settings.dark_mode;
            cc.egui_ctx.set_theme(if dark {
                egui::ThemePreference::Dark
            } else {
                egui::ThemePreference::Light
            });
            Ok(Box::new(SeorakelApp { app }))
        }),
    )
}

struct SeorakelApp {
    app: App,
}

impl eframe::App for SeorakelApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        seorakel::ui::draw(&mut self.app, ui);
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.app.persist_session();
        self.app.save_settings();
    }
}
