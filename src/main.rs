mod client_switch;
mod config;
mod launcher;
mod widget_test;
mod injector;
mod bilibili_dll;
use crate::launcher::Launcher as App;

// TODO: delete this
// All Comments and UI Tests are in English!

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "GI-Toolkit",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}
