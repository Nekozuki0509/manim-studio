use crate::app::{ManimStudio, TlDrag, TlDragMode};
use egui::{Color32, Context, CursorIcon, Pos2, Rect, Stroke, Vec2};

const TRACK_H: f32 = 26.0;
const HEADER_W: f32 = 150.0;
const RULER_H: f32 = 22.0;
const EDGE_ZONE: f32 = 8.0; // px from each edge = resize handle

pub fn show(app: &mut ManimStudio, ctx: &Context) {
    egui::TopBottomPanel::bottom("timeline")
        .min_height(180.0)
        .max_height(380.0)
        .resizable(true)
        .frame(egui::Frame::none().fill(Color32::from_rgb(20, 20, 25)).inner_margin(0.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("⏱ Timeline")
                        .strong()
                        .color(Color32::from_rgb(180, 180, 200)),
                );
                ui.separator();
                ui.label("Duration:");
                ui.add(egui::Slider::new(&mut app.scene.timeline.duration, 1.0..=120.0).suffix("s"));
                ui.separator();
                ui.label("Zoom:");
                ui.add(egui::Slider::new(&mut app.tl_zoom, 20.0..=300.0).suffix("px/s"));
            });

            let avail = ui.available_rect_before_wrap();
            let (resp, painter) = ui.allocate_painter(avail.size(), egui::Sense::click_and_drag());

            painter.rect_filled(avail, 0.0, Color32::from_rgb(22, 22, 28));

            let zoom = app.tl_zoom;
            let scroll = app.tl_scroll;
            let track_x0 = avail.min.x + HEADER_W;
            let track_y0 = avail.min.y + RULER_H;

            draw_ruler(&painter, avail, zoom, scroll);

            // ── Collect hit tests before mut borrows ──────────────────────────
            // We resolve: what did the mouse press on?
            // Possible outcomes:
            //   - drag start on block center → Move
            //   - drag start on block left edge → ResizeLeft
            //   - drag start on block right edge → ResizeRight
            //   - click on empty area → seek playhead
            //   - click on ruler → seek playhead

            let mouse_pos = resp.interact_pointer_pos();
            let drag_started = resp.drag_started();
            let is_dragging  = resp.dragged();
            let just_released = resp.drag_stopped();
            let clicked = resp.clicked();

            // Build list of (object rows) for hit-testing
            let objects: Vec<(String, String, &str)> = app.scene.objects
                .iter()
                .map(|o| (o.id.clone(), o.name.clone(), o.icon()))
                .collect();

            // ── Track rows ────────────────────────────────────────────────────
            let mut hovered_cursor = CursorIcon::Default;
            let sel_obj  = app.selected_obj.clone();
            let sel_anim = app.selected_anim.clone();

            // We need to decide which anim the drag started on (if any),
            // and whether any block was under a click.
            let mut drag_start_info: Option<(String, TlDragMode, f32, f32, f32)> = None; // (anim_id, mode, orig_start, orig_dur, origin_x)
            let mut click_on_block = false;

            for (row, (obj_id, obj_name, obj_icon)) in objects.iter().enumerate() {
                let y = track_y0 + row as f32 * TRACK_H;
                if y > avail.max.y { break; }

                // Row background
                let row_bg = if sel_obj.as_deref() == Some(obj_id) {
                    Color32::from_rgb(38, 40, 55)
                } else if row % 2 == 0 {
                    Color32::from_rgb(26, 26, 32)
                } else {
                    Color32::from_rgb(30, 30, 38)
                };
                painter.rect_filled(
                    Rect::from_min_size(Pos2::new(avail.min.x, y), Vec2::new(avail.width(), TRACK_H)),
                    0.0, row_bg,
                );

                // Object label
                painter.text(
                    Pos2::new(avail.min.x + 8.0, y + TRACK_H / 2.0),
                    egui::Align2::LEFT_CENTER,
                    format!("{} {}", obj_icon, obj_name),
                    egui::FontId::proportional(12.0),
                    if sel_obj.as_deref() == Some(obj_id) {
                        Color32::from_rgb(255, 220, 80)
                    } else {
                        Color32::from_rgb(190, 190, 200)
                    },
                );

                // Anim blocks
                let anim_ids: Vec<String> = app.scene.animations.iter()
                    .filter(|a| &a.object_id == obj_id)
                    .map(|a| a.id.clone())
                    .collect();

                for anim_id in &anim_ids {
                    let anim = match app.scene.animations.iter().find(|a| &a.id == anim_id) {
                        Some(a) => a.clone(),
                        None => continue,
                    };

                    let bx0 = track_x0 + (anim.start_time - scroll) * zoom;
                    let bx1 = bx0 + anim.duration * zoom;

                    if bx1 < track_x0 || bx0 > avail.max.x { continue; }

                    let vis_x0 = bx0.max(track_x0);
                    let vis_x1 = bx1.min(avail.max.x);
                    let block = Rect::from_min_max(
                        Pos2::new(vis_x0, y + 2.0),
                        Pos2::new(vis_x1, y + TRACK_H - 2.0),
                    );

                    let [r, g, b] = anim.anim_type.track_color();
                    let being_dragged = app.tl_drag.as_ref()
                        .map(|d| &d.anim_id == anim_id).unwrap_or(false);
                    let is_sel = sel_anim.as_deref() == Some(anim_id);

                    painter.rect_filled(block, 3.0,
                        Color32::from_rgba_unmultiplied(r, g, b,
                            if is_sel || being_dragged { 255 } else { 200 }));
                    if is_sel || being_dragged {
                        painter.rect_stroke(block.expand(1.5), 3.0, Stroke::new(2.0, Color32::WHITE));
                    }

                    // Left / right edge handles (visual)
                    for &ex in &[vis_x0, vis_x1 - EDGE_ZONE] {
                        painter.rect_filled(
                            Rect::from_min_size(Pos2::new(ex, y + 2.0), Vec2::new(EDGE_ZONE, TRACK_H - 4.0)),
                            0.0,
                            Color32::from_rgba_unmultiplied(255, 255, 255, 25),
                        );
                    }

                    if block.width() > 30.0 {
                        painter.text(
                            block.center(), egui::Align2::CENTER_CENTER,
                            anim.anim_type.name(),
                            egui::FontId::proportional(10.5),
                            Color32::from_rgba_unmultiplied(255, 255, 255, 220),
                        );
                    }

                    // Hit test
                    if let Some(mp) = mouse_pos {
                        if block.contains(mp) {
                            click_on_block = true;

                            // Determine cursor & mode based on edge proximity
                            let mode = if mp.x <= bx0 + EDGE_ZONE && bx0 >= track_x0 {
                                hovered_cursor = CursorIcon::ResizeHorizontal;
                                TlDragMode::ResizeLeft
                            } else if mp.x >= bx1 - EDGE_ZONE {
                                hovered_cursor = CursorIcon::ResizeHorizontal;
                                TlDragMode::ResizeRight
                            } else {
                                hovered_cursor = CursorIcon::Grab;
                                TlDragMode::Move
                            };

                            if drag_started {
                                drag_start_info = Some((
                                    anim_id.clone(), mode,
                                    anim.start_time, anim.duration,
                                    mp.x,
                                ));
                            }

                            // Click on block → only select, no seek
                            if clicked {
                                // selection handled below after borrow ends
                            }
                        }
                    }
                }

                // Row separator
                painter.line_segment(
                    [Pos2::new(avail.min.x, y + TRACK_H), Pos2::new(avail.max.x, y + TRACK_H)],
                    Stroke::new(0.5, Color32::from_rgb(40, 40, 50)),
                );
            }

            // Apply cursor hint
            if resp.hovered() { ctx.set_cursor_icon(hovered_cursor); }

            // ── Apply drag start ──────────────────────────────────────────────
            if let Some((id, mode, os, od, ox)) = drag_start_info {
                app.selected_anim = Some(id.clone());
                app.tl_drag = Some(TlDrag {
                    anim_id: id,
                    mode,
                    orig_start: os,
                    orig_dur: od,
                    drag_origin_x: ox,
                });
            }

            // ── Apply ongoing drag ────────────────────────────────────────────
            if is_dragging {
                if let Some(drag) = app.tl_drag.clone() {
                    let total_dx = mouse_pos
                        .map(|mp| mp.x - drag.drag_origin_x)
                        .unwrap_or(0.0);
                    let dt = total_dx / zoom;

                    if let Some(anim) = app.scene.animations.iter_mut()
                        .find(|a| a.id == drag.anim_id)
                    {
                        match drag.mode {
                            TlDragMode::Move => {
                                anim.start_time = (drag.orig_start + dt).max(0.0);
                            }
                            TlDragMode::ResizeRight => {
                                anim.duration = (drag.orig_dur + dt).max(0.05);
                            }
                            TlDragMode::ResizeLeft => {
                                // Moving left edge: start moves, duration shrinks
                                let new_start = (drag.orig_start + dt)
                                    .max(0.0)
                                    .min(drag.orig_start + drag.orig_dur - 0.05);
                                let delta_start = new_start - drag.orig_start;
                                anim.start_time = new_start;
                                anim.duration = (drag.orig_dur - delta_start).max(0.05);
                            }
                        }
                    }
                }
            }

            if just_released { app.tl_drag = None; }

            // ── Click on block → select anim only (no seek) ──────────────────
            if clicked && click_on_block {
                if let Some(mp) = mouse_pos {
                    // Find which anim was clicked
                    'outer: for (row, (obj_id, _, _)) in objects.iter().enumerate() {
                        let y = track_y0 + row as f32 * TRACK_H;
                        for anim in app.scene.animations.iter().filter(|a| &a.object_id == obj_id) {
                            let bx0 = track_x0 + (anim.start_time - scroll) * zoom;
                            let bx1 = bx0 + anim.duration * zoom;
                            let block = Rect::from_min_max(
                                Pos2::new(bx0.max(track_x0), y + 2.0),
                                Pos2::new(bx1.min(avail.max.x), y + TRACK_H - 2.0),
                            );
                            if block.contains(mp) {
                                app.selected_anim = Some(anim.id.clone());
                                break 'outer;
                            }
                        }
                    }
                }
            }

            // ── Click on empty area → seek ────────────────────────────────────
            if clicked && !click_on_block {
                if let Some(mp) = mouse_pos {
                    // Ruler or empty track area both seek
                    let t = ((mp.x - track_x0) / zoom + scroll).clamp(0.0, app.scene.timeline.duration);
                    app.scene.timeline.current_time = t;
                }
            }

            // ── Playhead ──────────────────────────────────────────────────────
            let ph_x = track_x0 + (app.scene.timeline.current_time - scroll) * zoom;
            if ph_x >= track_x0 && ph_x <= avail.max.x {
                painter.line_segment(
                    [Pos2::new(ph_x, avail.min.y), Pos2::new(ph_x, avail.max.y)],
                    Stroke::new(2.0, Color32::from_rgb(255, 100, 80)),
                );
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        Pos2::new(ph_x, avail.min.y + RULER_H),
                        Pos2::new(ph_x - 6.0, avail.min.y + 4.0),
                        Pos2::new(ph_x + 6.0, avail.min.y + 4.0),
                    ],
                    Color32::from_rgb(255, 100, 80),
                    Stroke::NONE,
                ));
            }

            // ── Scroll & zoom ─────────────────────────────────────────────────
            if resp.hovered() {
                let sd = ctx.input(|i| i.raw_scroll_delta);
                if sd.x.abs() > 0.5 {
                    app.tl_scroll = (app.tl_scroll - sd.x / zoom)
                        .clamp(0.0, app.scene.timeline.duration.max(1.0));
                }
                if sd.y.abs() > 0.5 && ctx.input(|i| i.modifiers.ctrl) {
                    app.tl_zoom = (app.tl_zoom * (1.0 + sd.y * 0.002)).clamp(10.0, 500.0);
                }
            }

            // ── Header divider ────────────────────────────────────────────────
            painter.line_segment(
                [Pos2::new(track_x0, avail.min.y), Pos2::new(track_x0, avail.max.y)],
                Stroke::new(1.0, Color32::from_rgb(50, 50, 65)),
            );
        });
}

fn draw_ruler(painter: &egui::Painter, avail: Rect, zoom: f32, scroll: f32) {
    let ruler = Rect::from_min_size(
        Pos2::new(avail.min.x + HEADER_W, avail.min.y),
        Vec2::new(avail.width() - HEADER_W, RULER_H),
    );
    painter.rect_filled(ruler, 0.0, Color32::from_rgb(30, 30, 38));

    let step = best_tick_step(zoom);
    let t_start = (scroll / step).floor() as i32;
    let t_end   = ((scroll + avail.width() / zoom) / step).ceil() as i32 + 1;

    for ti in t_start..=t_end {
        let t = ti as f32 * step;
        let x = ruler.min.x + (t - scroll) * zoom;
        if x < ruler.min.x || x > ruler.max.x { continue; }
        let major = (ti % 5) == 0;
        let tick_h = if major { RULER_H * 0.6 } else { RULER_H * 0.3 };
        painter.line_segment(
            [Pos2::new(x, ruler.max.y - tick_h), Pos2::new(x, ruler.max.y)],
            Stroke::new(0.8, Color32::from_rgb(90, 90, 110)),
        );
        if major {
            painter.text(
                Pos2::new(x + 2.0, ruler.min.y + 3.0),
                egui::Align2::LEFT_TOP,
                format!("{:.1}s", t),
                egui::FontId::monospace(10.0),
                Color32::from_rgb(140, 140, 160),
            );
        }
    }
}

fn best_tick_step(zoom: f32) -> f32 {
    let raw = 50.0 / zoom;
    for &s in &[0.1_f32, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0] {
        if s >= raw { return s; }
    }
    10.0
}