use crate::app::{ManimStudio, RenderQualityOpt};
use crate::scene::object_templates;
use egui::{Context, RichText};

pub fn show(app: &mut ManimStudio, ctx: &Context) {
    // ── Menu bar ──────────────────────────────────────────────────────────────
    egui::TopBottomPanel::top("menu_bar")
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(28, 28, 32)).inner_margin(4.0))
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // Logo
                ui.label(RichText::new("🎬 Manim Studio").strong().size(16.0).color(egui::Color32::from_rgb(100, 200, 255)));
                ui.separator();

                // File
                ui.menu_button("File", |ui| {
                    if ui.button("🆕  New Scene").clicked() {
                        app.scene = crate::scene::Scene::new();
                        app.selected_obj = None;
                        ui.close_menu();
                    }
                    if ui.button("📂  Open…").clicked() { ui.close_menu(); }
                    if ui.button("💾  Save…").clicked() { ui.close_menu(); }
                    ui.separator();
                    if ui.button("⎆  Export Python…").clicked() {
                        app.generate_code();
                        app.show_code = true;
                        ui.close_menu();
                    }
                });

                // Edit
                ui.menu_button("Edit", |ui| {
                    if ui.button("🗑  Delete selected  [Del]").clicked() {
                        app.delete_selected();
                        ui.close_menu();
                    }
                    if ui.button("⧉  Duplicate  [Ctrl+D]").clicked() {
                        app.duplicate_selected();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Load demo scene").clicked() {
                        app.scene = crate::scene::Scene::demo();
                        app.selected_obj = None;
                        ui.close_menu();
                    }
                });

                // Add
                ui.menu_button("Add", |ui| {
                    for (icon, label, obj_type) in object_templates() {
                        if ui.button(format!("{}  {}", icon, label)).clicked() {
                            app.add_object(obj_type);
                            ui.close_menu();
                        }
                    }
                });

                // Scene
                ui.menu_button("Scene", |ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut app.scene.name);
                    ui.separator();
                    ui.label("Scene Type:");
                    egui::ComboBox::from_id_source("scene_type_combo")
                        .selected_text(app.scene.scene_type.label())
                        .show_ui(ui, |ui| {
                            for st in crate::scene::SceneType::all() {
                                if ui.selectable_label(app.scene.scene_type == *st, st.label()).clicked() {
                                    app.scene.scene_type = st.clone();
                                    // Auto-set is_3d flag
                                    if *st == crate::scene::SceneType::ThreeDScene {
                                        app.scene.is_3d = true;
                                    }
                                }
                            }
                        });
                    ui.checkbox(&mut app.scene.is_3d, "3D Scene (ThreeDScene)");
                    ui.separator();
                    ui.checkbox(&mut app.scene.require_latex,
                        "Use LaTeX for MathTex (requires MiKTeX/TeX Live)");
                    if !app.scene.require_latex {
                        ui.label(
                            egui::RichText::new("⚠ MathTex will be rendered as Text()")
                                .small()
                                .color(egui::Color32::from_rgb(255, 200, 60)),
                        );
                    }
                    ui.separator();
                    ui.label("Duration (s):");
                    ui.add(egui::Slider::new(&mut app.scene.timeline.duration, 1.0..=120.0));
                    ui.label("FPS:");
                    ui.add(egui::Slider::new(&mut app.scene.timeline.fps, 15.0..=120.0));
                });

                // Render menu
                ui.menu_button("Render", |ui| {
                    ui.label("Quality:");
                    for q in [RenderQualityOpt::Low, RenderQualityOpt::Medium, RenderQualityOpt::High, RenderQualityOpt::Ultra] {
                        ui.radio_value(&mut app.render_quality, q.clone(), q.label());
                    }
                    ui.separator();
                    ui.checkbox(&mut app.render_preview, "Open preview after render");
                    ui.separator();
                    if ui.button("⚙  Render Settings…").clicked() {
                        app.show_render_settings = true;
                        ui.close_menu();
                    }
                    if ui.button("▶  Render Now!").clicked() {
                        app.render_scene();
                        ui.close_menu();
                    }
                });

                // Help
                ui.menu_button("Help", |ui| {
                    if ui.button("ℹ  About").clicked() {
                        app.show_about = true;
                        ui.close_menu();
                    }
                });

                // Right-aligned: render status
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    match &app.render_status.clone() {
                        crate::app::RenderStatus::Idle => {}
                        crate::app::RenderStatus::Done(p) => {
                            ui.label(
                                RichText::new(format!("✅ {}", p))
                                    .color(egui::Color32::from_rgb(80, 220, 100))
                                    .small(),
                            );
                        }
                        crate::app::RenderStatus::Error(_) => {
                            if ui.button(
                                RichText::new("❌ Render failed — click for details")
                                    .color(egui::Color32::from_rgb(255, 100, 100))
                                    .small(),
                            ).clicked() {
                                app.show_error_log = true;
                            }
                        }
                    }
                });
            });
        });

    // ── Toolbar ───────────────────────────────────────────────────────────────
    egui::TopBottomPanel::top("toolbar")
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(36, 36, 42)).inner_margin(egui::Margin { left: 6.0, right: 6.0, top: 4.0, bottom: 4.0 }))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Quick-add common objects
                ui.label("Add:");
                for (icon, label, obj_type) in object_templates().into_iter().take(8) {
                    if ui.button(format!("{} {}", icon, label))
                        .on_hover_text(format!("Add {}", label))
                        .clicked()
                    {
                        app.add_object(obj_type);
                    }
                }

                if ui.button("⬡ More…").clicked() {
                    app.show_add_panel = !app.show_add_panel;
                }

                ui.separator();

                // Playback controls
                if app.is_playing {
                    if ui.button(RichText::new("⏸").size(18.0)).clicked() {
                        app.is_playing = false;
                    }
                } else {
                    if ui.button(RichText::new("▶").size(18.0)).on_hover_text("Play preview").clicked() {
                        app.is_playing = true;
                    }
                }
                if ui.button("⏹").on_hover_text("Stop & rewind").clicked() {
                    app.is_playing = false;
                    app.scene.timeline.current_time = 0.0;
                }

                // Timeline time display
                let cur = app.scene.timeline.current_time;
                let dur = app.scene.timeline.duration;
                ui.label(
                    RichText::new(format!("{:5.2}s / {:.2}s", cur, dur))
                        .monospace()
                        .color(egui::Color32::from_rgb(200, 200, 200)),
                );

                ui.separator();

                // Code / Render buttons
                if ui.button("📄 Code").on_hover_text("Show generated Python").clicked() {
                    app.generate_code();
                    app.show_code = true;
                }
                if ui.button(RichText::new("🚀 Render").color(egui::Color32::from_rgb(120, 220, 120))).clicked() {
                    app.render_scene();
                }

                // Viewport zoom
                ui.separator();
                ui.label("Zoom:");
                ui.add(egui::Slider::new(&mut app.vp_zoom, 20.0..=200.0).text("px/u"));
            });
        });
}