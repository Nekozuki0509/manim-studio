use crate::app::ManimStudio;
use crate::scene::{object_templates, AnimEntry, AnimType};
use egui::{Color32, Context, RichText};

pub fn show(app: &mut ManimStudio, ctx: &Context) {
    egui::SidePanel::left("objects_panel")
        .min_width(190.0)
        .max_width(280.0)
        .resizable(true)
        .frame(egui::Frame::none().fill(Color32::from_rgb(24, 24, 30)).inner_margin(6.0))
        .show(ctx, |ui| {
            // ── Header ─────────────────────────────────────────────────────────
            ui.label(
                RichText::new("🎭 Scene Objects")
                    .strong()
                    .color(Color32::from_rgb(180, 180, 220)),
            );
            ui.separator();

            // ── Object list ───────────────────────────────────────────────────
            egui::ScrollArea::vertical()
                .id_source("obj_list_scroll")
                .max_height(300.0)
                .show(ui, |ui| {
                    let ids: Vec<String> = app.scene.objects.iter().map(|o| o.id.clone()).collect();
                    let mut to_delete: Option<String> = None;
                    let mut to_duplicate: Option<String> = None;
                    let mut to_toggle_vis: Option<String> = None;

                    for id in &ids {
                        let obj = match app.scene.get_object(id) {
                            Some(o) => o,
                            None => continue,
                        };

                        let is_sel = app.selected_obj.as_deref() == Some(id);
                        let obj_name = obj.name.clone();
                        let obj_icon = obj.icon();
                        let obj_visible = obj.visible;

                        let response = ui.selectable_label(
                            is_sel,
                            RichText::new(format!("{} {}", obj_icon, &obj_name))
                                .color(if obj_visible {
                                    Color32::from_rgb(210, 210, 230)
                                } else {
                                    Color32::from_rgb(100, 100, 120)
                                }),
                        );

                        if response.clicked() {
                            app.selected_obj = if is_sel { None } else { Some(id.clone()) };
                        }

                        response.context_menu(|ui| {
                            if ui.button("🗑 Delete").clicked() {
                                to_delete = Some(id.clone());
                                ui.close_menu();
                            }
                            if ui.button("⧉ Duplicate").clicked() {
                                to_duplicate = Some(id.clone());
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button(if obj_visible { "👁 Hide" } else { "👁 Show" }).clicked() {
                                to_toggle_vis = Some(id.clone());
                                ui.close_menu();
                            }
                            ui.separator();
                            ui.menu_button("➕ Add Animation", |ui| {
                                let t = app.scene.timeline.current_time;
                                for anim_type in AnimType::all_variants() {
                                    if ui.button(anim_type.name()).clicked() {
                                        app.scene.animations.push(AnimEntry::new(id, anim_type, t));
                                        ui.close_menu();
                                    }
                                }
                            });
                            ui.menu_button("➕ Add to New Lane", |ui| {
                                let t = app.scene.timeline.current_time;
                                let next_lane = crate::scene::max_lane_for_object(&app.scene.animations, id) + 1;
                                for anim_type in AnimType::all_variants() {
                                    if ui.button(anim_type.name()).clicked() {
                                        app.scene.animations.push(AnimEntry::new_on_lane(id, anim_type, t, next_lane));
                                        ui.close_menu();
                                    }
                                }
                            });
                        });
                    }

                    if let Some(id) = to_delete {
                        app.scene.remove_object(&id);
                        if app.selected_obj.as_deref() == Some(&id) {
                            app.selected_obj = None;
                        }
                    }
                    if let Some(id) = to_duplicate {
                        app.selected_obj = Some(id.clone());
                        app.duplicate_selected();
                    }
                    if let Some(id) = to_toggle_vis {
                        if let Some(obj) = app.scene.get_object_mut(&id) {
                            obj.visible = !obj.visible;
                        }
                    }
                });

            ui.separator();

            // ── Add Object templates ──────────────────────────────────────────
            ui.label(
                RichText::new("➕ Add Object")
                    .small()
                    .color(Color32::from_rgb(140, 140, 160)),
            );

            // Filter box
            ui.add(
                egui::TextEdit::singleline(&mut app.add_panel_filter)
                    .hint_text("🔍 filter…")
                    .desired_width(f32::INFINITY),
            );

            egui::ScrollArea::vertical()
                .id_source("add_obj_scroll")
                .show(ui, |ui| {
                    let filter = app.add_panel_filter.to_lowercase();
                    let templates = object_templates();
                    let mut to_add: Option<crate::scene::ObjType> = None;

                    // Category definitions
                    let categories: &[(&str, &[&str])] = &[
                        ("2D Shapes", &["Circle", "Square", "Rectangle", "Triangle", "Ellipse",
                            "Annulus", "Sector", "Arc", "RegularPolygon", "Star",
                            "RoundedRectangle", "Dot", "Polygon"]),
                        ("Text", &["Text", "MathTex", "Title", "MarkupText", "BulletedList",
                            "Paragraph", "Code"]),
                        ("Lines & Arrows", &["Arrow", "Line", "DashedLine", "DoubleArrow",
                            "Vector", "CurvedArrow", "CurvedDoubleArrow", "Elbow", "TangentLine"]),
                        ("Annotations", &["Brace", "BraceLabel", "Angle", "RightAngle",
                            "LabeledDot", "LabeledLine", "SurroundingRectangle",
                            "BackgroundRectangle", "Underline", "Cross"]),
                        ("Graphing", &["Axes", "NumberPlane", "NumberLine", "BarChart",
                            "FunctionGraph", "ParametricFunction", "ImplicitFunction",
                            "ComplexPlane", "PolarPlane", "CoordinateSystem"]),
                        ("Tables & Math", &["Table", "Matrix", "DecimalNumber", "Integer"]),
                        ("Grouping", &["VGroup"]),
                        ("3D Objects", &["Sphere", "Cube", "Cylinder", "Dot3D", "Cone", "Torus",
                            "Prism", "Arrow3D", "Line3D", "Surface", "Icosahedron", "Dodecahedron"]),
                        ("Special", &["TracedPath", "PointCloudDot"]),
                    ];

                    let render_group = |ui: &mut egui::Ui, label: &str, items: Vec<&(&str, &str, crate::scene::ObjType)>, to_add: &mut Option<crate::scene::ObjType>| {
                        if items.is_empty() { return; }
                        ui.label(RichText::new(label).small().color(Color32::from_rgb(100, 120, 160)));
                        ui.horizontal_wrapped(|ui| {
                            for (icon, name, obj_type) in items {
                                if ui.button(format!("{} {}", icon, name))
                                    .on_hover_text(format!("Add {}", name))
                                    .clicked()
                                {
                                    *to_add = Some(obj_type.clone());
                                }
                            }
                        });
                    };

                    for (cat_name, cat_names) in categories {
                        let items: Vec<_> = templates
                            .iter()
                            .filter(|(_, name, _)| {
                                cat_names.contains(name)
                                    && (filter.is_empty() || name.to_lowercase().contains(&filter))
                            })
                            .collect();
                        render_group(ui, cat_name, items, &mut to_add);
                    }

                    if let Some(obj_type) = to_add {
                        app.add_object(obj_type);
                    }
                });

            ui.separator();

            // ── Animation list for selected object ────────────────────────────
            if let Some(sel_id) = app.selected_obj.clone() {
                ui.label(
                    RichText::new("🎞 Animations")
                        .small()
                        .color(Color32::from_rgb(140, 140, 160)),
                );

                let anim_ids: Vec<String> = app
                    .scene
                    .animations
                    .iter()
                    .filter(|a| a.object_id == sel_id)
                    .map(|a| a.id.clone())
                    .collect();

                if anim_ids.is_empty() {
                    ui.label(
                        RichText::new("No animations")
                            .small()
                            .italics()
                            .color(Color32::from_rgb(90, 90, 110)),
                    );
                }

                let mut to_delete_anim: Option<String> = None;

                egui::ScrollArea::vertical()
                    .id_source("obj_anim_scroll")
                    .max_height(120.0)
                    .show(ui, |ui| {
                        for aid in &anim_ids {
                            if let Some(anim) = app.scene.animations.iter().find(|a| a.id == *aid) {
                                let is_sel = app.selected_anim.as_deref() == Some(aid);
                                let [r, g, b] = anim.anim_type.track_color();
                                let label = format!(
                                    "{} ({:.2}s → {:.2}s)",
                                    anim.anim_type.name(),
                                    anim.start_time,
                                    anim.end_time()
                                );
                                let resp = ui.selectable_label(
                                    is_sel,
                                    RichText::new(label).color(Color32::from_rgb(r, g, b)),
                                );
                                if resp.clicked() {
                                    app.selected_anim =
                                        if is_sel { None } else { Some(aid.clone()) };
                                    app.scene.timeline.current_time = anim.start_time;
                                }
                                resp.context_menu(|ui| {
                                    if ui.button("🗑 Delete animation").clicked() {
                                        to_delete_anim = Some(aid.clone());
                                        ui.close_menu();
                                    }
                                });
                            }
                        }
                    });

                if let Some(id) = to_delete_anim {
                    app.scene.animations.retain(|a| a.id != id);
                    if app.selected_anim.as_deref() == Some(&id) {
                        app.selected_anim = None;
                    }
                }

                // Quick-add anim at playhead
                let t = app.scene.timeline.current_time;
                ui.horizontal_wrapped(|ui| {
                    for at in [AnimType::Create, AnimType::FadeIn, AnimType::FadeOut, AnimType::Write, AnimType::Indicate] {
                        let [r, g, b] = at.track_color();
                        if ui.button(RichText::new(at.name()).color(Color32::from_rgb(r, g, b)).small()).clicked() {
                            app.scene.animations.push(AnimEntry::new(&sel_id, at, t));
                        }
                    }
                });
            }
        });
}