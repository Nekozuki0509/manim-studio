mod app;
mod scene;
mod codegen;
mod renderer;
mod ui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_title("Manim Studio")
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Manim Studio",
        native_options,
        Box::new(|_cc| Box::new(app::ManimStudio::new())),
    )
}
