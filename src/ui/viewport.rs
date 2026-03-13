use crate::app::{AxisConstraint, InteractionMode, ManimStudio};
use crate::scene::{compute_display_state, DisplayState, ManimObject, ObjType};
use egui::{Color32, Context, Painter, Pos2, Rect, Stroke, Vec2};

// ─────────────────────────────────────────────────────────────────────────────
// Grid colors
// ─────────────────────────────────────────────────────────────────────────────

const GRID_COLOR: Color32 = Color32::from_rgb(40, 42, 50);
const GRID_MAJOR: Color32 = Color32::from_rgb(55, 58, 68);
const AXIS_COLOR: Color32 = Color32::from_rgb(70, 75, 90);

// ─────────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────────

pub fn show(app: &mut ManimStudio, ctx: &Context) {
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(Color32::from_rgb(15, 15, 18)))
        .show(ctx, |ui| {
            let avail = ui.available_rect_before_wrap();
            let (response, painter) =
                ui.allocate_painter(avail.size(), egui::Sense::click_and_drag());

            let current_t = app.scene.timeline.current_time;
            let is_3d = app.scene.is_3d;
            let in_mode = app.interaction_mode != InteractionMode::Normal;

            // ── Background ────────────────────────────────────────────────────
            let bg = app.scene.bg_color;
            painter.rect_filled(avail, 0.0, Color32::from_rgb(
                (bg[0] * 255.0) as u8,
                (bg[1] * 255.0) as u8,
                (bg[2] * 255.0) as u8,
            ));

            // ── Middle-mouse input ────────────────────────────────────────────
            let middle_down = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Middle));
            let ctrl_held   = ctx.input(|i| i.modifiers.ctrl);
            let mouse_delta = ctx.input(|i| i.pointer.delta());

            if middle_down && response.hovered() && !in_mode {
                if is_3d {
                    if ctrl_held {
                        // Ctrl + middle drag → pan in camera space
                        let pan_speed = 1.0 / app.cam3d.zoom;
                        let (right, up) = cam3d_basis(app.cam3d.phi, app.cam3d.theta);
                        // screen delta → world delta (right/up directions)
                        app.cam3d.pan[0] -= (right[0] * mouse_delta.x - up[0] * mouse_delta.y) * pan_speed;
                        app.cam3d.pan[1] -= (right[1] * mouse_delta.x - up[1] * mouse_delta.y) * pan_speed;
                    } else {
                        // Middle drag → orbit
                        app.cam3d.theta += mouse_delta.x * 0.4;
                        app.cam3d.phi    = (app.cam3d.phi - mouse_delta.y * 0.4).clamp(1.0, 179.0);
                        // Sync back to scene for codegen
                        app.scene.camera_phi   = app.cam3d.phi;
                        app.scene.camera_theta = app.cam3d.theta;
                    }
                    ctx.set_cursor_icon(if ctrl_held { egui::CursorIcon::Move } else { egui::CursorIcon::Crosshair });
                } else {
                    // 2D: middle drag → pan
                    app.vp_pan += mouse_delta;
                    ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
                }
            }

            // ── Scroll to zoom ────────────────────────────────────────────────
            if response.hovered() && !in_mode {
                let scroll = ctx.input(|i| i.raw_scroll_delta.y);
                if scroll != 0.0 {
                    if is_3d {
                        app.cam3d.zoom = (app.cam3d.zoom * (1.0 + scroll * 0.001)).clamp(10.0, 400.0);
                    } else {
                        app.vp_zoom = (app.vp_zoom * (1.0 + scroll * 0.001)).clamp(10.0, 300.0);
                    }
                }
            }

            // ── Blender-style interaction mode handling ───────────────────────
            if in_mode && response.hovered() {
                // Initialize mode_start_mouse on first hover
                if app.mode_start_mouse.is_none() {
                    if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
                        app.mode_start_mouse = Some(pos);
                    }
                }

                if let Some(start) = app.mode_start_mouse {
                    if let Some(current) = ctx.input(|i| i.pointer.hover_pos()) {
                        let delta_screen = current - start;

                        if let Some(id) = app.selected_obj.clone() {
                            match app.interaction_mode {
                                InteractionMode::Grab => {
                                    apply_grab(app, &id, delta_screen, is_3d);
                                }
                                InteractionMode::Scale => {
                                    apply_scale(app, &id, start, current);
                                }
                                InteractionMode::Rotate => {
                                    apply_rotate(app, &id, start, current, avail);
                                }
                                InteractionMode::Normal => {}
                            }
                        }
                    }
                }

                // Left-click → confirm
                if ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary)) {
                    app.confirm_mode();
                }
                // Right-click → cancel
                if ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary)) {
                    app.cancel_mode();
                }

                ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
                ctx.request_repaint();
            }

            if is_3d {
                draw_3d(app, &painter, avail, current_t, &response);
            } else {
                draw_2d(app, &painter, avail, current_t, &response, ctx);
            }

            // ── Interaction mode overlay ──────────────────────────────────────
            if in_mode {
                draw_mode_overlay(app, &painter, avail, is_3d);
            }

            // ── Time overlay ──────────────────────────────────────────────────
            let t   = app.scene.timeline.current_time;
            let dur = app.scene.timeline.duration;
            let hint = if in_mode {
                match app.interaction_mode {
                    InteractionMode::Grab   => "[G: Grab / X,Y,Z: Axis / LMB,Enter: Confirm / RMB,Esc: Cancel]",
                    InteractionMode::Scale  => "[S: Scale / X,Y,Z: Axis / LMB,Enter: Confirm / RMB,Esc: Cancel]",
                    InteractionMode::Rotate => "[R: Rotate / LMB,Enter: Confirm / RMB,Esc: Cancel]",
                    InteractionMode::Normal => "",
                }
            } else if is_3d {
                "[G: Grab / S: Scale / R: Rotate / 1,3,7,0: View / Mid: Orbit / Ctrl+Mid: Pan]"
            } else {
                "[G: Grab / S: Scale / R: Rotate / Mid: Pan / Scroll: Zoom]"
            };
            painter.text(
                avail.left_top() + Vec2::new(8.0, 8.0),
                egui::Align2::LEFT_TOP,
                format!("▶ {:.2}s / {:.2}s  {}", t, dur, hint),
                egui::FontId::monospace(11.0),
                if app.is_playing {
                    Color32::from_rgb(100, 220, 100)
                } else {
                    Color32::from_rgba_premultiplied(160, 160, 160, 170)
                },
            );

            // ── 3D camera info overlay ────────────────────────────────────────
            if is_3d {
                let view_name = view_preset_name(app.cam3d.phi, app.cam3d.theta);
                painter.text(
                    avail.right_top() + Vec2::new(-8.0, 8.0),
                    egui::Align2::RIGHT_TOP,
                    format!("{}  φ={:.0}°  θ={:.0}°  zoom={:.0}", view_name, app.cam3d.phi, app.cam3d.theta, app.cam3d.zoom),
                    egui::FontId::monospace(11.0),
                    Color32::from_rgb(120, 200, 255),
                );
            }
        });
}

// ─────────────────────────────────────────────────────────────────────────────
// 2D rendering
// ─────────────────────────────────────────────────────────────────────────────

fn draw_2d(
    app: &mut ManimStudio,
    painter: &Painter,
    avail: Rect,
    current_t: f32,
    response: &egui::Response,
    ctx: &Context,
) {
    let canvas_center = avail.center() + app.vp_pan;
    let zoom = app.vp_zoom;

    draw_grid(painter, avail, canvas_center, zoom);

    for obj in &app.scene.objects {
        if !obj.visible { continue; }
        let is_sel = app.selected_obj.as_deref() == Some(&obj.id);
        let ds = compute_display_state(obj, &app.scene.animations, current_t);
        if !ds.visible { continue; }
        draw_obj_2d(painter, obj, &ds, canvas_center, zoom, is_sel);
    }

    // Click to select (disabled during interaction mode)
    if response.clicked() && app.interaction_mode == InteractionMode::Normal {
        let click = response.interact_pointer_pos().unwrap_or(avail.center());
        let mut hit = None;
        for obj in app.scene.objects.iter().rev() {
            if !obj.visible { continue; }
            let ds = compute_display_state(obj, &app.scene.animations, current_t);
            if !ds.visible { continue; }
            let sp = m2s(egui::pos2(ds.position[0], ds.position[1]), canvas_center, zoom);
            if (click - sp).length() < approx_r(obj) * zoom {
                hit = Some(obj.id.clone());
                break;
            }
        }
        app.selected_obj = hit;
    }

    // Left drag → move selected (disabled during interaction mode)
    if app.interaction_mode == InteractionMode::Normal
        && response.dragged()
        && !ctx.input(|i| i.pointer.button_down(egui::PointerButton::Middle))
    {
        if let Some(id) = app.selected_obj.clone() {
            let delta = response.drag_delta() / zoom;
            if let Some(obj) = app.scene.get_object_mut(&id) {
                obj.position[0] += delta.x;
                obj.position[1] -= delta.y;
            }
        }
    }

    // Coordinate overlay
    if let Some(hover) = response.hover_pos() {
        let mp = s2m(hover, canvas_center, zoom);
        painter.text(
            avail.left_bottom() + Vec2::new(8.0, -8.0),
            egui::Align2::LEFT_BOTTOM,
            format!("({:.2}, {:.2})", mp.x, mp.y),
            egui::FontId::monospace(11.0),
            Color32::from_rgba_premultiplied(180, 180, 180, 160),
        );
    }
    if let Some(obj) = app.sel_obj() {
        painter.text(
            avail.right_bottom() + Vec2::new(-8.0, -8.0),
            egui::Align2::RIGHT_BOTTOM,
            format!("📌 {} ({:.2}, {:.2})", obj.name, obj.position[0], obj.position[1]),
            egui::FontId::proportional(12.0),
            Color32::from_rgb(220, 220, 100),
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 3D rendering
// ─────────────────────────────────────────────────────────────────────────────

fn draw_3d(
    app: &mut ManimStudio,
    painter: &Painter,
    avail: Rect,
    current_t: f32,
    response: &egui::Response,
) {
    // Sync camera angles with scene values (in case scene was edited in toolbar)
    // (we keep cam3d as ground truth during interaction; sync on first draw if never touched)

    let proj = Projection3D::new(&app.cam3d, avail.center(), app.cam3d.pan);

    // Draw 3D axis gizmo
    draw_axis_gizmo(painter, &proj, avail);

    // Draw 3D grid (XY plane)
    draw_3d_grid(painter, &proj);

    // Objects
    let objects: Vec<_> = app.scene.objects.clone();
    for obj in &objects {
        if !obj.visible { continue; }
        let is_sel = app.selected_obj.as_deref() == Some(&obj.id);
        let ds = compute_display_state(obj, &app.scene.animations, current_t);
        if !ds.visible { continue; }
        draw_obj_3d(painter, obj, &ds, &proj, is_sel);
    }

    // Click to select (project and find nearest; disabled during interaction mode)
    if response.clicked() && app.interaction_mode == InteractionMode::Normal {
        let click = response.interact_pointer_pos().unwrap_or(avail.center());
        let mut best: Option<(f32, String)> = None;
        for obj in &objects {
            if !obj.visible { continue; }
            let ds = compute_display_state(obj, &app.scene.animations, current_t);
            if !ds.visible { continue; }
            let sp = proj.project([ds.position[0], ds.position[1], ds.position[2]]);
            let dist = (click - sp).length();
            let threshold = approx_r(obj) * proj.zoom;
            if dist < threshold {
                if best.as_ref().map(|(d, _)| dist < *d).unwrap_or(true) {
                    best = Some((dist, obj.id.clone()));
                }
            }
        }
        app.selected_obj = best.map(|(_, id)| id);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 3D projection math
// ─────────────────────────────────────────────────────────────────────────────

/// Orthographic projection for a spherical orbit camera.
struct Projection3D {
    /// Camera right vector in world space
    right: [f32; 3],
    /// Camera up vector in world space
    up: [f32; 3],
    /// Screen center in pixels
    center: Pos2,
    pub zoom: f32,
    /// Pan offset in Manim units (applied before projection)
    pan: [f32; 2],
}

impl Projection3D {
    fn new(cam: &crate::app::Camera3D, center: Pos2, pan: [f32; 2]) -> Self {
        let (right, up) = cam3d_basis(cam.phi, cam.theta);
        Self { right, up, center, zoom: cam.zoom, pan }
    }

    /// Project a 3D Manim-space point → screen Pos2.
    fn project(&self, p: [f32; 3]) -> Pos2 {
        // Apply pan: pan shifts in camera right/up directions
        let px = p[0] - self.pan[0];
        let py = p[1] - self.pan[1];
        let pz = p[2];
        let sx = dot3([px, py, pz], self.right);
        let sy = dot3([px, py, pz], self.up);
        Pos2::new(
            self.center.x + sx * self.zoom,
            self.center.y - sy * self.zoom,
        )
    }
}

/// Compute (right, up) basis vectors for a spherical camera.
/// phi = elevation from z-axis (degrees), theta = azimuth (degrees).
fn cam3d_basis(phi_deg: f32, theta_deg: f32) -> ([f32; 3], [f32; 3]) {
    let phi   = phi_deg.to_radians();
    let theta = theta_deg.to_radians();

    // right = (-sin θ, cos θ, 0)   (horizontal, perpendicular to azimuth)
    let right = [-theta.sin(), theta.cos(), 0.0_f32];

    // up = (-cos φ cos θ, -cos φ sin θ, sin φ)
    let up = [
        -phi.cos() * theta.cos(),
        -phi.cos() * theta.sin(),
         phi.sin(),
    ];

    (right, up)
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0]*b[0] + a[1]*b[1] + a[2]*b[2]
}

// ─────────────────────────────────────────────────────────────────────────────
// 3D grid and gizmo
// ─────────────────────────────────────────────────────────────────────────────

fn draw_3d_grid(painter: &Painter, proj: &Projection3D) {
    // Extend grid to cover the visible area; compute range from zoom level
    let range = ((20.0 * 60.0 / proj.zoom) as i32).clamp(10, 100);
    let grid_col  = Color32::from_rgba_premultiplied(50, 55, 65, 120);
    let major_col = Color32::from_rgba_premultiplied(65, 70, 85, 160);

    for i in -range..=range {
        let fi = i as f32;
        let col = if i == 0 { major_col } else { grid_col };
        // X lines (along X axis, varying Y)
        let p0 = proj.project([-range as f32, fi, 0.0]);
        let p1 = proj.project([ range as f32, fi, 0.0]);
        painter.line_segment([p0, p1], Stroke::new(if i == 0 { 1.2 } else { 0.5 }, col));
        // Y lines (along Y axis, varying X)
        let p0 = proj.project([fi, -range as f32, 0.0]);
        let p1 = proj.project([fi,  range as f32, 0.0]);
        painter.line_segment([p0, p1], Stroke::new(if i == 0 { 1.2 } else { 0.5 }, col));
    }
}

fn draw_axis_gizmo(painter: &Painter, proj: &Projection3D, avail: Rect) {
    // Small gizmo in bottom-left corner
    let gizmo_center = avail.left_bottom() + Vec2::new(55.0, -55.0);
    let gizmo_len = 35.0_f32;

    // Temporarily create a mini projection centered on the gizmo
    let mini = Projection3D {
        right: proj.right,
        up: proj.up,
        center: gizmo_center,
        zoom: gizmo_len,
        pan: [0.0, 0.0],
    };

    let origin = mini.project([0.0, 0.0, 0.0]);

    // X axis — red
    let xp = mini.project([1.0, 0.0, 0.0]);
    painter.line_segment([origin, xp], Stroke::new(2.5, Color32::from_rgb(230, 70, 70)));
    painter.text(xp + Vec2::new(3.0, -3.0), egui::Align2::LEFT_BOTTOM,
        "X", egui::FontId::proportional(11.0), Color32::from_rgb(230, 70, 70));

    // Y axis — green
    let yp = mini.project([0.0, 1.0, 0.0]);
    painter.line_segment([origin, yp], Stroke::new(2.5, Color32::from_rgb(70, 210, 70)));
    painter.text(yp + Vec2::new(3.0, -3.0), egui::Align2::LEFT_BOTTOM,
        "Y", egui::FontId::proportional(11.0), Color32::from_rgb(70, 210, 70));

    // Z axis — blue
    let zp = mini.project([0.0, 0.0, 1.0]);
    painter.line_segment([origin, zp], Stroke::new(2.5, Color32::from_rgb(80, 140, 255)));
    painter.text(zp + Vec2::new(3.0, -3.0), egui::Align2::LEFT_BOTTOM,
        "Z", egui::FontId::proportional(11.0), Color32::from_rgb(80, 140, 255));

    // Origin dot
    painter.circle_filled(origin, 3.5, Color32::from_rgb(220, 220, 220));
}

// ─────────────────────────────────────────────────────────────────────────────
// 3D object drawing
// ─────────────────────────────────────────────────────────────────────────────

fn draw_obj_3d(
    painter: &Painter,
    obj: &ManimObject,
    ds: &DisplayState,
    proj: &Projection3D,
    selected: bool,
) {
    let pos3 = [ds.position[0], ds.position[1], ds.position[2]];
    let center = proj.project(pos3);
    let c = obj.color;
    let fill = Color32::from_rgba_unmultiplied(
        (c[0]*255.0) as u8, (c[1]*255.0) as u8, (c[2]*255.0) as u8,
        (ds.fill_opacity * ds.opacity * 200.0) as u8,
    );
    let stroke_col = Color32::from_rgba_unmultiplied(
        (c[0]*255.0) as u8, (c[1]*255.0) as u8, (c[2]*255.0) as u8,
        (ds.opacity * 255.0) as u8,
    );
    let stroke = Stroke::new(obj.stroke_width.max(1.0), stroke_col);
    let sel_s = Stroke::new(2.5, Color32::from_rgb(255, 220, 50));
    let s = ds.scale;
    let z = proj.zoom;

    match &obj.object_type {
        ObjType::Sphere { radius } => {
            let r = radius * s * z;
            painter.circle(center, r, fill, stroke);
            // Highlight rim to suggest sphere
            painter.circle_filled(center + Vec2::new(-r*0.25, -r*0.25), r*0.25,
                Color32::from_rgba_premultiplied(255,255,255,40));
            if selected { painter.circle_stroke(center, r+3.0, sel_s); }
        }
        ObjType::Circle { radius } => {
            // Draw 2D circle flat in XY plane (as a projected ellipse)
            let r = radius * s;
            let n = 48;
            let pts: Vec<Pos2> = (0..n).map(|i| {
                let angle = std::f32::consts::TAU * i as f32 / n as f32;
                let lx = r * angle.cos();
                let ly = r * angle.sin();
                proj.project([pos3[0] + lx, pos3[1] + ly, pos3[2]])
            }).collect();
            for i in 0..n {
                painter.line_segment([pts[i], pts[(i+1)%n]], stroke);
            }
            // Fill approximation
            if pts.len() >= 3 {
                painter.add(egui::Shape::convex_polygon(pts.clone(), fill, Stroke::NONE));
            }
            if selected { painter.circle_stroke(center, radius * s * z + 3.0, sel_s); }
        }
        ObjType::Cube { side_length } => {
            let half = side_length * s / 2.0;
            // Project 8 cube corners
            let corners_3d: Vec<[f32;3]> = vec![
                [-half,-half,-half], [ half,-half,-half],
                [ half, half,-half], [-half, half,-half],
                [-half,-half, half], [ half,-half, half],
                [ half, half, half], [-half, half, half],
            ].iter().map(|[dx,dy,dz]| [pos3[0]+dx, pos3[1]+dy, pos3[2]+dz]).collect();
            let c2d: Vec<Pos2> = corners_3d.iter().map(|p| proj.project(*p)).collect();
            // Draw 12 edges
            let edges = [(0,1),(1,2),(2,3),(3,0),(4,5),(5,6),(6,7),(7,4),(0,4),(1,5),(2,6),(3,7)];
            for (a,b) in edges {
                painter.line_segment([c2d[a], c2d[b]], stroke);
            }
            if selected { painter.circle_stroke(center, half*z+3.0, sel_s); }
        }
        ObjType::Square { side_length } => {
            // Draw 2D square flat in XY plane
            let half = side_length * s / 2.0;
            let corners: Vec<Pos2> = vec![
                [-half, -half], [half, -half], [half, half], [-half, half],
            ].iter().map(|[dx, dy]| {
                proj.project([pos3[0] + dx, pos3[1] + dy, pos3[2]])
            }).collect();
            painter.add(egui::Shape::convex_polygon(corners.clone(), fill, stroke));
            if selected {
                painter.add(egui::Shape::convex_polygon(corners, Color32::TRANSPARENT, sel_s));
            }
        }
        ObjType::Rectangle { width, height } => {
            // Draw 2D rectangle flat in XY plane
            let hw = width * s / 2.0;
            let hh = height * s / 2.0;
            let corners: Vec<Pos2> = vec![
                [-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh],
            ].iter().map(|[dx, dy]| {
                proj.project([pos3[0] + dx, pos3[1] + dy, pos3[2]])
            }).collect();
            painter.add(egui::Shape::convex_polygon(corners.clone(), fill, stroke));
            if selected {
                painter.add(egui::Shape::convex_polygon(corners, Color32::TRANSPARENT, sel_s));
            }
        }
        ObjType::Triangle { side_length } => {
            // Draw 2D equilateral triangle flat in XY plane
            let r = side_length * s / 2.0;
            let pts: Vec<Pos2> = vec![
                [0.0, r],
                [-r * 0.866, -r * 0.5],
                [r * 0.866, -r * 0.5],
            ].iter().map(|[dx, dy]| {
                proj.project([pos3[0] + dx, pos3[1] + dy, pos3[2]])
            }).collect();
            painter.add(egui::Shape::convex_polygon(pts.clone(), fill, stroke));
            if selected {
                painter.add(egui::Shape::convex_polygon(pts, Color32::TRANSPARENT, sel_s));
            }
        }
        ObjType::Cylinder { radius, height } => {
            // Top and bottom circles approximated as ellipses (just draw projected circles)
            let r = radius * s * z;
            let ht = proj.project([pos3[0], pos3[1], pos3[2] + height*s/2.0]);
            let hb = proj.project([pos3[0], pos3[1], pos3[2] - height*s/2.0]);
            painter.circle(ht, r * 0.4, fill, stroke); // top face
            painter.circle(hb, r * 0.4, fill, stroke); // bottom face
            painter.line_segment([
                Pos2::new(ht.x - r, ht.y),
                Pos2::new(hb.x - r, hb.y),
            ], stroke);
            painter.line_segment([
                Pos2::new(ht.x + r, ht.y),
                Pos2::new(hb.x + r, hb.y),
            ], stroke);
            if selected { painter.circle_stroke(center, r+3.0, sel_s); }
        }
        ObjType::Axes => {
            // Full-size axes
            let l = 5.0;
            let xp = proj.project([pos3[0]+l, pos3[1], pos3[2]]);
            let yp = proj.project([pos3[0], pos3[1]+l, pos3[2]]);
            let zp = proj.project([pos3[0], pos3[1], pos3[2]+l]);
            painter.line_segment([center, xp], Stroke::new(2.0, Color32::from_rgb(230,70,70)));
            painter.line_segment([center, yp], Stroke::new(2.0, Color32::from_rgb(70,210,70)));
            painter.line_segment([center, zp], Stroke::new(2.0, Color32::from_rgb(80,140,255)));
        }
        ObjType::Arrow { start, end } => {
            let p1 = proj.project([start[0], start[1], start[2]]);
            let p2 = proj.project([end[0], end[1], end[2]]);
            painter.line_segment([p1, p2], stroke);
            let dir = (p2 - p1).normalized();
            let perp = Vec2::new(-dir.y, dir.x);
            let hs = 12.0_f32;
            painter.add(egui::Shape::convex_polygon(
                vec![p2, p2 - dir*hs + perp*hs*0.4, p2 - dir*hs - perp*hs*0.4],
                stroke_col, Stroke::NONE));
        }
        ObjType::Line { start, end } => {
            let p1 = proj.project([start[0], start[1], start[2]]);
            let p2 = proj.project([end[0], end[1], end[2]]);
            painter.line_segment([p1, p2], stroke);
        }
        // Text / MathTex: render as flat text plane in XY, oriented in 3D space
        ObjType::Text { content, font_size } => {
            // Approximate text extents in Manim units
            let char_w = font_size * s / 48.0 * 0.5;
            let text_w = char_w * content.len() as f32;
            let text_h = font_size * s / 48.0 * 0.8;
            let hw = text_w / 2.0;
            let hh = text_h / 2.0;
            // Project four corners of the text plane (lying in XY at the object's Z)
            let corners: Vec<Pos2> = vec![
                [-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh],
            ].iter().map(|[dx, dy]| {
                proj.project([pos3[0] + dx, pos3[1] + dy, pos3[2]])
            }).collect();
            // Draw text background plane
            painter.add(egui::Shape::convex_polygon(
                corners.clone(),
                Color32::from_rgba_premultiplied(0, 0, 0, 30),
                Stroke::new(0.5, stroke_col),
            ));
            // Draw the text at the projected center (readable but bounded by the plane)
            let fs = (font_size * s * z / 48.0 * 14.0).clamp(8.0, 60.0);
            painter.text(center, egui::Align2::CENTER_CENTER, content,
                egui::FontId::proportional(fs), stroke_col);
            if selected {
                painter.add(egui::Shape::convex_polygon(corners, Color32::TRANSPARENT, sel_s));
            }
        }
        ObjType::MathTex { content } => {
            let char_w = 0.3 * s;
            let text_w = char_w * content.len() as f32;
            let text_h = 0.5 * s;
            let hw = text_w / 2.0;
            let hh = text_h / 2.0;
            let corners: Vec<Pos2> = vec![
                [-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh],
            ].iter().map(|[dx, dy]| {
                proj.project([pos3[0] + dx, pos3[1] + dy, pos3[2]])
            }).collect();
            painter.add(egui::Shape::convex_polygon(
                corners.clone(),
                Color32::from_rgba_premultiplied(0, 0, 0, 30),
                Stroke::new(0.5, stroke_col),
            ));
            let fs = (14.0 * s * z / 60.0).clamp(8.0, 48.0);
            painter.text(center, egui::Align2::CENTER_CENTER, content,
                egui::FontId::monospace(fs), stroke_col);
            if selected {
                painter.add(egui::Shape::convex_polygon(corners, Color32::TRANSPARENT, sel_s));
            }
        }
        // Fallback — draw as a small dot at projected position
        _ => {
            let r = approx_r(obj) * s * z;
            painter.circle(center, r.max(4.0), fill, stroke);
            if selected { painter.circle_stroke(center, r+3.0, sel_s); }
        }
    }

    if selected {
        painter.text(
            center + Vec2::new(0.0, approx_r(obj) * z + 12.0),
            egui::Align2::CENTER_TOP,
            &obj.name,
            egui::FontId::proportional(11.0),
            Color32::from_rgb(255, 220, 50),
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 2D object drawing (unchanged from before)
// ─────────────────────────────────────────────────────────────────────────────

fn draw_obj_2d(
    painter: &Painter,
    obj: &ManimObject,
    ds: &DisplayState,
    center: Pos2,
    zoom: f32,
    selected: bool,
) {
    let pos = m2s(egui::pos2(ds.position[0], ds.position[1]), center, zoom);
    let c = obj.color;
    let fill = Color32::from_rgba_unmultiplied(
        (c[0]*255.0) as u8,(c[1]*255.0) as u8,(c[2]*255.0) as u8,
        (ds.fill_opacity * ds.opacity * 200.0) as u8,
    );
    let stroke_col = Color32::from_rgba_unmultiplied(
        (c[0]*255.0) as u8,(c[1]*255.0) as u8,(c[2]*255.0) as u8,
        (ds.opacity * 255.0) as u8,
    );
    let stroke = Stroke::new(obj.stroke_width.max(1.0), stroke_col);
    let sel = Stroke::new(2.5, Color32::from_rgb(255, 220, 50));
    let s = ds.scale;
    let z = zoom;

    match &obj.object_type {
        ObjType::Circle { radius } => {
            let r = radius * s * z;
            painter.circle(pos, r, fill, stroke);
            if selected { painter.circle_stroke(pos, r+3.0, sel); }
        }
        ObjType::Square { side_length } => {
            let half = side_length * s * z / 2.0;
            let r = Rect::from_center_size(pos, Vec2::splat(half*2.0));
            painter.rect(r, 0.0, fill, stroke);
            if selected { painter.rect_stroke(r.expand(3.0), 0.0, sel); }
        }
        ObjType::Rectangle { width, height } => {
            let r = Rect::from_center_size(pos, Vec2::new(width*s*z, height*s*z));
            painter.rect(r, 0.0, fill, stroke);
            if selected { painter.rect_stroke(r.expand(3.0), 0.0, sel); }
        }
        ObjType::Triangle { side_length } => {
            let r = side_length * s * z / 2.0;
            let pts = vec![
                Pos2::new(pos.x, pos.y - r),
                Pos2::new(pos.x - r*0.866, pos.y + r*0.5),
                Pos2::new(pos.x + r*0.866, pos.y + r*0.5),
            ];
            painter.add(egui::Shape::convex_polygon(pts.clone(), fill, stroke));
            if selected { painter.add(egui::Shape::convex_polygon(pts, Color32::TRANSPARENT, sel)); }
        }
        ObjType::Text { content, font_size } => {
            let fs = (font_size * s * z / 48.0 * 18.0).clamp(8.0, 72.0);
            painter.text(pos, egui::Align2::CENTER_CENTER, content,
                egui::FontId::proportional(fs), stroke_col);
            if selected {
                let tr = Rect::from_center_size(pos,
                    Vec2::new(fs * content.len() as f32 * 0.55, fs * 1.3));
                painter.rect_stroke(tr.expand(4.0), 3.0, sel);
            }
        }
        ObjType::MathTex { content } => {
            let fs = (18.0 * s * z / 60.0).clamp(8.0, 48.0);
            painter.text(pos, egui::Align2::CENTER_CENTER, content,
                egui::FontId::monospace(fs), stroke_col);
            if selected {
                let tr = Rect::from_center_size(pos,
                    Vec2::new(fs * content.len() as f32 * 0.65, fs * 1.3));
                painter.rect_stroke(tr.expand(4.0), 3.0, sel);
            }
        }
        ObjType::Line { start, end } => {
            let p1 = m2s(egui::pos2(start[0], start[1]), center, zoom);
            let p2 = m2s(egui::pos2(end[0], end[1]), center, zoom);
            painter.line_segment([p1, p2], stroke);
            if selected {
                painter.circle_filled(p1, 4.0, sel.color);
                painter.circle_filled(p2, 4.0, sel.color);
            }
        }
        ObjType::Arrow { start, end } => {
            let p1 = m2s(egui::pos2(start[0], start[1]), center, zoom);
            let p2 = m2s(egui::pos2(end[0], end[1]), center, zoom);
            painter.line_segment([p1, p2], stroke);
            let dir = (p2 - p1).normalized();
            let perp = Vec2::new(-dir.y, dir.x);
            let hs = 12.0_f32;
            let (a, b) = (p2-dir*hs+perp*hs*0.4, p2-dir*hs-perp*hs*0.4);
            painter.add(egui::Shape::convex_polygon(vec![p2, a, b], stroke_col, Stroke::NONE));
            if selected { painter.circle_stroke(p2, hs+3.0, sel); }
        }
        ObjType::Dot => {
            let r = (4.0*s*z/60.0*8.0).max(3.0);
            painter.circle_filled(pos, r, stroke_col);
            if selected { painter.circle_stroke(pos, r+4.0, sel); }
        }
        ObjType::Sphere { radius } => {
            let r = radius * s * z;
            painter.circle(pos, r, fill, stroke);
            painter.circle_filled(pos+Vec2::new(-r*0.25,-r*0.25), r*0.25,
                Color32::from_rgba_premultiplied(255,255,255,40));
            if selected { painter.circle_stroke(pos, r+3.0, sel); }
        }
        ObjType::Cube { side_length } => {
            let half = side_length * s * z / 2.0;
            let r = Rect::from_center_size(pos, Vec2::splat(half*2.0));
            painter.rect(r, 0.0, fill, stroke);
            let tr = Rect::from_center_size(pos+Vec2::new(half*0.5,-half*0.5), Vec2::splat(half*2.0));
            painter.rect(tr, 0.0, Color32::from_rgba_premultiplied(200,200,200,20),
                Stroke::new(1.0, stroke_col));
            if selected { painter.rect_stroke(r.expand(3.0), 0.0, sel); }
        }
        ObjType::Cylinder { radius, height } => {
            let r = Rect::from_center_size(pos, Vec2::new(radius*s*z*2.0, height*s*z));
            painter.rect(r, radius*s*z*0.4, fill, stroke);
            if selected { painter.rect_stroke(r.expand(3.0), radius*s*z*0.4, sel); }
        }
        ObjType::Axes => {
            let l = 5.0 * z;
            painter.line_segment([pos-Vec2::new(l,0.0), pos+Vec2::new(l,0.0)],
                Stroke::new(2.0, Color32::from_rgb(100,200,100)));
            painter.line_segment([pos+Vec2::new(0.0,l), pos-Vec2::new(0.0,l)],
                Stroke::new(2.0, Color32::from_rgb(200,100,100)));
        }
        ObjType::NumberPlane => {
            let half = 5.0 * z;
            let r = Rect::from_center_size(pos, Vec2::splat(half*2.0));
            painter.rect_stroke(r, 0.0,
                Stroke::new(1.0, Color32::from_rgba_premultiplied(80,100,200,120)));
            for i in -4..=4_i32 {
                let off = i as f32 * z;
                painter.line_segment(
                    [Pos2::new(pos.x+off, r.top()), Pos2::new(pos.x+off, r.bottom())],
                    Stroke::new(0.5, Color32::from_rgba_premultiplied(80,100,200,60)));
                painter.line_segment(
                    [Pos2::new(r.left(), pos.y+off), Pos2::new(r.right(), pos.y+off)],
                    Stroke::new(0.5, Color32::from_rgba_premultiplied(80,100,200,60)));
            }
        }
        ObjType::Polygon { points } => {
            if points.len() >= 3 {
                let poly: Vec<Pos2> = points.iter()
                    .map(|p| m2s(egui::pos2(ds.position[0]+p[0], ds.position[1]+p[1]), center, zoom))
                    .collect();
                painter.add(egui::Shape::convex_polygon(poly.clone(), fill, stroke));
                if selected {
                    painter.add(egui::Shape::convex_polygon(poly, Color32::TRANSPARENT, sel));
                }
            }
        }
    }

    if selected {
        painter.text(
            pos + Vec2::new(0.0, approx_r(obj) * zoom + 12.0),
            egui::Align2::CENTER_TOP,
            &obj.name,
            egui::FontId::proportional(11.0),
            Color32::from_rgb(255, 220, 50),
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Blender-style interaction helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Pixels of mouse movement for a 2× scale change.
const SCALE_SENSITIVITY: f32 = 100.0;
/// Length (in screen pixels) of the axis constraint guide line.
const AXIS_GUIDE_LENGTH: f32 = 2000.0;

/// Apply grab (move) transformation based on mouse delta.
fn apply_grab(app: &mut ManimStudio, id: &str, delta_screen: Vec2, is_3d: bool) {
    let zoom = if is_3d { app.cam3d.zoom } else { app.vp_zoom };
    let origin = app.mode_origin_pos;

    // Convert screen delta to Manim-space delta
    let (dx, dy, dz);
    if is_3d {
        let (right, up) = cam3d_basis(app.cam3d.phi, app.cam3d.theta);
        // Project screen delta into world space
        let world_dx = (right[0] * delta_screen.x - up[0] * delta_screen.y) / zoom;
        let world_dy = (right[1] * delta_screen.x - up[1] * delta_screen.y) / zoom;
        let world_dz = (right[2] * delta_screen.x - up[2] * delta_screen.y) / zoom;
        dx = world_dx;
        dy = world_dy;
        dz = world_dz;
    } else {
        dx = delta_screen.x / zoom;
        dy = -delta_screen.y / zoom;
        dz = 0.0;
    }

    if let Some(obj) = app.scene.get_object_mut(id) {
        match app.axis_constraint {
            AxisConstraint::None => {
                obj.position[0] = origin[0] + dx;
                obj.position[1] = origin[1] + dy;
                obj.position[2] = origin[2] + dz;
            }
            AxisConstraint::X => {
                obj.position[0] = origin[0] + dx;
                obj.position[1] = origin[1];
                obj.position[2] = origin[2];
            }
            AxisConstraint::Y => {
                obj.position[0] = origin[0];
                obj.position[1] = origin[1] + dy;
                obj.position[2] = origin[2];
            }
            AxisConstraint::Z => {
                obj.position[0] = origin[0];
                obj.position[1] = origin[1];
                if is_3d {
                    obj.position[2] = origin[2] + dz;
                } else {
                    obj.position[2] = origin[2];
                }
            }
        }
    }
}

/// Apply scale transformation based on mouse distance from start point.
fn apply_scale(app: &mut ManimStudio, id: &str, start: Pos2, current: Pos2) {
    let dist = (current - start).length();
    let factor = 1.0 + dist / SCALE_SENSITIVITY;
    let origin_scale = app.mode_origin_scale;

    if let Some(obj) = app.scene.get_object_mut(id) {
        match app.axis_constraint {
            AxisConstraint::None | AxisConstraint::X | AxisConstraint::Y | AxisConstraint::Z => {
                // Uniform scale (the object only supports uniform scale).
                // Moving right/up from the start point scales up; left/down scales down.
                let diagonal = (current.x - start.x) + (start.y - current.y);
                let signed_factor = if diagonal >= 0.0 { factor } else { 1.0 / factor };
                obj.scale = (origin_scale * signed_factor).clamp(0.01, 50.0);
            }
        }
    }
}

/// Apply rotation based on angle from start to current relative to the viewport center.
fn apply_rotate(
    app: &mut ManimStudio,
    id: &str,
    start: Pos2,
    current: Pos2,
    avail: Rect,
) {
    let center = avail.center();
    let start_angle = (start.y - center.y).atan2(start.x - center.x);
    let current_angle = (current.y - center.y).atan2(current.x - center.x);
    let delta_angle = (current_angle - start_angle).to_degrees();

    if let Some(obj) = app.scene.get_object_mut(id) {
        obj.rotation = (app.mode_origin_rotation - delta_angle) % 360.0;
    }
}

/// Draw the interaction mode overlay (mode indicator, axis guide line).
fn draw_mode_overlay(app: &ManimStudio, painter: &Painter, avail: Rect, _is_3d: bool) {
    let mode_label = match app.interaction_mode {
        InteractionMode::Grab   => "G: Grab",
        InteractionMode::Scale  => "S: Scale",
        InteractionMode::Rotate => "R: Rotate",
        InteractionMode::Normal => return,
    };

    let axis_label = match app.axis_constraint {
        AxisConstraint::None => "",
        AxisConstraint::X    => " > X",
        AxisConstraint::Y    => " > Y",
        AxisConstraint::Z    => " > Z",
    };

    let axis_color = match app.axis_constraint {
        AxisConstraint::None => Color32::from_rgb(255, 200, 50),
        AxisConstraint::X    => Color32::from_rgb(230, 70, 70),
        AxisConstraint::Y    => Color32::from_rgb(70, 210, 70),
        AxisConstraint::Z    => Color32::from_rgb(80, 140, 255),
    };

    // Mode indicator badge
    let badge_pos = avail.center_top() + Vec2::new(0.0, 30.0);
    let text = format!("{}{}", mode_label, axis_label);

    // Background for badge
    let text_rect = Rect::from_center_size(
        badge_pos,
        Vec2::new(text.len() as f32 * 8.0 + 24.0, 28.0),
    );
    painter.rect_filled(text_rect, 6.0, Color32::from_rgba_premultiplied(0, 0, 0, 180));
    painter.rect_stroke(text_rect, 6.0, Stroke::new(1.5, axis_color));

    painter.text(
        badge_pos,
        egui::Align2::CENTER_CENTER,
        &text,
        egui::FontId::proportional(14.0),
        axis_color,
    );

    // Draw axis guide line through the object if axis-constrained
    if app.axis_constraint != AxisConstraint::None {
        if let Some(id) = &app.selected_obj {
            if let Some(obj) = app.scene.get_object(id) {
                let guide_len = AXIS_GUIDE_LENGTH;
                let obj_screen_pos = if app.scene.is_3d {
                    let proj = Projection3D::new(&app.cam3d, avail.center(), app.cam3d.pan);
                    proj.project(obj.position)
                } else {
                    let canvas_center = avail.center() + app.vp_pan;
                    m2s(egui::pos2(obj.position[0], obj.position[1]), canvas_center, app.vp_zoom)
                };

                // Compute axis direction on screen
                let axis_dir = if app.scene.is_3d {
                    let proj = Projection3D::new(&app.cam3d, avail.center(), app.cam3d.pan);
                    let axis_vec = match app.axis_constraint {
                        AxisConstraint::X => [1.0, 0.0, 0.0],
                        AxisConstraint::Y => [0.0, 1.0, 0.0],
                        AxisConstraint::Z => [0.0, 0.0, 1.0],
                        AxisConstraint::None => return,
                    };
                    let p0 = proj.project(obj.position);
                    let p1 = proj.project([
                        obj.position[0] + axis_vec[0],
                        obj.position[1] + axis_vec[1],
                        obj.position[2] + axis_vec[2],
                    ]);
                    (p1 - p0).normalized()
                } else {
                    match app.axis_constraint {
                        AxisConstraint::X => Vec2::new(1.0, 0.0),
                        AxisConstraint::Y => Vec2::new(0.0, -1.0),
                        AxisConstraint::Z => Vec2::new(0.0, -1.0),  // Z acts like Y in 2D
                        AxisConstraint::None => return,
                    }
                };

                let p1 = obj_screen_pos - axis_dir * guide_len;
                let p2 = obj_screen_pos + axis_dir * guide_len;
                painter.line_segment(
                    [p1, p2],
                    Stroke::new(1.5, Color32::from_rgba_premultiplied(
                        axis_color.r(), axis_color.g(), axis_color.b(), 100,
                    )),
                );
            }
        }
    }
}

/// Return a human-readable name for well-known camera presets.
fn view_preset_name(phi: f32, theta: f32) -> &'static str {
    // Use approximate matching (within 2 degrees)
    let close = |a: f32, b: f32| (a - b).abs() < 2.0;
    if close(phi, 90.0) && close(theta, 0.0) {
        "Front"
    } else if close(phi, 90.0) && close(theta, -90.0) {
        "Right"
    } else if close(phi, 1.0) && close(theta, 0.0) {
        "Top"
    } else if close(phi, 70.0) && close(theta, -45.0) {
        "Persp"
    } else {
        ""
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 2D helpers
// ─────────────────────────────────────────────────────────────────────────────

fn draw_grid(painter: &Painter, rect: Rect, center: Pos2, zoom: f32) {
    let x_start = ((rect.left() - center.x) / zoom).floor() as i32 - 1;
    let x_end   = ((rect.right() - center.x) / zoom).ceil() as i32 + 1;
    let y_start = ((rect.top() - center.y) / zoom).floor() as i32 - 1;
    let y_end   = ((rect.bottom() - center.y) / zoom).ceil() as i32 + 1;

    for xi in x_start..=x_end {
        let sx = center.x + xi as f32 * zoom;
        let color = if xi == 0 { AXIS_COLOR } else if xi % 2 == 0 { GRID_MAJOR } else { GRID_COLOR };
        painter.line_segment(
            [Pos2::new(sx, rect.top()), Pos2::new(sx, rect.bottom())],
            Stroke::new(if xi == 0 { 1.5 } else { 0.5 }, color),
        );
    }
    for yi in y_start..=y_end {
        let sy = center.y + yi as f32 * zoom;
        let color = if yi == 0 { AXIS_COLOR } else if yi % 2 == 0 { GRID_MAJOR } else { GRID_COLOR };
        painter.line_segment(
            [Pos2::new(rect.left(), sy), Pos2::new(rect.right(), sy)],
            Stroke::new(if yi == 0 { 1.5 } else { 0.5 }, color),
        );
    }
}

fn m2s(manim: Pos2, center: Pos2, zoom: f32) -> Pos2 {
    Pos2::new(center.x + manim.x * zoom, center.y - manim.y * zoom)
}

fn s2m(screen: Pos2, center: Pos2, zoom: f32) -> Pos2 {
    Pos2::new((screen.x - center.x) / zoom, -(screen.y - center.y) / zoom)
}

fn approx_r(obj: &ManimObject) -> f32 {
    match &obj.object_type {
        ObjType::Circle { radius }           => *radius,
        ObjType::Square { side_length }      => side_length * 0.707,
        ObjType::Rectangle { width, height } => width.max(*height) * 0.5,
        ObjType::Triangle { side_length }    => side_length * 0.577,
        ObjType::Sphere { radius }           => *radius,
        ObjType::Cube { side_length }        => side_length * 0.866,
        ObjType::Cylinder { radius, .. }     => *radius,
        ObjType::Text { font_size, .. }      => font_size / 48.0,
        ObjType::NumberPlane | ObjType::Axes => 5.0,
        _ => 0.5,
    }
}