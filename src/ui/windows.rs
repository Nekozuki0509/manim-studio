use crate::app::{ManimStudio, RenderQualityOpt, RenderStatus};
use egui::{Color32, Context, RichText};

pub fn show_code(app: &mut ManimStudio, ctx: &Context) {
    if !app.show_code { return; }

    let mut open = app.show_code;
    egui::Window::new("📄 Generated Manim Code")
        .open(&mut open)
        .min_size([620.0, 440.0])
        .default_size([700.0, 520.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("📋 Copy to Clipboard").clicked() {
                    ui.output_mut(|o| o.copied_text = app.generated_code.clone());
                }
                if ui.button("🔄 Regenerate").clicked() {
                    app.generate_code();
                }
                ui.separator();
                if ui.button(RichText::new("🚀 Render").color(Color32::from_rgb(100, 220, 100))).clicked() {
                    app.render_scene();
                }
            });
            ui.separator();

            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut app.generated_code)
                            .font(egui::FontId::monospace(13.0))
                            .desired_width(f32::INFINITY)
                            .desired_rows(30)
                            .code_editor()
                            .lock_focus(true),
                    );
                });
        });
    app.show_code = open;
}

pub fn show_render_settings(app: &mut ManimStudio, ctx: &Context) {
    if !app.show_render_settings { return; }

    let mut open = app.show_render_settings;
    egui::Window::new("⚙ Render Settings")
        .open(&mut open)
        .resizable(false)
        .min_size([320.0, 300.0])
        .show(ctx, |ui| {
            ui.label(RichText::new("Quality").strong());
            for q in [RenderQualityOpt::Low, RenderQualityOpt::Medium, RenderQualityOpt::High, RenderQualityOpt::Ultra] {
                ui.radio_value(&mut app.render_quality, q.clone(), q.label());
            }

            ui.separator();

            ui.label(RichText::new("Output").strong());
            ui.horizontal(|ui| {
                ui.label("Directory:");
                ui.text_edit_singleline(&mut app.output_dir);
            });
            ui.checkbox(&mut app.render_preview, "Open preview after render");

            ui.separator();

            ui.label(RichText::new("3D Camera").strong());
            if app.scene.is_3d {
                ui.horizontal(|ui| {
                    ui.label("Phi (elevation):");
                    ui.add(egui::Slider::new(&mut app.scene.camera_phi, 0.0..=180.0).suffix("°"));
                });
                ui.horizontal(|ui| {
                    ui.label("Theta (azimuth):");
                    ui.add(egui::Slider::new(&mut app.scene.camera_theta, -360.0..=360.0).suffix("°"));
                });
            } else {
                ui.label(RichText::new("Enable 3D scene in Scene menu to adjust camera.").color(Color32::from_rgb(120, 120, 130)).small());
            }

            ui.separator();
            ui.label(RichText::new("Background Color").strong());
            ui.color_edit_button_rgb(&mut app.scene.bg_color);

            ui.separator();
            match &app.render_status.clone() {
                RenderStatus::Idle => {}
                RenderStatus::Done(p) => {
                    ui.label(RichText::new(format!("✅ Output: {}", p)).color(Color32::from_rgb(100, 220, 100)));
                }
                RenderStatus::Error(e) => {
                    ui.label(RichText::new("❌ Error:").color(Color32::from_rgb(255, 100, 100)));
                    egui::ScrollArea::vertical().max_height(80.0).show(ui, |ui| {
                        ui.label(RichText::new(e).color(Color32::from_rgb(255, 150, 150)).small());
                    });
                }
            }

            if ui.button(RichText::new("🚀 Render Now").color(Color32::from_rgb(100, 220, 100))).clicked() {
                app.render_scene();
            }
        });
    app.show_render_settings = open;
}

pub fn show_error_log(app: &mut ManimStudio, ctx: &Context) {
    if !app.show_error_log { return; }

    let error_text = match &app.render_status {
        crate::app::RenderStatus::Error(e) => e.clone(),
        _ => { app.show_error_log = false; return; }
    };

    let mut open = app.show_error_log;
    egui::Window::new("❌ Render Error")
        .open(&mut open)
        .min_size([560.0, 320.0])
        .default_size([700.0, 420.0])
        .show(ctx, |ui| {
            ui.label(RichText::new("Manim failed to render. Full error log:").color(Color32::from_rgb(255, 150, 150)));
            ui.separator();

            // Copyable text area
            let mut text = error_text.clone();
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .max_height(280.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut text)
                            .font(egui::FontId::monospace(12.0))
                            .desired_width(f32::INFINITY)
                            .desired_rows(16)
                            .text_color(Color32::from_rgb(255, 180, 180)),
                    );
                });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("📋 Copy to Clipboard").clicked() {
                    ui.output_mut(|o| o.copied_text = error_text.clone());
                }
                ui.separator();
                ui.label(
                    RichText::new("Tip: Make sure `python -m manim --version` works in your terminal.")
                        .small()
                        .color(Color32::from_rgb(160, 160, 180)),
                );
            });
        });
    app.show_error_log = open;
}

pub fn show_about(app: &mut ManimStudio, ctx: &Context) {

    let mut open = app.show_about;
    egui::Window::new("ℹ About Manim Studio")
        .open(&mut open)
        .resizable(false)
        .min_size([380.0, 250.0])
        .show(ctx, |ui| {
            ui.label(RichText::new("🎬 Manim Studio").heading().strong().color(Color32::from_rgb(100, 200, 255)));
            ui.label(RichText::new("v0.1.0").small().color(Color32::from_rgb(140, 140, 160)));
            ui.separator();
            ui.label("A Blender/Premiere-style GUI for creating Manim animations.");
            ui.label("Built in Rust using egui.");
            ui.separator();
            ui.label(RichText::new("Requirements").strong());
            ui.label("• Python 3.8+");
            ui.label("• pip install manim");
            ui.label("• LaTeX (for MathTex objects)");
            ui.separator();
            ui.label(RichText::new("Keyboard Shortcuts").strong());
            for (key, desc) in [
                ("Space",        "Play / Pause"),
                ("Home",         "Rewind to start"),
                ("Del",          "Delete selected"),
                ("Ctrl+Z",       "Undo"),
                ("Ctrl+Y / Ctrl+Shift+Z", "Redo"),
                ("Ctrl+C",       "Copy selected object"),
                ("Ctrl+X",       "Cut selected object"),
                ("Ctrl+V",       "Paste object"),
                ("Ctrl+D",       "Duplicate selected"),
                ("Scroll",       "Zoom viewport / scroll timeline"),
                ("Drag (Select)","Move selected object"),
                ("Drag (Pan)",   "Pan viewport"),
                ("Click ruler",  "Seek to time"),
            ] {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(key).monospace().color(Color32::from_rgb(200, 200, 100)));
                    ui.label(desc);
                });
            }
        });
    app.show_about = open;
}