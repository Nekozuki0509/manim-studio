use crate::renderer::RenderQuality;
use crate::scene::{AnimEntry, AnimType, ManimObject, ObjType, Scene};
use crate::{codegen, renderer};
use egui::Context;

// ─────────────────────────────────────────────────────────────────────────────
// 3D Camera
// ─────────────────────────────────────────────────────────────────────────────

/// Spherical orbit camera matching Manim's ThreeDScene convention.
#[derive(Clone, Debug)]
pub struct Camera3D {
    pub phi: f32,       // elevation from z-axis, degrees (Manim default 70)
    pub theta: f32,     // azimuth in xy-plane, degrees (Manim default -45)
    pub zoom: f32,      // pixels per Manim unit
    pub pan: [f32; 2],  // camera-space pan offset in Manim units
}

impl Default for Camera3D {
    fn default() -> Self {
        Self { phi: 70.0, theta: -45.0, zoom: 60.0, pan: [0.0, 0.0] }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Timeline drag state
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum TlDragMode { Move, ResizeLeft, ResizeRight }

#[derive(Clone, Debug)]
pub struct TlDrag {
    pub anim_id: String,
    pub mode: TlDragMode,
    pub orig_start: f32,
    pub orig_dur: f32,
    pub drag_origin_x: f32,  // screen x where drag started
}

// ─────────────────────────────────────────────────────────────────────────────
// App state
// ─────────────────────────────────────────────────────────────────────────────

pub struct ManimStudio {
    // Scene data
    pub scene: Scene,

    // Selection
    pub selected_obj: Option<String>,
    pub selected_anim: Option<String>,

    // Viewport (2D)
    pub vp_zoom: f32,        // pixels per Manim unit
    pub vp_pan: egui::Vec2,  // canvas offset in pixels

    // Viewport (3D camera — used when scene.is_3d)
    pub cam3d: Camera3D,

    // Timeline
    pub tl_zoom: f32,    // pixels per second
    pub tl_scroll: f32,  // horizontal scroll in seconds
    pub tl_drag: Option<TlDrag>,

    // Add-object panel state
    pub show_add_panel: bool,
    pub add_panel_filter: String,

    // Code preview
    pub show_code: bool,
    pub generated_code: String,

    // Render
    pub show_render_settings: bool,
    pub render_quality: RenderQualityOpt,
    pub render_preview: bool,
    pub output_dir: String,
    pub render_status: RenderStatus,

    // Playback
    pub is_playing: bool,
    pub last_t: f64,

    // UI
    pub show_about: bool,
    pub show_error_log: bool,

    // Undo / Redo
    pub history: Vec<crate::scene::Scene>,
    pub history_cursor: usize,

    // Copy / Paste clipboard
    pub clipboard: Option<crate::scene::ManimObject>,
}

#[derive(Clone, Debug)]
pub enum RenderStatus {
    Idle,
    Done(String),
    Error(String),
}

#[derive(Clone, PartialEq, Debug)]
pub enum RenderQualityOpt {
    Low,
    Medium,
    High,
    Ultra,
}

impl RenderQualityOpt {
    pub fn label(&self) -> &str {
        match self {
            Self::Low    => "480p  (Low)",
            Self::Medium => "720p  (Medium)",
            Self::High   => "1080p (High)",
            Self::Ultra  => "4K    (Ultra)",
        }
    }
    pub fn to_renderer(&self) -> RenderQuality {
        match self {
            Self::Low    => RenderQuality::Low,
            Self::Medium => RenderQuality::Medium,
            Self::High   => RenderQuality::High,
            Self::Ultra  => RenderQuality::Ultra,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Init
// ─────────────────────────────────────────────────────────────────────────────

impl ManimStudio {
    pub fn new() -> Self {
        Self {
            scene: Scene::demo(),
            selected_obj: None,
            selected_anim: None,
            vp_zoom: 60.0,
            vp_pan: egui::Vec2::ZERO,
            cam3d: Camera3D::default(),
            tl_zoom: 80.0,
            tl_scroll: 0.0,
            tl_drag: None,
            show_add_panel: false,
            add_panel_filter: String::new(),
            show_code: false,
            generated_code: String::new(),
            show_render_settings: false,
            render_quality: RenderQualityOpt::Medium,
            render_preview: false,
            output_dir: "./media".into(),
            render_status: RenderStatus::Idle,
            is_playing: false,
            last_t: 0.0,
            show_about: false,
            show_error_log: false,
            history: vec![],
            history_cursor: 0,
            clipboard: None,
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────────────

    pub fn sel_obj(&self) -> Option<&ManimObject> {
        self.selected_obj.as_ref().and_then(|id| self.scene.get_object(id))
    }

    /// Snapshot the current scene for undo. Call before any destructive edit.
    pub fn push_history(&mut self) {
        // Truncate any redo history above the cursor
        self.history.truncate(self.history_cursor);
        self.history.push(self.scene.clone());
        // Cap at 50 steps
        if self.history.len() > 50 {
            self.history.remove(0);
        }
        self.history_cursor = self.history.len();
    }

    pub fn undo(&mut self) {
        if self.history_cursor == 0 { return; }
        // Push current state as redo target if we haven't done so
        if self.history_cursor == self.history.len() {
            self.history.push(self.scene.clone());
        }
        self.history_cursor -= 1;
        self.scene = self.history[self.history_cursor].clone();
        self.selected_obj = None;
        self.selected_anim = None;
    }

    pub fn redo(&mut self) {
        if self.history_cursor + 1 >= self.history.len() { return; }
        self.history_cursor += 1;
        self.scene = self.history[self.history_cursor].clone();
        self.selected_obj = None;
        self.selected_anim = None;
    }

    pub fn copy_selected(&mut self) {
        if let Some(obj) = self.sel_obj().cloned() {
            self.clipboard = Some(obj);
        }
    }

    pub fn cut_selected(&mut self) {
        self.copy_selected();
        self.push_history();
        self.delete_selected();
    }

    pub fn paste(&mut self) {
        if let Some(mut obj) = self.clipboard.clone() {
            self.push_history();
            obj.id = crate::scene::new_id();
            obj.name = format!("{}_paste", obj.name);
            obj.position[0] += 0.3;
            obj.position[1] -= 0.3;
            let id = obj.id.clone();
            self.scene.add_object(obj);
            self.selected_obj = Some(id);
        }
    }

    pub fn add_object(&mut self, obj_type: ObjType) {
        self.push_history();
        let name = format!("{}", obj_type.name());
        let count = self.scene.objects.iter().filter(|o| o.type_name() == obj_type.name()).count();
        let name = format!("{}_{}", name, count + 1);
        let obj = ManimObject::new(&name, obj_type);
        let id = obj.id.clone();
        self.scene.add_object(obj);
        self.selected_obj = Some(id.clone());
        // Auto-add Create animation at current time
        self.scene.animations.push(AnimEntry::new(
            &id,
            AnimType::Create,
            self.scene.timeline.current_time,
        ));
    }

    pub fn delete_selected(&mut self) {
        if self.selected_obj.is_some() || self.selected_anim.is_some() {
            self.push_history();
        }
        if let Some(id) = self.selected_obj.take() {
            self.scene.remove_object(&id);
        }
        if let Some(id) = self.selected_anim.take() {
            self.scene.animations.retain(|a| a.id != id);
        }
    }

    pub fn generate_code(&mut self) {
        self.generated_code = codegen::generate(&self.scene);
    }

    pub fn render_scene(&mut self) {
        self.generate_code();
        let code = self.generated_code.clone();
        let name = self.scene.name.clone();
        let quality = self.render_quality.to_renderer();
        let out = self.output_dir.clone();
        let preview = self.render_preview;

        match renderer::render(&code, &name, &quality, &out, preview) {
            Ok(path) => {
                self.render_status = RenderStatus::Done(path.display().to_string())
            }
            Err(e) => {
                self.render_status = RenderStatus::Error(e);
                self.show_error_log = true;
            }
        }
    }

    pub fn duplicate_selected(&mut self) {
        if let Some(id) = &self.selected_obj.clone() {
            if let Some(obj) = self.scene.get_object(id).cloned() {
                self.push_history();
                let mut new_obj = obj.clone();
                new_obj.id = crate::scene::new_id();
                new_obj.name = format!("{}_copy", new_obj.name);
                new_obj.position[0] += 0.5;
                new_obj.position[1] += 0.5;
                let new_id = new_obj.id.clone();
                self.scene.add_object(new_obj);
                self.selected_obj = Some(new_id);
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// eframe::App
// ─────────────────────────────────────────────────────────────────────────────

impl eframe::App for ManimStudio {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // ── Playback tick ────────────────────────────────────────────────────
        if self.is_playing {
            let now = ctx.input(|i| i.time);
            if self.last_t == 0.0 {
                self.last_t = now;
            }
            let dt = (now - self.last_t) as f32;
            self.last_t = now;
            self.scene.timeline.current_time += dt;
            if self.scene.timeline.current_time >= self.scene.timeline.duration {
                self.scene.timeline.current_time = 0.0;
                self.is_playing = false;
            }
            ctx.request_repaint();
        } else {
            self.last_t = 0.0;
        }

        // ── Keyboard shortcuts ───────────────────────────────────────────────
        let ctrl = ctx.input(|i| i.modifiers.ctrl || i.modifiers.command);

        if ctx.input(|i| i.key_pressed(egui::Key::Z)) && ctrl {
            if ctx.input(|i| i.modifiers.shift) {
                self.redo();
            } else {
                self.undo();
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Y)) && ctrl {
            self.redo();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::C)) && ctrl {
            self.copy_selected();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::X)) && ctrl {
            self.cut_selected();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::V)) && ctrl {
            self.paste();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::D)) && ctrl {
            self.duplicate_selected();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
            self.delete_selected();
        }
        // Space = play/pause
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.is_playing = !self.is_playing;
        }
        // Home = rewind
        if ctx.input(|i| i.key_pressed(egui::Key::Home)) {
            self.scene.timeline.current_time = 0.0;
            self.is_playing = false;
        }

        // ── Layout ───────────────────────────────────────────────────────────
        crate::ui::toolbar::show(self, ctx);
        crate::ui::timeline::show(self, ctx);
        crate::ui::objects_panel::show(self, ctx);
        crate::ui::properties::show(self, ctx);
        crate::ui::viewport::show(self, ctx);

        // ── Floating windows ─────────────────────────────────────────────────
        crate::ui::windows::show_code(self, ctx);
        crate::ui::windows::show_render_settings(self, ctx);
        crate::ui::windows::show_about(self, ctx);
        crate::ui::windows::show_error_log(self, ctx);
    }
}