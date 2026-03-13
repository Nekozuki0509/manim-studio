use crate::app::ManimStudio;
use crate::scene::{AnimType, ObjType, RateFunc};
use egui::{Color32, Context, RichText};

pub fn show(app: &mut ManimStudio, ctx: &Context) {
    egui::SidePanel::right("properties_panel")
        .min_width(240.0)
        .max_width(340.0)
        .resizable(true)
        .frame(egui::Frame::none().fill(Color32::from_rgb(24, 24, 30)).inner_margin(8.0))
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // ── Object Properties ──────────────────────────────────────────
                if let Some(id) = app.selected_obj.clone() {
                    show_object_props(app, ui, &id);
                }

                // ── Animation Properties ───────────────────────────────────────
                if let Some(id) = app.selected_anim.clone() {
                    show_anim_props(app, ui, &id);
                }

                if app.selected_obj.is_none() && app.selected_anim.is_none() {
                    ui.label(
                        RichText::new("Select an object or animation\nto edit its properties")
                            .color(Color32::from_rgb(90, 90, 110))
                            .italics(),
                    );
                }
            });
        });
}

fn show_object_props(app: &mut ManimStudio, ui: &mut egui::Ui, id: &str) {
    if app.scene.get_object(id).is_none() { return; }

    ui.label(RichText::new("📦 Object Properties").strong().color(Color32::from_rgb(180, 180, 220)));
    ui.separator();

    // Name
    let name = app.scene.get_object(id).unwrap().name.clone();
    ui.horizontal(|ui| {
        ui.label("Name:");
        let mut n = name.clone();
        if ui.text_edit_singleline(&mut n).changed() {
            if let Some(obj) = app.scene.get_object_mut(id) { obj.name = n; }
        }
    });

    // Type badge
    let type_name = app.scene.get_object(id).unwrap().type_name().to_string();
    ui.label(RichText::new(format!("Type: {}", type_name)).color(Color32::from_rgb(140, 160, 200)).small());

    ui.separator();

    // Position
    let pos = app.scene.get_object(id).unwrap().position;
    ui.label("Position:");
    ui.horizontal(|ui| {
        let mut x = pos[0]; let mut y = pos[1]; let mut z = pos[2];
        ui.label("X");
        let cx = ui.add(egui::DragValue::new(&mut x).speed(0.05).fixed_decimals(2)).changed();
        ui.label("Y");
        let cy = ui.add(egui::DragValue::new(&mut y).speed(0.05).fixed_decimals(2)).changed();
        ui.label("Z");
        let cz = ui.add(egui::DragValue::new(&mut z).speed(0.05).fixed_decimals(2)).changed();
        if cx || cy || cz {
            if let Some(obj) = app.scene.get_object_mut(id) { obj.position = [x, y, z]; }
        }
    });

    // Scale
    let scale = app.scene.get_object(id).unwrap().scale;
    ui.horizontal(|ui| {
        ui.label("Scale:");
        let mut s = scale;
        if ui.add(egui::DragValue::new(&mut s).speed(0.01).clamp_range(0.01..=50.0).fixed_decimals(2)).changed() {
            if let Some(obj) = app.scene.get_object_mut(id) { obj.scale = s; }
        }
    });

    // Rotation
    let rot = app.scene.get_object(id).unwrap().rotation;
    ui.horizontal(|ui| {
        ui.label("Rotation:");
        let mut r = rot;
        if ui.add(egui::Slider::new(&mut r, -360.0..=360.0).suffix("°")).changed() {
            if let Some(obj) = app.scene.get_object_mut(id) { obj.rotation = r; }
        }
    });

    ui.separator();

    // Color
    let col = app.scene.get_object(id).unwrap().color;
    ui.horizontal(|ui| {
        ui.label("Fill Color:");
        let mut c = col;
        if ui.color_edit_button_rgb(&mut c).changed() {
            if let Some(obj) = app.scene.get_object_mut(id) { obj.color = c; }
        }
    });

    let fo = app.scene.get_object(id).unwrap().fill_opacity;
    ui.horizontal(|ui| {
        ui.label("Fill Opacity:");
        let mut v = fo;
        if ui.add(egui::Slider::new(&mut v, 0.0..=1.0)).changed() {
            if let Some(obj) = app.scene.get_object_mut(id) { obj.fill_opacity = v; }
        }
    });

    let op = app.scene.get_object(id).unwrap().opacity;
    ui.horizontal(|ui| {
        ui.label("Opacity:");
        let mut v = op;
        if ui.add(egui::Slider::new(&mut v, 0.0..=1.0)).changed() {
            if let Some(obj) = app.scene.get_object_mut(id) { obj.opacity = v; }
        }
    });

    let sw = app.scene.get_object(id).unwrap().stroke_width;
    ui.horizontal(|ui| {
        ui.label("Stroke Width:");
        let mut v = sw;
        if ui.add(egui::DragValue::new(&mut v).speed(0.1).clamp_range(0.0..=20.0).fixed_decimals(1)).changed() {
            if let Some(obj) = app.scene.get_object_mut(id) { obj.stroke_width = v; }
        }
    });

    ui.separator();

    // Visibility
    let vis = app.scene.get_object(id).unwrap().visible;
    let mut v = vis;
    if ui.checkbox(&mut v, "Visible").changed() {
        if let Some(obj) = app.scene.get_object_mut(id) { obj.visible = v; }
    }

    ui.separator();

    // Type-specific properties
    let ot = app.scene.get_object(id).unwrap().object_type.clone();
    show_type_props(app, ui, id, &ot);
}

fn show_type_props(app: &mut ManimStudio, ui: &mut egui::Ui, id: &str, ot: &ObjType) {
    ui.label(RichText::new("⚙ Type Properties").color(Color32::from_rgb(140, 160, 200)).small());

    match ot {
        ObjType::Circle { radius } => {
            let mut r = *radius;
            ui.horizontal(|ui| {
                ui.label("Radius:");
                if ui.add(egui::DragValue::new(&mut r).speed(0.05).clamp_range(0.1..=20.0).fixed_decimals(2)).changed() {
                    if let Some(obj) = app.scene.get_object_mut(id) {
                        obj.object_type = ObjType::Circle { radius: r };
                    }
                }
            });
        }
        ObjType::Square { side_length } => {
            let mut s = *side_length;
            ui.horizontal(|ui| {
                ui.label("Side Length:");
                if ui.add(egui::DragValue::new(&mut s).speed(0.05).clamp_range(0.1..=20.0).fixed_decimals(2)).changed() {
                    if let Some(obj) = app.scene.get_object_mut(id) {
                        obj.object_type = ObjType::Square { side_length: s };
                    }
                }
            });
        }
        ObjType::Rectangle { width, height } => {
            let mut w = *width; let mut h = *height;
            ui.horizontal(|ui| {
                ui.label("Width:");
                ui.add(egui::DragValue::new(&mut w).speed(0.05).clamp_range(0.1..=20.0).fixed_decimals(2));
                ui.label("Height:");
                if ui.add(egui::DragValue::new(&mut h).speed(0.05).clamp_range(0.1..=20.0).fixed_decimals(2)).changed() || w != *width {
                    if let Some(obj) = app.scene.get_object_mut(id) {
                        obj.object_type = ObjType::Rectangle { width: w, height: h };
                    }
                }
            });
        }
        ObjType::Text { content, font_size } => {
            let mut c = content.clone(); let mut fs = *font_size;
            ui.label("Content:");
            if ui.text_edit_multiline(&mut c).changed() {
                if let Some(obj) = app.scene.get_object_mut(id) {
                    obj.object_type = ObjType::Text { content: c, font_size: fs };
                }
            }
            ui.horizontal(|ui| {
                ui.label("Font Size:");
                if ui.add(egui::DragValue::new(&mut fs).speed(1.0).clamp_range(8.0..=200.0)).changed() {
                    if let Some(obj) = app.scene.get_object_mut(id) {
                        let c = content.clone();
                        obj.object_type = ObjType::Text { content: c, font_size: fs };
                    }
                }
            });
        }
        ObjType::MathTex { content } => {
            let mut c = content.clone();
            ui.label("LaTeX:");
            if ui.text_edit_singleline(&mut c).changed() {
                if let Some(obj) = app.scene.get_object_mut(id) {
                    obj.object_type = ObjType::MathTex { content: c };
                }
            }
            ui.label(
                egui::RichText::new("⚠ MathTex requires LaTeX (e.g. MiKTeX) to render.\nInstall from: miktex.org")
                    .small()
                    .color(egui::Color32::from_rgb(255, 200, 80)),
            );
        }
        ObjType::Sphere { radius } => {
            let mut r = *radius;
            ui.horizontal(|ui| {
                ui.label("Radius:");
                if ui.add(egui::DragValue::new(&mut r).speed(0.05).clamp_range(0.1..=20.0)).changed() {
                    if let Some(obj) = app.scene.get_object_mut(id) {
                        obj.object_type = ObjType::Sphere { radius: r };
                    }
                }
            });
        }
        ObjType::Cylinder { radius, height } => {
            let mut r = *radius; let mut h = *height;
            ui.horizontal(|ui| {
                ui.label("R:");
                if ui.add(egui::DragValue::new(&mut r).speed(0.05)).changed() {
                    let h = *height;
                    if let Some(obj) = app.scene.get_object_mut(id) {
                        obj.object_type = ObjType::Cylinder { radius: r, height: h };
                    }
                }
                ui.label("H:");
                if ui.add(egui::DragValue::new(&mut h).speed(0.05)).changed() {
                    let r = *radius;
                    if let Some(obj) = app.scene.get_object_mut(id) {
                        obj.object_type = ObjType::Cylinder { radius: r, height: h };
                    }
                }
            });
        }
        ObjType::Triangle { side_length } => {
            let mut s = *side_length;
            ui.horizontal(|ui| {
                ui.label("Side:");
                if ui.add(egui::DragValue::new(&mut s).speed(0.05).clamp_range(0.1..=20.0)).changed() {
                    if let Some(obj) = app.scene.get_object_mut(id) {
                        obj.object_type = ObjType::Triangle { side_length: s };
                    }
                }
            });
        }
        ObjType::Cube { side_length } => {
            let mut s = *side_length;
            ui.horizontal(|ui| {
                ui.label("Side:");
                if ui.add(egui::DragValue::new(&mut s).speed(0.05).clamp_range(0.1..=20.0)).changed() {
                    if let Some(obj) = app.scene.get_object_mut(id) {
                        obj.object_type = ObjType::Cube { side_length: s };
                    }
                }
            });
        }
        ObjType::Ellipse { width, height } => {
            let mut w = *width; let mut h = *height;
            ui.horizontal(|ui| {
                ui.label("W:");
                ui.add(egui::DragValue::new(&mut w).speed(0.05).clamp_range(0.1..=20.0));
                ui.label("H:");
                if ui.add(egui::DragValue::new(&mut h).speed(0.05).clamp_range(0.1..=20.0)).changed() || w != *width {
                    if let Some(obj) = app.scene.get_object_mut(id) {
                        obj.object_type = ObjType::Ellipse { width: w, height: h };
                    }
                }
            });
        }
        ObjType::Arc { radius, start_angle, angle } => {
            let mut r = *radius; let mut sa = *start_angle; let mut a = *angle;
            ui.horizontal(|ui| {
                ui.label("R:");
                ui.add(egui::DragValue::new(&mut r).speed(0.05).clamp_range(0.1..=20.0));
            });
            ui.horizontal(|ui| {
                ui.label("Start°:");
                ui.add(egui::DragValue::new(&mut sa).speed(1.0).clamp_range(-360.0..=360.0));
                ui.label("Angle°:");
                ui.add(egui::DragValue::new(&mut a).speed(1.0).clamp_range(-360.0..=360.0));
            });
            if r != *radius || sa != *start_angle || a != *angle {
                if let Some(obj) = app.scene.get_object_mut(id) {
                    obj.object_type = ObjType::Arc { radius: r, start_angle: sa, angle: a };
                }
            }
        }
        ObjType::ArcBetweenPoints { start, end, angle } => {
            let mut s = *start; let mut e = *end; let mut a = *angle;
            ui.label("Start:");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut s[0]).speed(0.05).prefix("x:"));
                ui.add(egui::DragValue::new(&mut s[1]).speed(0.05).prefix("y:"));
                ui.add(egui::DragValue::new(&mut s[2]).speed(0.05).prefix("z:"));
            });
            ui.label("End:");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut e[0]).speed(0.05).prefix("x:"));
                ui.add(egui::DragValue::new(&mut e[1]).speed(0.05).prefix("y:"));
                ui.add(egui::DragValue::new(&mut e[2]).speed(0.05).prefix("z:"));
            });
            ui.horizontal(|ui| {
                ui.label("Angle°:");
                ui.add(egui::DragValue::new(&mut a).speed(1.0).clamp_range(-360.0..=360.0));
            });
            if s != *start || e != *end || a != *angle {
                if let Some(obj) = app.scene.get_object_mut(id) {
                    obj.object_type = ObjType::ArcBetweenPoints { start: s, end: e, angle: a };
                }
            }
        }
        ObjType::Annulus { inner_radius, outer_radius } => {
            let mut ri = *inner_radius; let mut ro = *outer_radius;
            ui.horizontal(|ui| {
                ui.label("Inner R:");
                ui.add(egui::DragValue::new(&mut ri).speed(0.05).clamp_range(0.05..=19.0));
                ui.label("Outer R:");
                ui.add(egui::DragValue::new(&mut ro).speed(0.05).clamp_range(0.1..=20.0));
            });
            if ri != *inner_radius || ro != *outer_radius {
                if let Some(obj) = app.scene.get_object_mut(id) {
                    obj.object_type = ObjType::Annulus { inner_radius: ri, outer_radius: ro };
                }
            }
        }
        ObjType::Sector { radius, start_angle, angle } => {
            let mut r = *radius; let mut sa = *start_angle; let mut a = *angle;
            ui.horizontal(|ui| {
                ui.label("R:");
                ui.add(egui::DragValue::new(&mut r).speed(0.05).clamp_range(0.1..=20.0));
            });
            ui.horizontal(|ui| {
                ui.label("Start°:");
                ui.add(egui::DragValue::new(&mut sa).speed(1.0));
                ui.label("Angle°:");
                ui.add(egui::DragValue::new(&mut a).speed(1.0));
            });
            if r != *radius || sa != *start_angle || a != *angle {
                if let Some(obj) = app.scene.get_object_mut(id) {
                    obj.object_type = ObjType::Sector { radius: r, start_angle: sa, angle: a };
                }
            }
        }
        ObjType::RegularPolygon { n, radius } => {
            let mut nn = *n; let mut r = *radius;
            ui.horizontal(|ui| {
                ui.label("Sides:");
                ui.add(egui::DragValue::new(&mut nn).speed(0.1).clamp_range(3..=50));
                ui.label("R:");
                ui.add(egui::DragValue::new(&mut r).speed(0.05).clamp_range(0.1..=20.0));
            });
            if nn != *n || r != *radius {
                if let Some(obj) = app.scene.get_object_mut(id) {
                    obj.object_type = ObjType::RegularPolygon { n: nn, radius: r };
                }
            }
        }
        ObjType::Star { n, outer_radius, inner_radius } => {
            let mut nn = *n; let mut ro = *outer_radius; let mut ri = *inner_radius;
            ui.horizontal(|ui| {
                ui.label("Points:");
                ui.add(egui::DragValue::new(&mut nn).speed(0.1).clamp_range(3..=20));
            });
            ui.horizontal(|ui| {
                ui.label("Outer R:");
                ui.add(egui::DragValue::new(&mut ro).speed(0.05).clamp_range(0.1..=20.0));
                ui.label("Inner R:");
                ui.add(egui::DragValue::new(&mut ri).speed(0.05).clamp_range(0.05..=19.0));
            });
            if nn != *n || ro != *outer_radius || ri != *inner_radius {
                if let Some(obj) = app.scene.get_object_mut(id) {
                    obj.object_type = ObjType::Star { n: nn, outer_radius: ro, inner_radius: ri };
                }
            }
        }
        ObjType::RoundedRectangle { width, height, corner_radius } => {
            let mut w = *width; let mut h = *height; let mut cr = *corner_radius;
            ui.horizontal(|ui| {
                ui.label("W:");
                ui.add(egui::DragValue::new(&mut w).speed(0.05).clamp_range(0.1..=20.0));
                ui.label("H:");
                ui.add(egui::DragValue::new(&mut h).speed(0.05).clamp_range(0.1..=20.0));
            });
            ui.horizontal(|ui| {
                ui.label("Corner R:");
                ui.add(egui::DragValue::new(&mut cr).speed(0.02).clamp_range(0.0..=5.0));
            });
            if w != *width || h != *height || cr != *corner_radius {
                if let Some(obj) = app.scene.get_object_mut(id) {
                    obj.object_type = ObjType::RoundedRectangle { width: w, height: h, corner_radius: cr };
                }
            }
        }
        ObjType::Line { start, end } | ObjType::Arrow { start, end }
        | ObjType::DashedLine { start, end, .. } | ObjType::DoubleArrow { start, end }
        | ObjType::BraceBetweenPoints { start, end } | ObjType::Arrow3D { start, end }
        | ObjType::Line3D { start, end } => {
            let mut s = *start; let mut e = *end;
            ui.label("Start:");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut s[0]).speed(0.05).prefix("x:"));
                ui.add(egui::DragValue::new(&mut s[1]).speed(0.05).prefix("y:"));
                ui.add(egui::DragValue::new(&mut s[2]).speed(0.05).prefix("z:"));
            });
            ui.label("End:");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut e[0]).speed(0.05).prefix("x:"));
                ui.add(egui::DragValue::new(&mut e[1]).speed(0.05).prefix("y:"));
                ui.add(egui::DragValue::new(&mut e[2]).speed(0.05).prefix("z:"));
            });
            if s != *start || e != *end {
                if let Some(obj) = app.scene.get_object_mut(id) {
                    match &obj.object_type {
                        ObjType::Line { .. } => obj.object_type = ObjType::Line { start: s, end: e },
                        ObjType::Arrow { .. } => obj.object_type = ObjType::Arrow { start: s, end: e },
                        ObjType::DashedLine { dash_length, .. } => {
                            let dl = *dash_length;
                            obj.object_type = ObjType::DashedLine { start: s, end: e, dash_length: dl };
                        }
                        ObjType::DoubleArrow { .. } => obj.object_type = ObjType::DoubleArrow { start: s, end: e },
                        ObjType::BraceBetweenPoints { .. } => obj.object_type = ObjType::BraceBetweenPoints { start: s, end: e },
                        ObjType::Arrow3D { .. } => obj.object_type = ObjType::Arrow3D { start: s, end: e },
                        ObjType::Line3D { .. } => obj.object_type = ObjType::Line3D { start: s, end: e },
                        ObjType::ArcBetweenPoints { angle, .. } => {
                            let a = *angle;
                            obj.object_type = ObjType::ArcBetweenPoints { start: s, end: e, angle: a };
                        }
                        _ => {}
                    }
                }
            }
        }
        ObjType::Vector { direction } => {
            let mut d = *direction;
            ui.label("Direction:");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut d[0]).speed(0.05).prefix("x:"));
                ui.add(egui::DragValue::new(&mut d[1]).speed(0.05).prefix("y:"));
                ui.add(egui::DragValue::new(&mut d[2]).speed(0.05).prefix("z:"));
            });
            if d != *direction {
                if let Some(obj) = app.scene.get_object_mut(id) {
                    obj.object_type = ObjType::Vector { direction: d };
                }
            }
        }
        ObjType::Brace { direction, length } => {
            let mut d = *direction; let mut l = *length;
            ui.horizontal(|ui| {
                ui.label("Length:");
                ui.add(egui::DragValue::new(&mut l).speed(0.05).clamp_range(0.1..=20.0));
            });
            ui.label("Direction:");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut d[0]).speed(0.05).prefix("x:"));
                ui.add(egui::DragValue::new(&mut d[1]).speed(0.05).prefix("y:"));
                ui.add(egui::DragValue::new(&mut d[2]).speed(0.05).prefix("z:"));
            });
            if d != *direction || l != *length {
                if let Some(obj) = app.scene.get_object_mut(id) {
                    obj.object_type = ObjType::Brace { direction: d, length: l };
                }
            }
        }
        ObjType::Angle { radius, start_angle, angle } => {
            let mut r = *radius; let mut sa = *start_angle; let mut a = *angle;
            ui.horizontal(|ui| {
                ui.label("R:");
                ui.add(egui::DragValue::new(&mut r).speed(0.05).clamp_range(0.1..=10.0));
                ui.label("Start°:");
                ui.add(egui::DragValue::new(&mut sa).speed(1.0));
                ui.label("Angle°:");
                ui.add(egui::DragValue::new(&mut a).speed(1.0));
            });
            if r != *radius || sa != *start_angle || a != *angle {
                if let Some(obj) = app.scene.get_object_mut(id) {
                    obj.object_type = ObjType::Angle { radius: r, start_angle: sa, angle: a };
                }
            }
        }
        ObjType::RightAngle { size } => {
            let mut sz = *size;
            ui.horizontal(|ui| {
                ui.label("Size:");
                if ui.add(egui::DragValue::new(&mut sz).speed(0.05).clamp_range(0.1..=5.0)).changed() {
                    if let Some(obj) = app.scene.get_object_mut(id) {
                        obj.object_type = ObjType::RightAngle { size: sz };
                    }
                }
            });
        }
        ObjType::NumberLine { x_min, x_max, step } => {
            let mut mn = *x_min; let mut mx = *x_max; let mut st = *step;
            ui.horizontal(|ui| {
                ui.label("Min:");
                ui.add(egui::DragValue::new(&mut mn).speed(0.1));
                ui.label("Max:");
                ui.add(egui::DragValue::new(&mut mx).speed(0.1));
                ui.label("Step:");
                ui.add(egui::DragValue::new(&mut st).speed(0.1).clamp_range(0.1..=10.0));
            });
            if mn != *x_min || mx != *x_max || st != *step {
                if let Some(obj) = app.scene.get_object_mut(id) {
                    obj.object_type = ObjType::NumberLine { x_min: mn, x_max: mx, step: st };
                }
            }
        }
        ObjType::BarChart { values, bar_width } => {
            let mut bw = *bar_width;
            ui.horizontal(|ui| {
                ui.label("Bar Width:");
                if ui.add(egui::DragValue::new(&mut bw).speed(0.05).clamp_range(0.1..=5.0)).changed() {
                    if let Some(obj) = app.scene.get_object_mut(id) {
                        if let ObjType::BarChart { values, .. } = &obj.object_type {
                            let v = values.clone();
                            obj.object_type = ObjType::BarChart { values: v, bar_width: bw };
                        }
                    }
                }
            });
            ui.label(format!("Values: {:?}", values));
        }
        ObjType::DecimalNumber { number, num_decimal_places } => {
            let mut num = *number; let mut dp = *num_decimal_places;
            ui.horizontal(|ui| {
                ui.label("Number:");
                ui.add(egui::DragValue::new(&mut num).speed(0.1));
                ui.label("Decimals:");
                ui.add(egui::DragValue::new(&mut dp).speed(0.1).clamp_range(0..=10));
            });
            if num != *number || dp != *num_decimal_places {
                if let Some(obj) = app.scene.get_object_mut(id) {
                    obj.object_type = ObjType::DecimalNumber { number: num, num_decimal_places: dp };
                }
            }
        }
        ObjType::Integer { number } => {
            let mut n = *number;
            ui.horizontal(|ui| {
                ui.label("Number:");
                if ui.add(egui::DragValue::new(&mut n).speed(1.0)).changed() {
                    if let Some(obj) = app.scene.get_object_mut(id) {
                        obj.object_type = ObjType::Integer { number: n };
                    }
                }
            });
        }
        ObjType::Cone { radius, height } => {
            let mut r = *radius; let mut h = *height;
            ui.horizontal(|ui| {
                ui.label("R:");
                ui.add(egui::DragValue::new(&mut r).speed(0.05).clamp_range(0.1..=20.0));
                ui.label("H:");
                ui.add(egui::DragValue::new(&mut h).speed(0.05).clamp_range(0.1..=20.0));
            });
            if r != *radius || h != *height {
                if let Some(obj) = app.scene.get_object_mut(id) {
                    obj.object_type = ObjType::Cone { radius: r, height: h };
                }
            }
        }
        ObjType::Torus { major_radius, minor_radius } => {
            let mut mr = *major_radius; let mut mnr = *minor_radius;
            ui.horizontal(|ui| {
                ui.label("Major R:");
                ui.add(egui::DragValue::new(&mut mr).speed(0.05).clamp_range(0.1..=20.0));
                ui.label("Minor R:");
                ui.add(egui::DragValue::new(&mut mnr).speed(0.05).clamp_range(0.05..=10.0));
            });
            if mr != *major_radius || mnr != *minor_radius {
                if let Some(obj) = app.scene.get_object_mut(id) {
                    obj.object_type = ObjType::Torus { major_radius: mr, minor_radius: mnr };
                }
            }
        }
        ObjType::Prism { width, height, depth } => {
            let mut w = *width; let mut h = *height; let mut d = *depth;
            ui.horizontal(|ui| {
                ui.label("W:");
                ui.add(egui::DragValue::new(&mut w).speed(0.05).clamp_range(0.1..=20.0));
                ui.label("H:");
                ui.add(egui::DragValue::new(&mut h).speed(0.05).clamp_range(0.1..=20.0));
                ui.label("D:");
                ui.add(egui::DragValue::new(&mut d).speed(0.05).clamp_range(0.1..=20.0));
            });
            if w != *width || h != *height || d != *depth {
                if let Some(obj) = app.scene.get_object_mut(id) {
                    obj.object_type = ObjType::Prism { width: w, height: h, depth: d };
                }
            }
        }
        _ => {}
    }
}

fn show_anim_props(app: &mut ManimStudio, ui: &mut egui::Ui, id: &str) {
    let anim = match app.scene.animations.iter().find(|a| a.id == *id) {
        Some(a) => a.clone(),
        None => return,
    };

    ui.separator();
    ui.label(RichText::new("🎞 Animation Properties").strong().color(Color32::from_rgb(180, 200, 180)));
    ui.separator();

    let obj_name = app.scene.object_name(&anim.object_id).to_string();
    ui.label(format!("Object: {}", obj_name));
    let [r, g, b] = anim.anim_type.track_color();
    ui.label(
        RichText::new(format!("Type: {}", anim.anim_type.name()))
            .color(Color32::from_rgb(r, g, b)),
    );

    // Start time
    {
        let a = app.scene.animations.iter_mut().find(|a| a.id == *id).unwrap();
        ui.horizontal(|ui| {
            ui.label("Start:");
            ui.add(egui::DragValue::new(&mut a.start_time).speed(0.05).clamp_range(0.0..=200.0).suffix("s").fixed_decimals(2));
        });
        ui.horizontal(|ui| {
            ui.label("Duration:");
            ui.add(egui::DragValue::new(&mut a.duration).speed(0.05).clamp_range(0.05..=30.0).suffix("s").fixed_decimals(2));
        });
    }

    // Rate function
    {
        let current_rate = app.scene.animations.iter().find(|a| a.id == *id).unwrap().rate_func.clone();
        ui.horizontal(|ui| {
            ui.label("Easing:");
            egui::ComboBox::from_id_source("rate_func_combo")
                .selected_text(current_rate.label())
                .show_ui(ui, |ui| {
                    for rf in RateFunc::all() {
                        let label = rf.label();
                        let selected = rf == current_rate;
                        if ui.selectable_label(selected, label).clicked() {
                            if let Some(a) = app.scene.animations.iter_mut().find(|a| a.id == *id) {
                                a.rate_func = rf;
                            }
                        }
                    }
                });
        });
    }

    // Type-specific parameters
    let anim_type = app.scene.animations.iter().find(|a| a.id == *id).unwrap().anim_type.clone();
    match anim_type {
        AnimType::Rotate { angle } => {
            let mut a = angle;
            ui.horizontal(|ui| {
                ui.label("Angle:");
                if ui.add(egui::Slider::new(&mut a, -720.0..=720.0).suffix("°")).changed() {
                    if let Some(anim) = app.scene.animations.iter_mut().find(|a| a.id == *id) {
                        anim.anim_type = AnimType::Rotate { angle: a };
                    }
                }
            });
        }
        AnimType::Scale { factor } => {
            let mut f = factor;
            ui.horizontal(|ui| {
                ui.label("Factor:");
                if ui.add(egui::DragValue::new(&mut f).speed(0.05).clamp_range(0.01..=20.0).fixed_decimals(2)).changed() {
                    if let Some(anim) = app.scene.animations.iter_mut().find(|a| a.id == *id) {
                        anim.anim_type = AnimType::Scale { factor: f };
                    }
                }
            });
        }
        AnimType::Shift { dx, dy } => {
            let mut x = dx; let mut y = dy;
            ui.horizontal(|ui| {
                ui.label("dX:");
                ui.add(egui::DragValue::new(&mut x).speed(0.05).fixed_decimals(2));
                ui.label("dY:");
                let changed = ui.add(egui::DragValue::new(&mut y).speed(0.05).fixed_decimals(2)).changed();
                if changed || x != dx {
                    if let Some(anim) = app.scene.animations.iter_mut().find(|a| a.id == *id) {
                        anim.anim_type = AnimType::Shift { dx: x, dy: y };
                    }
                }
            });
        }
        AnimType::MoveTo { x, y } => {
            let mut tx = x; let mut ty = y;
            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(egui::DragValue::new(&mut tx).speed(0.05).fixed_decimals(2));
                ui.label("Y:");
                let changed = ui.add(egui::DragValue::new(&mut ty).speed(0.05).fixed_decimals(2)).changed();
                if changed || tx != x {
                    if let Some(anim) = app.scene.animations.iter_mut().find(|a| a.id == *id) {
                        anim.anim_type = AnimType::MoveTo { x: tx, y: ty };
                    }
                }
            });
        }
        AnimType::GrowFromPoint { point } => {
            let mut p = point;
            ui.label("Point:");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut p[0]).speed(0.05).prefix("x:"));
                ui.add(egui::DragValue::new(&mut p[1]).speed(0.05).prefix("y:"));
                ui.add(egui::DragValue::new(&mut p[2]).speed(0.05).prefix("z:"));
            });
            if p != point {
                if let Some(anim) = app.scene.animations.iter_mut().find(|a| a.id == *id) {
                    anim.anim_type = AnimType::GrowFromPoint { point: p };
                }
            }
        }
        AnimType::GrowFromEdge { edge } => {
            let mut e = edge;
            ui.label("Edge direction:");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut e[0]).speed(0.05).prefix("x:"));
                ui.add(egui::DragValue::new(&mut e[1]).speed(0.05).prefix("y:"));
                ui.add(egui::DragValue::new(&mut e[2]).speed(0.05).prefix("z:"));
            });
            if e != edge {
                if let Some(anim) = app.scene.animations.iter_mut().find(|a| a.id == *id) {
                    anim.anim_type = AnimType::GrowFromEdge { edge: e };
                }
            }
        }
        AnimType::Transform { target_id } | AnimType::FadeTransform { target_id }
        | AnimType::ReplacementTransform { target_id }
        | AnimType::CounterclockwiseTransform { target_id }
        | AnimType::ClockwiseTransform { target_id } => {
            let mut tid = target_id.clone();
            let obj_names: Vec<(String, String)> = app.scene.objects.iter()
                .map(|o| (o.id.clone(), o.name.clone())).collect();
            ui.horizontal(|ui| {
                ui.label("Target:");
                egui::ComboBox::from_id_source("transform_target_combo")
                    .selected_text(
                        app.scene.get_object(&tid).map(|o| o.name.as_str()).unwrap_or("(none)")
                    )
                    .show_ui(ui, |ui| {
                        for (oid, oname) in &obj_names {
                            if ui.selectable_label(*oid == tid, oname).clicked() {
                                tid = oid.clone();
                            }
                        }
                    });
            });
            if tid != *target_id {
                if let Some(anim) = app.scene.animations.iter_mut().find(|a| a.id == *id) {
                    match &anim.anim_type {
                        AnimType::Transform { .. } => anim.anim_type = AnimType::Transform { target_id: tid },
                        AnimType::FadeTransform { .. } => anim.anim_type = AnimType::FadeTransform { target_id: tid },
                        AnimType::ReplacementTransform { .. } => anim.anim_type = AnimType::ReplacementTransform { target_id: tid },
                        AnimType::CounterclockwiseTransform { .. } => anim.anim_type = AnimType::CounterclockwiseTransform { target_id: tid },
                        AnimType::ClockwiseTransform { .. } => anim.anim_type = AnimType::ClockwiseTransform { target_id: tid },
                        _ => {}
                    }
                }
            }
        }
        _ => {}
    }

    // Delete button
    ui.separator();
    if ui.button(RichText::new("🗑 Delete Animation").color(Color32::from_rgb(255, 100, 100))).clicked() {
        app.scene.animations.retain(|a| a.id != *id);
        app.selected_anim = None;
    }
}