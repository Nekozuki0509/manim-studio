use crate::app::{AxisConstraint, InteractionMode, ManimStudio};
use crate::scene::{compute_display_state, DisplayState, ManimObject, ObjType};
use egui::{Color32, Context, Painter, Pos2, Rect, Stroke, Vec2};

// ─────────────────────────────────────────────────────────────────────────────
// Grid colors
// ─────────────────────────────────────────────────────────────────────────────

const GRID_COLOR: Color32 = Color32::from_rgb(40, 42, 50);
const GRID_MAJOR: Color32 = Color32::from_rgb(55, 58, 68);
const AXIS_COLOR: Color32 = Color32::from_rgb(70, 75, 90);

/// Conversion factor between Manim font_size units and rendering scale.
const FONT_SIZE_SCALE: f32 = 48.0;
/// cos(30°) = √3/2 ≈ 0.866, used for equilateral triangle geometry.
const COS_30: f32 = 0.866;
/// sin(30°) = 0.5, used for equilateral triangle geometry.
const SIN_30: f32 = 0.5;

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
    // Make the grid effectively infinite: extend grid lines far enough to always
    // cover the visible viewport regardless of zoom or pan.  The constant 3000
    // was chosen so that the grid edge is off-screen even at the maximum zoom-out
    // level supported by the camera (zoom ≈ 1..3000).
    const GRID_COVERAGE_PX: f32 = 3000.0;
    let range = ((GRID_COVERAGE_PX / proj.zoom) as i32).max(50);
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
            if selected {
                // Use the projected polygon extent for selection highlight
                let extent = pts.iter().map(|p| (center - *p).length()).fold(0.0_f32, f32::max);
                painter.circle_stroke(center, extent + 3.0, sel_s);
            }
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
                [-r * COS_30, -r * SIN_30],
                [r * COS_30, -r * SIN_30],
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
        // Text / MathTex: render as a flat plane in XY, properly projected in 3D.
        // The text content is drawn along the projected X-axis of the plane
        // (NOT as a screen-aligned billboard).
        ObjType::Text { content, font_size } => {
            let char_w = font_size * s / FONT_SIZE_SCALE * 0.5;
            let text_w = char_w * content.len() as f32;
            let text_h = font_size * s / FONT_SIZE_SCALE * 0.8;
            let hw = text_w / 2.0;
            let hh = text_h / 2.0;
            let corners: Vec<Pos2> = vec![
                [-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh],
            ].iter().map(|[dx, dy]| {
                proj.project([pos3[0] + dx, pos3[1] + dy, pos3[2]])
            }).collect();
            // Draw the text background plane lying flat in XY
            painter.add(egui::Shape::convex_polygon(
                corners.clone(),
                Color32::from_rgba_premultiplied(0, 0, 0, 30),
                Stroke::new(0.5, stroke_col),
            ));
            // Render the text string within the projected plane using per-character
            // positioning along the plane's local X axis, so it rotates with the view.
            draw_text_on_plane(painter, proj, content, pos3, text_w, stroke_col);
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
            draw_text_on_plane(painter, proj, content, pos3, text_w, stroke_col);
            if selected {
                painter.add(egui::Shape::convex_polygon(corners, Color32::TRANSPARENT, sel_s));
            }
        }
        // Fallback — draw projected shape at position for types with simple geometry
        _ => {
            // For types with specific 3D projectable geometry, handle them:
            match &obj.object_type {
                ObjType::Ellipse { width, height } => {
                    let n = 48;
                    let hw = width * s / 2.0;
                    let hh = height * s / 2.0;
                    let pts: Vec<Pos2> = (0..n).map(|i| {
                        let angle = std::f32::consts::TAU * i as f32 / n as f32;
                        proj.project([pos3[0] + hw * angle.cos(), pos3[1] + hh * angle.sin(), pos3[2]])
                    }).collect();
                    if pts.len() >= 3 {
                        painter.add(egui::Shape::convex_polygon(pts.clone(), fill, stroke));
                    }
                    if selected {
                        let extent = pts.iter().map(|p| (center - *p).length()).fold(0.0_f32, f32::max);
                        painter.circle_stroke(center, extent + 3.0, sel_s);
                    }
                }
                ObjType::RegularPolygon { n, radius } => {
                    let r = radius * s;
                    let nn = (*n).max(3) as usize;
                    let pts: Vec<Pos2> = (0..nn).map(|i| {
                        let angle = std::f32::consts::TAU * i as f32 / nn as f32 - std::f32::consts::FRAC_PI_2;
                        proj.project([pos3[0] + r * angle.cos(), pos3[1] + r * angle.sin(), pos3[2]])
                    }).collect();
                    painter.add(egui::Shape::convex_polygon(pts.clone(), fill, stroke));
                    if selected { painter.add(egui::Shape::convex_polygon(pts, Color32::TRANSPARENT, sel_s)); }
                }
                ObjType::Star { n, outer_radius, inner_radius } => {
                    let ro = outer_radius * s;
                    let ri = inner_radius * s;
                    let nn = (*n).max(3) as usize;
                    let pts: Vec<Pos2> = (0..nn*2).map(|i| {
                        let angle = std::f32::consts::TAU * i as f32 / (nn * 2) as f32 - std::f32::consts::FRAC_PI_2;
                        let r = if i % 2 == 0 { ro } else { ri };
                        proj.project([pos3[0] + r * angle.cos(), pos3[1] + r * angle.sin(), pos3[2]])
                    }).collect();
                    painter.add(egui::Shape::convex_polygon(pts.clone(), fill, stroke));
                    if selected { painter.add(egui::Shape::convex_polygon(pts, Color32::TRANSPARENT, sel_s)); }
                }
                ObjType::RoundedRectangle { width, height, .. } => {
                    let hw = width * s / 2.0;
                    let hh = height * s / 2.0;
                    let corners: Vec<Pos2> = vec![
                        [-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh],
                    ].iter().map(|[dx, dy]| proj.project([pos3[0]+dx, pos3[1]+dy, pos3[2]])).collect();
                    painter.add(egui::Shape::convex_polygon(corners.clone(), fill, stroke));
                    if selected { painter.add(egui::Shape::convex_polygon(corners, Color32::TRANSPARENT, sel_s)); }
                }
                ObjType::Annulus { inner_radius, outer_radius } => {
                    let n = 48;
                    let pts_outer: Vec<Pos2> = (0..n).map(|i| {
                        let angle = std::f32::consts::TAU * i as f32 / n as f32;
                        proj.project([pos3[0] + outer_radius * s * angle.cos(), pos3[1] + outer_radius * s * angle.sin(), pos3[2]])
                    }).collect();
                    let pts_inner: Vec<Pos2> = (0..n).map(|i| {
                        let angle = std::f32::consts::TAU * i as f32 / n as f32;
                        proj.project([pos3[0] + inner_radius * s * angle.cos(), pos3[1] + inner_radius * s * angle.sin(), pos3[2]])
                    }).collect();
                    painter.add(egui::Shape::convex_polygon(pts_outer.clone(), fill, stroke));
                    for i in 0..n { painter.line_segment([pts_inner[i], pts_inner[(i+1)%n]], stroke); }
                    if selected {
                        let extent = pts_outer.iter().map(|p| (center - *p).length()).fold(0.0_f32, f32::max);
                        painter.circle_stroke(center, extent + 3.0, sel_s);
                    }
                }
                ObjType::Sector { radius, start_angle, angle } => {
                    let r = radius * s;
                    let sa = start_angle.to_radians();
                    let a = angle.to_radians();
                    let n = 24;
                    let mut pts = vec![center];
                    for i in 0..=n {
                        let t = sa + a * i as f32 / n as f32;
                        pts.push(proj.project([pos3[0] + r * t.cos(), pos3[1] + r * t.sin(), pos3[2]]));
                    }
                    painter.add(egui::Shape::convex_polygon(pts.clone(), fill, stroke));
                    if selected { painter.circle_stroke(center, r * z + 3.0, sel_s); }
                }
                ObjType::Arc { radius, start_angle, angle } => {
                    let r = radius * s;
                    let sa = start_angle.to_radians();
                    let a = angle.to_radians();
                    let n = 32;
                    let pts: Vec<Pos2> = (0..=n).map(|i| {
                        let t = sa + a * i as f32 / n as f32;
                        proj.project([pos3[0] + r * t.cos(), pos3[1] + r * t.sin(), pos3[2]])
                    }).collect();
                    for i in 0..n as usize { painter.line_segment([pts[i], pts[i+1]], stroke); }
                    if selected { painter.circle_stroke(center, r * z + 3.0, sel_s); }
                }
                ObjType::DashedLine { start, end, dash_length } => {
                    let p1 = proj.project(*start);
                    let p2 = proj.project(*end);
                    let total = (p2 - p1).length();
                    let dash_px = dash_length * z;
                    let dir = (p2 - p1).normalized();
                    let mut t = 0.0;
                    let mut drawing = true;
                    while t < total {
                        let next_t = (t + dash_px).min(total);
                        if drawing { painter.line_segment([p1 + dir * t, p1 + dir * next_t], stroke); }
                        t = next_t;
                        drawing = !drawing;
                    }
                    if selected { painter.circle_filled(p1, 4.0, sel_s.color); painter.circle_filled(p2, 4.0, sel_s.color); }
                }
                ObjType::DoubleArrow { start, end } => {
                    let p1 = proj.project(*start);
                    let p2 = proj.project(*end);
                    painter.line_segment([p1, p2], stroke);
                    let dir = (p2 - p1).normalized();
                    let perp = Vec2::new(-dir.y, dir.x);
                    let hs = 12.0_f32;
                    painter.add(egui::Shape::convex_polygon(vec![p2, p2-dir*hs+perp*hs*0.4, p2-dir*hs-perp*hs*0.4], stroke_col, Stroke::NONE));
                    painter.add(egui::Shape::convex_polygon(vec![p1, p1+dir*hs+perp*hs*0.4, p1+dir*hs-perp*hs*0.4], stroke_col, Stroke::NONE));
                    if selected { painter.circle_stroke(center, (p2-p1).length()/2.0+3.0, sel_s); }
                }
                ObjType::Vector { direction } => {
                    let p2 = proj.project([pos3[0]+direction[0], pos3[1]+direction[1], pos3[2]+direction[2]]);
                    painter.line_segment([center, p2], stroke);
                    let dir = (p2 - center).normalized();
                    let perp = Vec2::new(-dir.y, dir.x);
                    let hs = 12.0_f32;
                    painter.add(egui::Shape::convex_polygon(vec![p2, p2-dir*hs+perp*hs*0.4, p2-dir*hs-perp*hs*0.4], stroke_col, Stroke::NONE));
                    if selected { painter.circle_stroke(p2, hs+3.0, sel_s); }
                }
                ObjType::Arrow3D { start, end } => {
                    let p1 = proj.project(*start);
                    let p2 = proj.project(*end);
                    painter.line_segment([p1, p2], stroke);
                    let dir = (p2 - p1).normalized();
                    let perp = Vec2::new(-dir.y, dir.x);
                    let hs = 14.0_f32;
                    painter.add(egui::Shape::convex_polygon(vec![p2, p2-dir*hs+perp*hs*0.4, p2-dir*hs-perp*hs*0.4], stroke_col, Stroke::NONE));
                    if selected { painter.circle_stroke(p2, hs+3.0, sel_s); }
                }
                ObjType::Line3D { start, end } => {
                    let p1 = proj.project(*start);
                    let p2 = proj.project(*end);
                    painter.line_segment([p1, p2], stroke);
                    if selected { painter.circle_filled(p1, 4.0, sel_s.color); painter.circle_filled(p2, 4.0, sel_s.color); }
                }
                ObjType::Cone { radius, height } => {
                    let tip = proj.project([pos3[0], pos3[1], pos3[2] + height * s]);
                    let n = 24;
                    let base_pts: Vec<Pos2> = (0..n).map(|i| {
                        let angle = std::f32::consts::TAU * i as f32 / n as f32;
                        proj.project([pos3[0] + radius * s * angle.cos(), pos3[1] + radius * s * angle.sin(), pos3[2]])
                    }).collect();
                    // Draw base circle
                    for i in 0..n { painter.line_segment([base_pts[i], base_pts[(i+1)%n]], stroke); }
                    // Draw lines from tip to base
                    for i in (0..n).step_by(3) { painter.line_segment([tip, base_pts[i]], stroke); }
                    if selected { painter.circle_stroke(center, approx_r(obj) * z + 3.0, sel_s); }
                }
                ObjType::Torus { major_radius, minor_radius } => {
                    let n = 48;
                    // Outer ring
                    let outer: Vec<Pos2> = (0..n).map(|i| {
                        let angle = std::f32::consts::TAU * i as f32 / n as f32;
                        let r = (major_radius + minor_radius) * s;
                        proj.project([pos3[0] + r * angle.cos(), pos3[1] + r * angle.sin(), pos3[2]])
                    }).collect();
                    let inner: Vec<Pos2> = (0..n).map(|i| {
                        let angle = std::f32::consts::TAU * i as f32 / n as f32;
                        let r = (major_radius - minor_radius) * s;
                        proj.project([pos3[0] + r * angle.cos(), pos3[1] + r * angle.sin(), pos3[2]])
                    }).collect();
                    painter.add(egui::Shape::convex_polygon(outer.clone(), fill, stroke));
                    for i in 0..n { painter.line_segment([inner[i], inner[(i+1)%n]], stroke); }
                    if selected { let ext = outer.iter().map(|p| (center-*p).length()).fold(0.0_f32, f32::max); painter.circle_stroke(center, ext+3.0, sel_s); }
                }
                ObjType::Prism { width, height, depth } => {
                    let hw = width * s / 2.0;
                    let hh = height * s / 2.0;
                    let hd = depth * s / 2.0;
                    let corners_3d: Vec<[f32;3]> = vec![
                        [-hw,-hh,-hd], [ hw,-hh,-hd], [ hw, hh,-hd], [-hw, hh,-hd],
                        [-hw,-hh, hd], [ hw,-hh, hd], [ hw, hh, hd], [-hw, hh, hd],
                    ].iter().map(|[dx,dy,dz]| [pos3[0]+dx, pos3[1]+dy, pos3[2]+dz]).collect();
                    let c2d: Vec<Pos2> = corners_3d.iter().map(|p| proj.project(*p)).collect();
                    let edges = [(0,1),(1,2),(2,3),(3,0),(4,5),(5,6),(6,7),(7,4),(0,4),(1,5),(2,6),(3,7)];
                    for (a,b) in edges { painter.line_segment([c2d[a], c2d[b]], stroke); }
                    if selected { painter.circle_stroke(center, approx_r(obj) * z + 3.0, sel_s); }
                }
                ObjType::DecimalNumber { number, num_decimal_places } => {
                    let text = format!("{:.1$}", number, *num_decimal_places as usize);
                    let text_w = 0.3 * s * text.len() as f32;
                    draw_text_on_plane(painter, proj, &text, pos3, text_w, stroke_col);
                    if selected { painter.circle_stroke(center, text_w * z / 2.0 + 3.0, sel_s); }
                }
                ObjType::Integer { number } => {
                    let text = format!("{}", number);
                    let text_w = 0.3 * s * text.len() as f32;
                    draw_text_on_plane(painter, proj, &text, pos3, text_w, stroke_col);
                    if selected { painter.circle_stroke(center, text_w * z / 2.0 + 3.0, sel_s); }
                }
                // Generic fallback for anything else
                _ => {
                    let r = approx_r(obj) * s * z;
                    painter.circle(center, r.max(4.0), fill, stroke);
                    if selected { painter.circle_stroke(center, r+3.0, sel_s); }
                }
            }
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

/// Render text content as per-character glyphs positioned along the XY plane
/// so that the text rotates with the 3D view instead of staying billboard.
fn draw_text_on_plane(
    painter: &Painter,
    proj: &Projection3D,
    content: &str,
    pos3: [f32; 3],
    text_w: f32,
    color: Color32,
) {
    let n = content.chars().count().max(1);
    let char_w = text_w / n as f32;
    let start_x = pos3[0] - text_w / 2.0 + char_w / 2.0;

    // Compute projected character width on screen to derive font size
    let p_left = proj.project([pos3[0] - text_w / 2.0, pos3[1], pos3[2]]);
    let p_right = proj.project([pos3[0] + text_w / 2.0, pos3[1], pos3[2]]);
    let proj_w = (p_right - p_left).length();
    let fs = (proj_w / n as f32 * 1.6).clamp(6.0, 60.0);

    // Compute the screen-space angle of the text baseline
    let dir = p_right - p_left;
    let angle = dir.y.atan2(dir.x);

    // Use galley with rotation to render each character along the plane direction
    for (i, ch) in content.chars().enumerate() {
        let cx = start_x + i as f32 * char_w;
        let sp = proj.project([cx, pos3[1], pos3[2]]);

        let galley = painter.layout_no_wrap(
            ch.to_string(),
            egui::FontId::proportional(fs),
            color,
        );
        let gw = galley.size().x;
        let gh = galley.size().y;

        // Translate so the glyph center is at `sp`, then rotate by `angle`
        let shape = egui::Shape::Text(egui::epaint::TextShape {
            pos: egui::pos2(sp.x - gw / 2.0, sp.y - gh / 2.0),
            galley,
            underline: Stroke::NONE,
            fallback_color: color,
            override_text_color: Some(color),
            opacity_factor: 1.0,
            angle,
        });
        painter.add(shape);
    }
}

/// Render text in 2D with rotation around its center.
/// When `angle` is 0 the text is horizontal. Non-zero angles rotate around
/// the text center so the text direction matches the object's rotation.
fn draw_text_2d_rotated(
    painter: &Painter,
    center: Pos2,
    content: &str,
    font_size: f32,
    angle: f32,
    color: Color32,
) {
    if font_size < 0.5 { return; }
    // Clamp rendered font size for readability; we still position correctly
    let fs = font_size.clamp(4.0, 200.0);
    let galley = painter.layout_no_wrap(
        content.to_string(),
        egui::FontId::proportional(fs),
        color,
    );
    let gw = galley.size().x;
    let gh = galley.size().y;

    let shape = egui::Shape::Text(egui::epaint::TextShape {
        pos: egui::pos2(center.x - gw / 2.0, center.y - gh / 2.0),
        galley,
        underline: Stroke::NONE,
        fallback_color: color,
        override_text_color: Some(color),
        opacity_factor: 1.0,
        angle,
    });
    painter.add(shape);
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
                Pos2::new(pos.x - r*COS_30, pos.y + r*SIN_30),
                Pos2::new(pos.x + r*COS_30, pos.y + r*SIN_30),
            ];
            painter.add(egui::Shape::convex_polygon(pts.clone(), fill, stroke));
            if selected { painter.add(egui::Shape::convex_polygon(pts, Color32::TRANSPARENT, sel)); }
        }
        ObjType::Text { content, font_size } => {
            // Scale text size proportionally with zoom (no tight upper clamp)
            let fs = (font_size * s * z / FONT_SIZE_SCALE * 18.0).max(1.0);
            let angle = ds.rotation.to_radians();
            draw_text_2d_rotated(painter, pos, content, fs, angle, stroke_col);
            if selected {
                let tr = Rect::from_center_size(pos,
                    Vec2::new(fs * content.chars().count() as f32 * 0.55, fs * 1.3));
                painter.rect_stroke(tr.expand(4.0), 3.0, sel);
            }
        }
        ObjType::MathTex { content } => {
            let fs = (18.0 * s * z / 60.0).max(1.0);
            let angle = ds.rotation.to_radians();
            draw_text_2d_rotated(painter, pos, content, fs, angle, stroke_col);
            if selected {
                let tr = Rect::from_center_size(pos,
                    Vec2::new(fs * content.chars().count() as f32 * 0.65, fs * 1.3));
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
        ObjType::Ellipse { width, height } => {
            let n = 48;
            let hw = width * s * z / 2.0;
            let hh = height * s * z / 2.0;
            let pts: Vec<Pos2> = (0..n).map(|i| {
                let angle = std::f32::consts::TAU * i as f32 / n as f32;
                Pos2::new(pos.x + hw * angle.cos(), pos.y - hh * angle.sin())
            }).collect();
            painter.add(egui::Shape::convex_polygon(pts.clone(), fill, stroke));
            if selected {
                let extent = hw.max(hh);
                painter.circle_stroke(pos, extent + 3.0, sel);
            }
        }
        ObjType::Arc { radius, start_angle, angle } => {
            let r = radius * s * z;
            let sa = start_angle.to_radians();
            let a = angle.to_radians();
            let n = 32;
            let pts: Vec<Pos2> = (0..=n).map(|i| {
                let t = sa + a * i as f32 / n as f32;
                Pos2::new(pos.x + r * t.cos(), pos.y - r * t.sin())
            }).collect();
            for i in 0..n as usize {
                painter.line_segment([pts[i], pts[i+1]], stroke);
            }
            if selected { painter.circle_stroke(pos, r + 3.0, sel); }
        }
        ObjType::ArcBetweenPoints { start, end, angle: _ } => {
            let p1 = m2s(egui::pos2(start[0], start[1]), center, zoom);
            let p2 = m2s(egui::pos2(end[0], end[1]), center, zoom);
            // Draw as a simple curved arc between the two points
            let mid = Pos2::new((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0);
            let perp = Vec2::new(-(p2.y - p1.y), p2.x - p1.x).normalized();
            let bulge = (p2 - p1).length() * 0.3;
            let control = mid + perp * bulge;
            let n = 24;
            let pts: Vec<Pos2> = (0..=n).map(|i| {
                let t = i as f32 / n as f32;
                let mt = 1.0 - t;
                Pos2::new(
                    mt*mt*p1.x + 2.0*mt*t*control.x + t*t*p2.x,
                    mt*mt*p1.y + 2.0*mt*t*control.y + t*t*p2.y,
                )
            }).collect();
            for i in 0..n as usize {
                painter.line_segment([pts[i], pts[i+1]], stroke);
            }
            if selected {
                painter.circle_filled(p1, 4.0, sel.color);
                painter.circle_filled(p2, 4.0, sel.color);
            }
        }
        ObjType::Annulus { inner_radius, outer_radius } => {
            let ro = outer_radius * s * z;
            let ri = inner_radius * s * z;
            painter.circle(pos, ro, fill, stroke);
            // Punch out the inner circle with background color
            painter.circle_filled(pos, ri, Color32::from_rgb(15, 15, 18));
            painter.circle_stroke(pos, ri, stroke);
            if selected { painter.circle_stroke(pos, ro + 3.0, sel); }
        }
        ObjType::Sector { radius, start_angle, angle } => {
            let r = radius * s * z;
            let sa = start_angle.to_radians();
            let a = angle.to_radians();
            let n = 24;
            let mut pts = vec![pos];
            for i in 0..=n {
                let t = sa + a * i as f32 / n as f32;
                pts.push(Pos2::new(pos.x + r * t.cos(), pos.y - r * t.sin()));
            }
            painter.add(egui::Shape::convex_polygon(pts.clone(), fill, stroke));
            if selected { painter.circle_stroke(pos, r + 3.0, sel); }
        }
        ObjType::RegularPolygon { n, radius } => {
            let r = radius * s * z;
            let nn = (*n).max(3) as usize;
            let pts: Vec<Pos2> = (0..nn).map(|i| {
                let angle = std::f32::consts::TAU * i as f32 / nn as f32 - std::f32::consts::FRAC_PI_2;
                Pos2::new(pos.x + r * angle.cos(), pos.y + r * angle.sin())
            }).collect();
            painter.add(egui::Shape::convex_polygon(pts.clone(), fill, stroke));
            if selected { painter.add(egui::Shape::convex_polygon(pts, Color32::TRANSPARENT, sel)); }
        }
        ObjType::Star { n, outer_radius, inner_radius } => {
            let ro = outer_radius * s * z;
            let ri = inner_radius * s * z;
            let nn = (*n).max(3) as usize;
            let pts: Vec<Pos2> = (0..nn*2).map(|i| {
                let angle = std::f32::consts::TAU * i as f32 / (nn * 2) as f32 - std::f32::consts::FRAC_PI_2;
                let r = if i % 2 == 0 { ro } else { ri };
                Pos2::new(pos.x + r * angle.cos(), pos.y + r * angle.sin())
            }).collect();
            painter.add(egui::Shape::convex_polygon(pts.clone(), fill, stroke));
            if selected { painter.add(egui::Shape::convex_polygon(pts, Color32::TRANSPARENT, sel)); }
        }
        ObjType::RoundedRectangle { width, height, corner_radius } => {
            let r = Rect::from_center_size(pos, Vec2::new(width*s*z, height*s*z));
            let cr = corner_radius * s * z;
            painter.rect(r, cr, fill, stroke);
            if selected { painter.rect_stroke(r.expand(3.0), cr, sel); }
        }
        ObjType::DashedLine { start, end, dash_length } => {
            let p1 = m2s(egui::pos2(start[0], start[1]), center, zoom);
            let p2 = m2s(egui::pos2(end[0], end[1]), center, zoom);
            let total = (p2 - p1).length();
            let dash_px = dash_length * z;
            let dir = (p2 - p1).normalized();
            let mut t = 0.0;
            let mut drawing = true;
            while t < total {
                let next_t = (t + dash_px).min(total);
                if drawing {
                    let a = p1 + dir * t;
                    let b = p1 + dir * next_t;
                    painter.line_segment([a, b], stroke);
                }
                t = next_t;
                drawing = !drawing;
            }
            if selected {
                painter.circle_filled(p1, 4.0, sel.color);
                painter.circle_filled(p2, 4.0, sel.color);
            }
        }
        ObjType::DoubleArrow { start, end } => {
            let p1 = m2s(egui::pos2(start[0], start[1]), center, zoom);
            let p2 = m2s(egui::pos2(end[0], end[1]), center, zoom);
            painter.line_segment([p1, p2], stroke);
            let dir = (p2 - p1).normalized();
            let perp = Vec2::new(-dir.y, dir.x);
            let hs = 12.0_f32;
            // Arrowhead at p2
            painter.add(egui::Shape::convex_polygon(
                vec![p2, p2 - dir*hs + perp*hs*0.4, p2 - dir*hs - perp*hs*0.4],
                stroke_col, Stroke::NONE));
            // Arrowhead at p1
            painter.add(egui::Shape::convex_polygon(
                vec![p1, p1 + dir*hs + perp*hs*0.4, p1 + dir*hs - perp*hs*0.4],
                stroke_col, Stroke::NONE));
            if selected { painter.circle_stroke(pos, (p2-p1).length()/2.0+3.0, sel); }
        }
        ObjType::Vector { direction } => {
            let p1 = pos;
            let p2 = m2s(egui::pos2(ds.position[0]+direction[0], ds.position[1]+direction[1]), center, zoom);
            painter.line_segment([p1, p2], stroke);
            let dir = (p2 - p1).normalized();
            let perp = Vec2::new(-dir.y, dir.x);
            let hs = 12.0_f32;
            painter.add(egui::Shape::convex_polygon(
                vec![p2, p2 - dir*hs + perp*hs*0.4, p2 - dir*hs - perp*hs*0.4],
                stroke_col, Stroke::NONE));
            if selected { painter.circle_stroke(p2, hs+3.0, sel); }
        }
        ObjType::Brace { direction: _, length } => {
            let l = length * s * z;
            let p1 = Pos2::new(pos.x - l/2.0, pos.y);
            let p2 = Pos2::new(pos.x + l/2.0, pos.y);
            let mid = Pos2::new(pos.x, pos.y - l * 0.15);
            // Simple curly brace approximation
            painter.line_segment([p1, Pos2::new(p1.x + l*0.1, mid.y)], stroke);
            painter.line_segment([Pos2::new(p1.x + l*0.1, mid.y), mid], stroke);
            painter.line_segment([mid, Pos2::new(p2.x - l*0.1, mid.y)], stroke);
            painter.line_segment([Pos2::new(p2.x - l*0.1, mid.y), p2], stroke);
            if selected { painter.circle_stroke(pos, l/2.0 + 3.0, sel); }
        }
        ObjType::BraceBetweenPoints { start, end } => {
            let p1 = m2s(egui::pos2(start[0], start[1]), center, zoom);
            let p2 = m2s(egui::pos2(end[0], end[1]), center, zoom);
            let mid = Pos2::new((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0);
            let perp = Vec2::new(-(p2.y - p1.y), p2.x - p1.x).normalized() * 10.0;
            painter.line_segment([p1, p1 + perp], stroke);
            painter.line_segment([p1 + perp, mid + perp * 2.0], stroke);
            painter.line_segment([mid + perp * 2.0, p2 + perp], stroke);
            painter.line_segment([p2 + perp, p2], stroke);
            if selected {
                painter.circle_filled(p1, 4.0, sel.color);
                painter.circle_filled(p2, 4.0, sel.color);
            }
        }
        ObjType::Angle { radius, start_angle, angle } => {
            let r = radius * s * z;
            let sa = start_angle.to_radians();
            let a = angle.to_radians();
            let n = 16;
            let pts: Vec<Pos2> = (0..=n).map(|i| {
                let t = sa + a * i as f32 / n as f32;
                Pos2::new(pos.x + r * t.cos(), pos.y - r * t.sin())
            }).collect();
            for i in 0..n as usize {
                painter.line_segment([pts[i], pts[i+1]], stroke);
            }
            // Draw the two angle legs
            let leg_len = r * 1.5;
            painter.line_segment([pos, Pos2::new(pos.x + leg_len * sa.cos(), pos.y - leg_len * sa.sin())],
                Stroke::new(0.8, stroke_col));
            painter.line_segment([pos, Pos2::new(pos.x + leg_len * (sa+a).cos(), pos.y - leg_len * (sa+a).sin())],
                Stroke::new(0.8, stroke_col));
            if selected { painter.circle_stroke(pos, r + 3.0, sel); }
        }
        ObjType::RightAngle { size } => {
            let sz = size * s * z;
            let p1 = Pos2::new(pos.x + sz, pos.y);
            let p2 = Pos2::new(pos.x + sz, pos.y - sz);
            let p3 = Pos2::new(pos.x, pos.y - sz);
            painter.line_segment([pos, p1], Stroke::new(0.8, stroke_col));
            painter.line_segment([pos, p3], Stroke::new(0.8, stroke_col));
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
            if selected { painter.circle_stroke(pos, sz + 3.0, sel); }
        }
        ObjType::NumberLine { x_min, x_max, step } => {
            let left = m2s(egui::pos2(ds.position[0] + x_min, ds.position[1]), center, zoom);
            let right = m2s(egui::pos2(ds.position[0] + x_max, ds.position[1]), center, zoom);
            painter.line_segment([left, right], stroke);
            // Tick marks
            let mut x = *x_min;
            while x <= *x_max + 0.001 {
                let px = m2s(egui::pos2(ds.position[0] + x, ds.position[1]), center, zoom);
                painter.line_segment(
                    [Pos2::new(px.x, px.y - 5.0), Pos2::new(px.x, px.y + 5.0)],
                    stroke);
                x += step;
            }
            if selected {
                painter.circle_filled(left, 4.0, sel.color);
                painter.circle_filled(right, 4.0, sel.color);
            }
        }
        ObjType::BarChart { values, bar_width } => {
            if !values.is_empty() {
                let bw = bar_width * s * z;
                let gap = bw * 0.2;
                let total_w = values.len() as f32 * (bw + gap) - gap;
                let start_x = pos.x - total_w / 2.0;
                for (i, val) in values.iter().enumerate() {
                    let bx = start_x + i as f32 * (bw + gap);
                    let bh = val * s * z;
                    let bar = Rect::from_min_size(
                        Pos2::new(bx, pos.y - bh),
                        Vec2::new(bw, bh),
                    );
                    painter.rect(bar, 0.0, fill, stroke);
                }
                if selected {
                    let max_h = values.iter().fold(0.0_f32, |a, &b| a.max(b)) * s * z;
                    let r = Rect::from_center_size(pos, Vec2::new(total_w + 6.0, max_h + 6.0));
                    painter.rect_stroke(r, 0.0, sel);
                }
            }
        }
        ObjType::DecimalNumber { number, num_decimal_places } => {
            let text = format!("{:.1$}", number, *num_decimal_places as usize);
            let fs = (18.0 * s * z / 60.0 * 14.0).max(1.0);
            let angle = ds.rotation.to_radians();
            draw_text_2d_rotated(painter, pos, &text, fs, angle, stroke_col);
            if selected {
                let tr = Rect::from_center_size(pos, Vec2::new(fs * text.len() as f32 * 0.6, fs * 1.3));
                painter.rect_stroke(tr.expand(4.0), 3.0, sel);
            }
        }
        ObjType::Integer { number } => {
            let text = format!("{}", number);
            let fs = (18.0 * s * z / 60.0 * 14.0).max(1.0);
            let angle = ds.rotation.to_radians();
            draw_text_2d_rotated(painter, pos, &text, fs, angle, stroke_col);
            if selected {
                let tr = Rect::from_center_size(pos, Vec2::new(fs * text.len() as f32 * 0.6, fs * 1.3));
                painter.rect_stroke(tr.expand(4.0), 3.0, sel);
            }
        }
        ObjType::Dot3D => {
            let r = (6.0*s*z/60.0*8.0).max(4.0);
            painter.circle_filled(pos, r, stroke_col);
            // Highlight to suggest 3D
            painter.circle_filled(pos + Vec2::new(-r*0.2, -r*0.2), r*0.25,
                Color32::from_rgba_premultiplied(255,255,255,50));
            if selected { painter.circle_stroke(pos, r+4.0, sel); }
        }
        ObjType::Cone { radius, height } => {
            let r = radius * s * z;
            let h = height * s * z;
            let pts = vec![
                Pos2::new(pos.x, pos.y - h/2.0),       // tip
                Pos2::new(pos.x - r, pos.y + h/2.0),   // base left
                Pos2::new(pos.x + r, pos.y + h/2.0),   // base right
            ];
            painter.add(egui::Shape::convex_polygon(pts.clone(), fill, stroke));
            if selected { painter.add(egui::Shape::convex_polygon(pts, Color32::TRANSPARENT, sel)); }
        }
        ObjType::Torus { major_radius, minor_radius } => {
            let mr = major_radius * s * z;
            let mnr = minor_radius * s * z;
            // Draw outer and inner ellipses
            painter.circle(pos, mr + mnr, fill, stroke);
            painter.circle_stroke(pos, (mr - mnr).max(1.0),
                Stroke::new(1.0, stroke_col));
            if selected { painter.circle_stroke(pos, mr + mnr + 3.0, sel); }
        }
        ObjType::Prism { width, height, depth } => {
            let hw = width * s * z / 2.0;
            let hh = height * s * z / 2.0;
            let d = depth * s * z * 0.3; // perspective offset
            let front = Rect::from_center_size(pos, Vec2::new(hw*2.0, hh*2.0));
            painter.rect(front, 0.0, fill, stroke);
            let back = Rect::from_center_size(pos + Vec2::new(d, -d), Vec2::new(hw*2.0, hh*2.0));
            painter.rect(back, 0.0, Color32::from_rgba_premultiplied(200,200,200,20),
                Stroke::new(0.8, stroke_col));
            // Connect corners
            for (fp, bp) in [(front.left_top(), back.left_top()),
                             (front.right_top(), back.right_top()),
                             (front.right_bottom(), back.right_bottom()),
                             (front.left_bottom(), back.left_bottom())] {
                painter.line_segment([fp, bp], Stroke::new(0.8, stroke_col));
            }
            if selected { painter.rect_stroke(front.expand(3.0), 0.0, sel); }
        }
        ObjType::Arrow3D { start, end } => {
            let p1 = m2s(egui::pos2(start[0], start[1]), center, zoom);
            let p2 = m2s(egui::pos2(end[0], end[1]), center, zoom);
            painter.line_segment([p1, p2], stroke);
            let dir = (p2 - p1).normalized();
            let perp = Vec2::new(-dir.y, dir.x);
            let hs = 14.0_f32;
            painter.add(egui::Shape::convex_polygon(
                vec![p2, p2 - dir*hs + perp*hs*0.4, p2 - dir*hs - perp*hs*0.4],
                stroke_col, Stroke::NONE));
            if selected { painter.circle_stroke(p2, hs+3.0, sel); }
        }
        ObjType::Line3D { start, end } => {
            let p1 = m2s(egui::pos2(start[0], start[1]), center, zoom);
            let p2 = m2s(egui::pos2(end[0], end[1]), center, zoom);
            painter.line_segment([p1, p2], stroke);
            if selected {
                painter.circle_filled(p1, 4.0, sel.color);
                painter.circle_filled(p2, 4.0, sel.color);
            }
        }
        ObjType::Surface => {
            // Wireframe grid for surface preview
            let half = 2.0 * s * z;
            let r = Rect::from_center_size(pos, Vec2::splat(half*2.0));
            painter.rect(r, 0.0, Color32::from_rgba_premultiplied(
                (c[0]*255.0) as u8, (c[1]*255.0) as u8, (c[2]*255.0) as u8, 30),
                stroke);
            for i in -3..=3_i32 {
                let off = i as f32 * z * s * 2.0 / 3.0;
                painter.line_segment(
                    [Pos2::new(pos.x + off, r.top()), Pos2::new(pos.x + off, r.bottom())],
                    Stroke::new(0.4, Color32::from_rgba_premultiplied(
                        (c[0]*255.0) as u8, (c[1]*255.0) as u8, (c[2]*255.0) as u8, 60)));
                painter.line_segment(
                    [Pos2::new(r.left(), pos.y + off), Pos2::new(r.right(), pos.y + off)],
                    Stroke::new(0.4, Color32::from_rgba_premultiplied(
                        (c[0]*255.0) as u8, (c[1]*255.0) as u8, (c[2]*255.0) as u8, 60)));
            }
            if selected { painter.rect_stroke(r.expand(3.0), 0.0, sel); }
        }
        // ── New types: simplified preview renderings ─────────────────────
        ObjType::CurvedArrow { start, end } | ObjType::CurvedDoubleArrow { start, end } => {
            let s0 = m2s(egui::pos2(start[0], start[1]), center, zoom);
            let e0 = m2s(egui::pos2(end[0], end[1]), center, zoom);
            let mid = Pos2::new((s0.x + e0.x) / 2.0, (s0.y + e0.y) / 2.0 - 20.0 * z);
            painter.line_segment([s0, mid], stroke);
            painter.line_segment([mid, e0], stroke);
            if selected { painter.circle_stroke(Pos2::new((s0.x+e0.x)/2.0, (s0.y+e0.y)/2.0), 6.0, sel); }
        }
        ObjType::Elbow => {
            let half = 1.0 * s * z;
            painter.line_segment([Pos2::new(pos.x - half, pos.y), Pos2::new(pos.x - half, pos.y - half)], stroke);
            painter.line_segment([Pos2::new(pos.x - half, pos.y - half), Pos2::new(pos.x, pos.y - half)], stroke);
            if selected { painter.circle_stroke(pos, half + 3.0, sel); }
        }
        ObjType::TangentLine { length, angle } => {
            let half = length * s * z / 2.0;
            let rad = angle.to_radians();
            let dx = half * rad.cos();
            let dy = half * rad.sin();
            painter.line_segment(
                [Pos2::new(pos.x - dx, pos.y + dy), Pos2::new(pos.x + dx, pos.y - dy)],
                stroke,
            );
            if selected { painter.circle_stroke(pos, half + 3.0, sel); }
        }
        ObjType::BraceLabel { direction, length, label } => {
            let half = length * s * z / 2.0;
            let d = Vec2::new(direction[0], -direction[1]).normalized();
            let perp = Vec2::new(-d.y, d.x);
            let s0 = pos + perp * half;
            let e0 = pos - perp * half;
            painter.line_segment([s0, e0], stroke);
            painter.text(pos + d * 10.0, egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(10.0), stroke_col);
            if selected { painter.circle_stroke(pos, half + 3.0, sel); }
        }
        ObjType::LabeledDot { label, radius } => {
            let r = radius * s * z;
            painter.circle(pos, r.max(4.0), fill, stroke);
            painter.text(pos, egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(9.0), stroke_col);
            if selected { painter.circle_stroke(pos, r.max(4.0) + 3.0, sel); }
        }
        ObjType::LabeledLine { label, start, end } => {
            let s0 = m2s(egui::pos2(start[0], start[1]), center, zoom);
            let e0 = m2s(egui::pos2(end[0], end[1]), center, zoom);
            painter.line_segment([s0, e0], stroke);
            let mid = Pos2::new((s0.x + e0.x) / 2.0, (s0.y + e0.y) / 2.0 - 8.0);
            painter.text(mid, egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(10.0), stroke_col);
            if selected { painter.circle_stroke(Pos2::new((s0.x+e0.x)/2.0, (s0.y+e0.y)/2.0), 6.0, sel); }
        }
        ObjType::SurroundingRectangle { buff } => {
            let half = (1.0 + buff) * s * z;
            let r = Rect::from_center_size(pos, Vec2::splat(half * 2.0));
            painter.rect_stroke(r, 2.0, stroke);
            if selected { painter.rect_stroke(r.expand(3.0), 2.0, sel); }
        }
        ObjType::BackgroundRectangle => {
            let half = 1.0 * s * z;
            let r = Rect::from_center_size(pos, Vec2::splat(half * 2.0));
            painter.rect(r, 0.0, Color32::from_rgba_premultiplied(0, 0, 0, 120), Stroke::NONE);
            if selected { painter.rect_stroke(r.expand(3.0), 0.0, sel); }
        }
        ObjType::Underline => {
            let half = 1.5 * s * z;
            painter.line_segment([Pos2::new(pos.x - half, pos.y + 2.0), Pos2::new(pos.x + half, pos.y + 2.0)], stroke);
            if selected { painter.circle_stroke(pos, half + 3.0, sel); }
        }
        ObjType::Cross { scale } => {
            let half = scale * s * z;
            painter.line_segment([Pos2::new(pos.x - half, pos.y - half), Pos2::new(pos.x + half, pos.y + half)], stroke);
            painter.line_segment([Pos2::new(pos.x - half, pos.y + half), Pos2::new(pos.x + half, pos.y - half)], stroke);
            if selected { painter.circle_stroke(pos, half + 3.0, sel); }
        }
        ObjType::FunctionGraph { .. } | ObjType::ParametricFunction { .. }
        | ObjType::ImplicitFunction { .. } => {
            // Draw a wavy line as placeholder
            let half = 2.0 * s * z;
            let points: Vec<Pos2> = (0..20).map(|i| {
                let t = i as f32 / 19.0;
                Pos2::new(pos.x - half + t * half * 2.0, pos.y + (t * 6.28).sin() * half * 0.3)
            }).collect();
            for w in points.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
            if selected { painter.rect_stroke(Rect::from_center_size(pos, Vec2::splat(half * 2.0 + 6.0)), 0.0, sel); }
        }
        ObjType::ComplexPlane | ObjType::PolarPlane | ObjType::CoordinateSystem => {
            // Draw axes cross
            let half = 3.0 * s * z;
            painter.line_segment([Pos2::new(pos.x - half, pos.y), Pos2::new(pos.x + half, pos.y)], stroke);
            painter.line_segment([Pos2::new(pos.x, pos.y - half), Pos2::new(pos.x, pos.y + half)], stroke);
            if selected { painter.rect_stroke(Rect::from_center_size(pos, Vec2::splat(half * 2.0 + 6.0)), 0.0, sel); }
        }
        ObjType::Title { content } => {
            let fs = (20.0 * s * z).max(1.0);
            let angle = ds.rotation.to_radians();
            draw_text_2d_rotated(painter, pos, content, fs, angle, stroke_col);
            if selected { painter.circle_stroke(pos, 20.0, sel); }
        }
        ObjType::MarkupText { content } | ObjType::Paragraph { content } => {
            let fs = (12.0 * s * z).max(1.0);
            let angle = ds.rotation.to_radians();
            draw_text_2d_rotated(painter, pos, content, fs, angle, stroke_col);
            if selected { painter.circle_stroke(pos, 15.0, sel); }
        }
        ObjType::Code { code, .. } => {
            let fs = (10.0 * s * z).max(1.0);
            let first_line = code.lines().next().unwrap_or("code");
            let angle = ds.rotation.to_radians();
            draw_text_2d_rotated(painter, pos, first_line, fs, angle, stroke_col);
            if selected { painter.circle_stroke(pos, 15.0, sel); }
        }
        ObjType::BulletedList { items } => {
            let fid = egui::FontId::proportional(10.0 * s * z);
            for (i, item) in items.iter().take(5).enumerate() {
                let y_off = i as f32 * 14.0 * s * z;
                let label = format!("• {}", item);
                painter.text(Pos2::new(pos.x, pos.y + y_off), egui::Align2::CENTER_CENTER, &label, fid.clone(), stroke_col);
            }
            if selected { painter.circle_stroke(pos, 20.0, sel); }
        }
        ObjType::Table { rows, cols } => {
            let cw = 20.0 * s * z; let rh = 14.0 * s * z;
            let total_w = *cols as f32 * cw; let total_h = *rows as f32 * rh;
            let r = Rect::from_center_size(pos, Vec2::new(total_w, total_h));
            painter.rect_stroke(r, 0.0, stroke);
            for i in 1..*cols { let x = r.left() + i as f32 * cw; painter.line_segment([Pos2::new(x, r.top()), Pos2::new(x, r.bottom())], stroke); }
            for i in 1..*rows { let y = r.top() + i as f32 * rh; painter.line_segment([Pos2::new(r.left(), y), Pos2::new(r.right(), y)], stroke); }
            if selected { painter.rect_stroke(r.expand(3.0), 0.0, sel); }
        }
        ObjType::Matrix { rows, cols } => {
            let cw = 18.0 * s * z; let rh = 14.0 * s * z;
            let total_w = *cols as f32 * cw; let total_h = *rows as f32 * rh;
            let r = Rect::from_center_size(pos, Vec2::new(total_w, total_h));
            // Draw brackets
            let bw = 4.0 * s * z;
            painter.line_segment([Pos2::new(r.left()-bw, r.top()), Pos2::new(r.left(), r.top())], stroke);
            painter.line_segment([Pos2::new(r.left()-bw, r.top()), Pos2::new(r.left()-bw, r.bottom())], stroke);
            painter.line_segment([Pos2::new(r.left()-bw, r.bottom()), Pos2::new(r.left(), r.bottom())], stroke);
            painter.line_segment([Pos2::new(r.right()+bw, r.top()), Pos2::new(r.right(), r.top())], stroke);
            painter.line_segment([Pos2::new(r.right()+bw, r.top()), Pos2::new(r.right()+bw, r.bottom())], stroke);
            painter.line_segment([Pos2::new(r.right()+bw, r.bottom()), Pos2::new(r.right(), r.bottom())], stroke);
            if selected { painter.rect_stroke(r.expand(bw + 3.0), 0.0, sel); }
        }
        ObjType::VGroup { .. } => {
            // Draw grouped circle indicator
            painter.circle_stroke(pos, 8.0 * s * z, stroke);
            painter.circle_stroke(pos + Vec2::new(4.0, -4.0) * s * z, 8.0 * s * z, Stroke::new(stroke.width * 0.5, stroke_col));
            if selected { painter.circle_stroke(pos, 16.0 * s * z, sel); }
        }
        ObjType::Icosahedron { radius } | ObjType::Dodecahedron { radius } => {
            let r = radius * s * z;
            // Approximation: draw hexagon
            let n = if matches!(&obj.object_type, ObjType::Icosahedron { .. }) { 6 } else { 5 };
            let pts: Vec<Pos2> = (0..=n).map(|i| {
                let a = std::f32::consts::TAU * i as f32 / n as f32;
                Pos2::new(pos.x + r * a.cos(), pos.y + r * a.sin())
            }).collect();
            for w in pts.windows(2) { painter.line_segment([w[0], w[1]], stroke); }
            if selected { painter.circle_stroke(pos, r + 3.0, sel); }
        }
        ObjType::TracedPath | ObjType::PointCloudDot => {
            // Dotted line placeholder
            for i in 0..5_i32 {
                let off = (i as f32 - 2.0) * 6.0 * s * z;
                painter.circle(Pos2::new(pos.x + off, pos.y), 2.0 * s * z, fill, Stroke::NONE);
            }
            if selected { painter.circle_stroke(pos, 15.0 * s * z, sel); }
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
        ObjType::Cube { side_length }        => side_length * COS_30,
        ObjType::Cylinder { radius, .. }     => *radius,
        ObjType::Text { font_size, .. }      => font_size / FONT_SIZE_SCALE,
        ObjType::NumberPlane | ObjType::Axes => 5.0,
        ObjType::Ellipse { width, height }   => width.max(*height) * 0.5,
        ObjType::Arc { radius, .. }          => *radius,
        ObjType::Annulus { outer_radius, .. } => *outer_radius,
        ObjType::Sector { radius, .. }       => *radius,
        ObjType::RegularPolygon { radius, .. } => *radius,
        ObjType::Star { outer_radius, .. }   => *outer_radius,
        ObjType::RoundedRectangle { width, height, .. } => width.max(*height) * 0.5,
        ObjType::Cone { radius, height }     => radius.max(*height * 0.5),
        ObjType::Torus { major_radius, minor_radius } => major_radius + minor_radius,
        ObjType::Prism { width, height, .. } => width.max(*height) * 0.5,
        ObjType::NumberLine { x_min, x_max, .. } => (x_max - x_min) * 0.5,
        ObjType::BarChart { values, .. }     => values.len() as f32 * 0.5,
        _ => 0.5,
    }
}