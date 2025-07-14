mod client_switch;
mod hutao_config;
mod hutao_launcher;
mod process_utils;
mod widget_test;
use crate::hutao_launcher::Launcher as App;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_maximize_button(false)
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "GI-Toolkit",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}
