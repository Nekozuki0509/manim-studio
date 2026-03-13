use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

pub fn new_id() -> String {
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:x}{:x}", t.as_secs(), t.subsec_nanos())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scene
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub name: String,
    pub objects: Vec<ManimObject>,
    pub animations: Vec<AnimEntry>,
    pub timeline: Timeline,
    pub bg_color: [f32; 3],
    pub is_3d: bool,
    pub camera_phi: f32,
    pub camera_theta: f32,
    /// If false, MathTex is emitted as Text() in codegen (no LaTeX required)
    #[serde(default)]
    pub require_latex: bool,
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene {
    pub fn new() -> Self {
        Self {
            name: "MyScene".into(),
            objects: vec![],
            animations: vec![],
            timeline: Timeline::default(),
            bg_color: [0.05, 0.05, 0.05],
            is_3d: false,
            camera_phi: 70.0,
            camera_theta: -45.0,
            require_latex: false,
        }
    }

    pub fn add_object(&mut self, obj: ManimObject) {
        self.objects.push(obj);
    }

    pub fn remove_object(&mut self, id: &str) {
        self.objects.retain(|o| o.id != id);
        self.animations.retain(|a| a.object_id != id);
    }

    pub fn get_object(&self, id: &str) -> Option<&ManimObject> {
        self.objects.iter().find(|o| o.id == id)
    }

    pub fn get_object_mut(&mut self, id: &str) -> Option<&mut ManimObject> {
        self.objects.iter_mut().find(|o| o.id == id)
    }

    pub fn object_name(&self, id: &str) -> &str {
        self.get_object(id).map(|o| o.name.as_str()).unwrap_or("Unknown")
    }

    /// Insert a demo scene for first launch
    pub fn demo() -> Self {
        let mut scene = Self::new();

        let mut c = ManimObject::new("Circle", ObjType::Circle { radius: 1.0 });
        c.position = [-2.5, 0.5, 0.0];
        c.color = [0.22, 0.60, 0.95];
        let cid = c.id.clone();

        let mut s = ManimObject::new("Square", ObjType::Square { side_length: 2.0 });
        s.position = [2.5, 0.5, 0.0];
        s.color = [0.95, 0.30, 0.30];
        let sid = s.id.clone();

        // Use Text instead of MathTex so LaTeX is not required for the demo
        let mut t = ManimObject::new("Title", ObjType::Text { content: "f(x) = x^2".into(), font_size: 48.0 });
        t.position = [0.0, -2.0, 0.0];
        t.color = [1.0, 1.0, 1.0];
        let tid = t.id.clone();

        scene.add_object(c);
        scene.add_object(s);
        scene.add_object(t);

        scene.animations.push(AnimEntry::new(&cid, AnimType::Create, 0.0));
        scene.animations.push(AnimEntry::new(&sid, AnimType::Create, 1.0));
        scene.animations.push(AnimEntry::new(&tid, AnimType::Write, 2.0));
        scene.animations.push(AnimEntry::new(&cid, AnimType::Shift { dx: 1.0, dy: 1.0 }, 4.5));
        scene.animations.push(AnimEntry::new(&sid, AnimType::Rotate { angle: 45.0 }, 4.5));
        scene.animations.push(AnimEntry::new(&cid, AnimType::FadeOut, 6.5));
        scene.animations.push(AnimEntry::new(&sid, AnimType::FadeOut, 6.5));
        scene.animations.push(AnimEntry::new(&tid, AnimType::FadeOut, 7.0));

        scene.timeline.duration = 9.0;
        scene
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Animation preview: compute display state at a given time
// ─────────────────────────────────────────────────────────────────────────────

/// The visual state of an object at a specific playback time.
#[derive(Debug, Clone)]
pub struct DisplayState {
    pub position: [f32; 3],
    pub scale: f32,
    pub opacity: f32,
    pub fill_opacity: f32,
    pub visible: bool,
    pub rotation: f32,
}

impl DisplayState {
    pub fn from_obj(obj: &ManimObject) -> Self {
        Self {
            position: obj.position,
            scale: obj.scale,
            opacity: obj.opacity,
            fill_opacity: obj.fill_opacity,
            visible: obj.visible,
            rotation: obj.rotation,
        }
    }
}

/// Smooth easing (matches Manim's default `smooth` rate function).
fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t * (10.0 + t * (-15.0 + 6.0 * t))
}

pub fn compute_display_state(obj: &ManimObject, animations: &[AnimEntry], t: f32) -> DisplayState {
    let mut state = DisplayState::from_obj(obj);

    // Collect animations for this object, sorted by start time.
    let mut obj_anims: Vec<&AnimEntry> = animations.iter()
        .filter(|a| a.object_id == obj.id)
        .collect();
    obj_anims.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());

    // Determine whether any "creation" animation exists.
    let has_creation_anim = obj_anims.iter().any(|a| {
        matches!(a.anim_type,
            AnimType::Create | AnimType::FadeIn | AnimType::Write |
            AnimType::GrowFromCenter | AnimType::DrawBorderThenFill |
            AnimType::SpinInFromNothing | AnimType::GrowFromPoint { .. } |
            AnimType::GrowFromEdge { .. } | AnimType::GrowArrow |
            AnimType::SpiralIn)
    });

    // If there's no creation anim, object is always visible.
    if !has_creation_anim {
        return state;
    }

    // Start invisible; creation animations will reveal it.
    state.visible = false;
    state.opacity = 0.0;
    state.fill_opacity = 0.0;

    // Accumulated shift from completed Shift animations
    let mut pos_offset = [0.0_f32; 3];
    // Accumulated scale multiplier from completed Scale animations
    let mut scale_mul = 1.0_f32;

    for anim in &obj_anims {
        if t < anim.start_time {
            // Not yet started — skip
            continue;
        }

        let raw_progress = ((t - anim.start_time) / anim.duration).clamp(0.0, 1.0);
        let p = smooth(raw_progress);
        let done = raw_progress >= 1.0;

        match &anim.anim_type {
            // ── Appearance ─────────────────────────────────────────────────
            AnimType::Create
            | AnimType::Write
            | AnimType::DrawBorderThenFill
            | AnimType::SpinInFromNothing => {
                state.visible = true;
                state.opacity = p * obj.opacity;
                state.fill_opacity = p * obj.fill_opacity;
            }
            AnimType::FadeIn => {
                state.visible = true;
                state.opacity = p * obj.opacity;
                state.fill_opacity = p * obj.fill_opacity;
            }
            AnimType::GrowFromCenter => {
                state.visible = true;
                state.scale = obj.scale * p;
                state.opacity = obj.opacity;
                state.fill_opacity = obj.fill_opacity;
            }

            // ── Disappearance ───────────────────────────────────────────────
            AnimType::FadeOut | AnimType::Uncreate | AnimType::Unwrite => {
                state.opacity = (1.0 - p) * obj.opacity;
                state.fill_opacity = (1.0 - p) * obj.fill_opacity;
                if done {
                    state.visible = false;
                    state.opacity = 0.0;
                }
            }

            // ── Transform ──────────────────────────────────────────────────
            AnimType::Shift { dx, dy } => {
                if done {
                    pos_offset[0] += dx;
                    pos_offset[1] += dy;
                } else {
                    pos_offset[0] += dx * p;
                    pos_offset[1] += dy * p;
                }
            }
            AnimType::MoveTo { x, y } => {
                let tx = x - obj.position[0];
                let ty = y - obj.position[1];
                if done {
                    pos_offset[0] += tx;
                    pos_offset[1] += ty;
                } else {
                    pos_offset[0] += tx * p;
                    pos_offset[1] += ty * p;
                }
            }
            AnimType::Scale { factor } => {
                let f = 1.0 + (factor - 1.0) * p;
                if done {
                    scale_mul *= factor;
                } else {
                    scale_mul *= f;
                }
            }
            AnimType::Rotate { angle } => {
                let delta = if done { *angle } else { angle * p };
                state.rotation += delta;
            }

            // ── Highlights (no persistent visual change) ──────────────────
            AnimType::Flash | AnimType::Indicate | AnimType::Wiggle
            | AnimType::ApplyWave | AnimType::Circumscribe
            | AnimType::ShowPassingFlash => {
                // pulse scale during the animation
                if !done {
                    let pulse = 1.0 + 0.15 * (p * std::f32::consts::PI).sin();
                    state.scale *= pulse;
                }
            }

            AnimType::ShrinkToCenter => {
                state.scale = obj.scale * (1.0 - p);
                state.opacity = (1.0 - p) * obj.opacity;
                state.fill_opacity = (1.0 - p) * obj.fill_opacity;
                if done {
                    state.visible = false;
                    state.opacity = 0.0;
                }
            }
            AnimType::GrowFromPoint { .. } | AnimType::GrowFromEdge { .. } | AnimType::GrowArrow => {
                state.visible = true;
                state.scale = obj.scale * p;
                state.opacity = obj.opacity;
                state.fill_opacity = obj.fill_opacity;
            }
            AnimType::SpiralIn => {
                state.visible = true;
                state.scale = obj.scale * p;
                state.opacity = p * obj.opacity;
                state.fill_opacity = p * obj.fill_opacity;
            }

            AnimType::Transform { .. } | AnimType::FadeTransform { .. } | AnimType::ReplacementTransform { .. }
            | AnimType::CounterclockwiseTransform { .. } | AnimType::ClockwiseTransform { .. } => {
                // These are handled as no-ops for preview (actual transform is complex)
            }

            _ => {}
        }
    }

    state.position[0] += pos_offset[0];
    state.position[1] += pos_offset[1];
    state.scale *= scale_mul;

    state
}

// ─────────────────────────────────────────────────────────────────────────────
// ManimObject
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManimObject {
    pub id: String,
    pub name: String,
    pub object_type: ObjType,
    pub position: [f32; 3],
    pub scale: f32,
    pub rotation: f32,    // degrees
    pub color: [f32; 3],
    pub opacity: f32,
    pub fill_opacity: f32,
    pub stroke_width: f32,
    pub visible: bool,
}

impl ManimObject {
    pub fn new(name: &str, obj_type: ObjType) -> Self {
        let color = default_color(&obj_type);
        Self {
            id: new_id(),
            name: name.into(),
            object_type: obj_type,
            position: [0.0; 3],
            scale: 1.0,
            rotation: 0.0,
            color,
            opacity: 1.0,
            fill_opacity: 1.0,
            stroke_width: 2.0,
            visible: true,
        }
    }

    /// Safe Python variable name
    pub fn var_name(&self) -> String {
        self.name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect::<String>()
    }

    pub fn type_name(&self) -> &str {
        self.object_type.name()
    }

    pub fn icon(&self) -> &str {
        self.object_type.icon()
    }
}

fn default_color(t: &ObjType) -> [f32; 3] {
    match t {
        ObjType::Circle { .. } => [0.22, 0.60, 0.95],
        ObjType::Square { .. } => [0.95, 0.30, 0.30],
        ObjType::Rectangle { .. } => [0.30, 0.85, 0.40],
        ObjType::Triangle { .. } => [0.95, 0.80, 0.15],
        ObjType::Sphere { .. } => [0.55, 0.55, 0.95],
        ObjType::Cube { .. } => [0.85, 0.45, 0.85],
        ObjType::Cylinder { .. } => [0.45, 0.85, 0.85],
        ObjType::NumberPlane => [0.30, 0.40, 0.90],
        ObjType::Ellipse { .. } => [0.40, 0.75, 0.95],
        ObjType::Annulus { .. } => [0.80, 0.50, 0.30],
        ObjType::RegularPolygon { .. } => [0.70, 0.90, 0.30],
        ObjType::Star { .. } => [0.95, 0.85, 0.20],
        ObjType::RoundedRectangle { .. } => [0.30, 0.85, 0.40],
        ObjType::Cone { .. } => [0.90, 0.65, 0.30],
        ObjType::Torus { .. } => [0.65, 0.40, 0.90],
        ObjType::Prism { .. } => [0.80, 0.50, 0.70],
        ObjType::BarChart { .. } => [0.30, 0.70, 0.90],
        _ => [1.0, 1.0, 1.0],
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Object types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ObjType {
    Circle { radius: f32 },
    Square { side_length: f32 },
    Rectangle { width: f32, height: f32 },
    Triangle { side_length: f32 },
    Text { content: String, font_size: f32 },
    MathTex { content: String },
    Line { start: [f32; 3], end: [f32; 3] },
    Arrow { start: [f32; 3], end: [f32; 3] },
    Dot,
    Sphere { radius: f32 },
    Cube { side_length: f32 },
    Cylinder { radius: f32, height: f32 },
    Axes,
    NumberPlane,
    Polygon { points: Vec<[f32; 2]> },
    // 2D geometry
    Arc { radius: f32, start_angle: f32, angle: f32 },
    ArcBetweenPoints { start: [f32; 3], end: [f32; 3], angle: f32 },
    Ellipse { width: f32, height: f32 },
    Annulus { inner_radius: f32, outer_radius: f32 },
    Sector { radius: f32, start_angle: f32, angle: f32 },
    RegularPolygon { n: u32, radius: f32 },
    Star { n: u32, outer_radius: f32, inner_radius: f32 },
    RoundedRectangle { width: f32, height: f32, corner_radius: f32 },
    // Lines & arrows
    DashedLine { start: [f32; 3], end: [f32; 3], dash_length: f32 },
    DoubleArrow { start: [f32; 3], end: [f32; 3] },
    Vector { direction: [f32; 3] },
    // Annotations
    Brace { direction: [f32; 3], length: f32 },
    BraceBetweenPoints { start: [f32; 3], end: [f32; 3] },
    Angle { radius: f32, start_angle: f32, angle: f32 },
    RightAngle { size: f32 },
    // Graphing
    NumberLine { x_min: f32, x_max: f32, step: f32 },
    BarChart { values: Vec<f32>, bar_width: f32 },
    // Numbers
    DecimalNumber { number: f32, num_decimal_places: u32 },
    Integer { number: i32 },
    // 3D objects
    Dot3D,
    Cone { radius: f32, height: f32 },
    Torus { major_radius: f32, minor_radius: f32 },
    Prism { width: f32, height: f32, depth: f32 },
    Arrow3D { start: [f32; 3], end: [f32; 3] },
    Line3D { start: [f32; 3], end: [f32; 3] },
    Surface,
}

impl ObjType {
    pub fn name(&self) -> &str {
        match self {
            Self::Circle { .. } => "Circle",
            Self::Square { .. } => "Square",
            Self::Rectangle { .. } => "Rectangle",
            Self::Triangle { .. } => "Triangle",
            Self::Text { .. } => "Text",
            Self::MathTex { .. } => "MathTex",
            Self::Line { .. } => "Line",
            Self::Arrow { .. } => "Arrow",
            Self::Dot => "Dot",
            Self::Sphere { .. } => "Sphere",
            Self::Cube { .. } => "Cube",
            Self::Cylinder { .. } => "Cylinder",
            Self::Axes => "Axes",
            Self::NumberPlane => "NumberPlane",
            Self::Polygon { .. } => "Polygon",
            Self::Arc { .. } => "Arc",
            Self::ArcBetweenPoints { .. } => "ArcBetweenPoints",
            Self::Ellipse { .. } => "Ellipse",
            Self::Annulus { .. } => "Annulus",
            Self::Sector { .. } => "Sector",
            Self::RegularPolygon { .. } => "RegularPolygon",
            Self::Star { .. } => "Star",
            Self::RoundedRectangle { .. } => "RoundedRectangle",
            Self::DashedLine { .. } => "DashedLine",
            Self::DoubleArrow { .. } => "DoubleArrow",
            Self::Vector { .. } => "Vector",
            Self::Brace { .. } => "Brace",
            Self::BraceBetweenPoints { .. } => "BraceBetweenPoints",
            Self::Angle { .. } => "Angle",
            Self::RightAngle { .. } => "RightAngle",
            Self::NumberLine { .. } => "NumberLine",
            Self::BarChart { .. } => "BarChart",
            Self::DecimalNumber { .. } => "DecimalNumber",
            Self::Integer { .. } => "Integer",
            Self::Dot3D => "Dot3D",
            Self::Cone { .. } => "Cone",
            Self::Torus { .. } => "Torus",
            Self::Prism { .. } => "Prism",
            Self::Arrow3D { .. } => "Arrow3D",
            Self::Line3D { .. } => "Line3D",
            Self::Surface => "Surface",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            Self::Circle { .. } => "⬤",
            Self::Square { .. } => "■",
            Self::Rectangle { .. } => "▬",
            Self::Triangle { .. } => "▲",
            Self::Text { .. } => "T",
            Self::MathTex { .. } => "∑",
            Self::Line { .. } => "╱",
            Self::Arrow { .. } => "→",
            Self::Dot => "•",
            Self::Sphere { .. } => "◉",
            Self::Cube { .. } => "▣",
            Self::Cylinder { .. } => "⊡",
            Self::Axes => "⊞",
            Self::NumberPlane => "⊟",
            Self::Polygon { .. } => "⬡",
            Self::Arc { .. } => "⌒",
            Self::ArcBetweenPoints { .. } => "⌒",
            Self::Ellipse { .. } => "⬮",
            Self::Annulus { .. } => "◎",
            Self::Sector { .. } => "◔",
            Self::RegularPolygon { .. } => "⬡",
            Self::Star { .. } => "★",
            Self::RoundedRectangle { .. } => "▢",
            Self::DashedLine { .. } => "┄",
            Self::DoubleArrow { .. } => "↔",
            Self::Vector { .. } => "⇀",
            Self::Brace { .. } => "⏞",
            Self::BraceBetweenPoints { .. } => "⏞",
            Self::Angle { .. } => "∠",
            Self::RightAngle { .. } => "∟",
            Self::NumberLine { .. } => "├",
            Self::BarChart { .. } => "📊",
            Self::DecimalNumber { .. } => "🔢",
            Self::Integer { .. } => "🔢",
            Self::Dot3D => "•",
            Self::Cone { .. } => "▲",
            Self::Torus { .. } => "◍",
            Self::Prism { .. } => "▱",
            Self::Arrow3D { .. } => "➜",
            Self::Line3D { .. } => "╱",
            Self::Surface => "🌊",
        }
    }
}

/// Templates shown in the Add Object panel
pub fn object_templates() -> Vec<(&'static str, &'static str, ObjType)> {
    vec![
        ("⬤", "Circle",      ObjType::Circle { radius: 1.0 }),
        ("■", "Square",      ObjType::Square { side_length: 2.0 }),
        ("▬", "Rectangle",   ObjType::Rectangle { width: 3.0, height: 2.0 }),
        ("▲", "Triangle",    ObjType::Triangle { side_length: 2.0 }),
        ("T", "Text",        ObjType::Text { content: "Hello".into(), font_size: 48.0 }),
        ("∑", "MathTex",     ObjType::MathTex { content: r"E = mc^2".into() }),
        ("→", "Arrow",       ObjType::Arrow { start: [-2.0, 0.0, 0.0], end: [2.0, 0.0, 0.0] }),
        ("╱", "Line",        ObjType::Line { start: [-2.0, 0.0, 0.0], end: [2.0, 0.0, 0.0] }),
        ("•", "Dot",         ObjType::Dot),
        ("◉", "Sphere",      ObjType::Sphere { radius: 1.0 }),
        ("▣", "Cube",        ObjType::Cube { side_length: 2.0 }),
        ("⊡", "Cylinder",    ObjType::Cylinder { radius: 1.0, height: 2.0 }),
        ("⊞", "Axes",        ObjType::Axes),
        ("⊟", "NumberPlane", ObjType::NumberPlane),
        // More 2D Shapes
        ("⬮", "Ellipse",    ObjType::Ellipse { width: 3.0, height: 2.0 }),
        ("◎", "Annulus",     ObjType::Annulus { inner_radius: 0.5, outer_radius: 1.0 }),
        ("◔", "Sector",     ObjType::Sector { radius: 1.0, start_angle: 0.0, angle: 90.0 }),
        ("⌒", "Arc",        ObjType::Arc { radius: 1.0, start_angle: 0.0, angle: 180.0 }),
        ("⬡", "RegularPolygon", ObjType::RegularPolygon { n: 6, radius: 1.0 }),
        ("★", "Star",       ObjType::Star { n: 5, outer_radius: 1.0, inner_radius: 0.5 }),
        ("▢", "RoundedRectangle", ObjType::RoundedRectangle { width: 3.0, height: 2.0, corner_radius: 0.3 }),
        // Lines & Arrows
        ("┄", "DashedLine", ObjType::DashedLine { start: [-2.0, 0.0, 0.0], end: [2.0, 0.0, 0.0], dash_length: 0.2 }),
        ("↔", "DoubleArrow", ObjType::DoubleArrow { start: [-2.0, 0.0, 0.0], end: [2.0, 0.0, 0.0] }),
        ("⇀", "Vector",     ObjType::Vector { direction: [2.0, 1.0, 0.0] }),
        // Annotations
        ("⏞", "Brace",      ObjType::Brace { direction: [0.0, 1.0, 0.0], length: 2.0 }),
        ("∠", "Angle",      ObjType::Angle { radius: 0.5, start_angle: 0.0, angle: 45.0 }),
        ("∟", "RightAngle", ObjType::RightAngle { size: 0.5 }),
        // Graphing
        ("├", "NumberLine",  ObjType::NumberLine { x_min: -5.0, x_max: 5.0, step: 1.0 }),
        ("📊", "BarChart",   ObjType::BarChart { values: vec![3.0, 5.0, 2.0, 4.0, 1.0], bar_width: 0.6 }),
        // Numbers
        ("🔢", "DecimalNumber", ObjType::DecimalNumber { number: 3.14, num_decimal_places: 2 }),
        ("🔢", "Integer",    ObjType::Integer { number: 42 }),
        // 3D
        ("•", "Dot3D",      ObjType::Dot3D),
        ("▲", "Cone",       ObjType::Cone { radius: 1.0, height: 2.0 }),
        ("◍", "Torus",      ObjType::Torus { major_radius: 1.0, minor_radius: 0.3 }),
        ("▱", "Prism",      ObjType::Prism { width: 2.0, height: 2.0, depth: 2.0 }),
        ("➜", "Arrow3D",    ObjType::Arrow3D { start: [0.0, 0.0, 0.0], end: [2.0, 1.0, 1.0] }),
        ("╱", "Line3D",     ObjType::Line3D { start: [-1.0, -1.0, -1.0], end: [1.0, 1.0, 1.0] }),
        ("🌊", "Surface",    ObjType::Surface),
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// Animations
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimEntry {
    pub id: String,
    pub object_id: String,
    pub anim_type: AnimType,
    pub start_time: f32,
    pub duration: f32,
    pub rate_func: RateFunc,
}

impl AnimEntry {
    pub fn new(object_id: &str, anim_type: AnimType, start_time: f32) -> Self {
        let duration = anim_type.default_duration();
        Self {
            id: new_id(),
            object_id: object_id.into(),
            anim_type,
            start_time,
            duration,
            rate_func: RateFunc::Smooth,
        }
    }

    pub fn end_time(&self) -> f32 {
        self.start_time + self.duration
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnimType {
    Create,
    Uncreate,
    Write,
    FadeIn,
    FadeOut,
    GrowFromCenter,
    DrawBorderThenFill,
    SpinInFromNothing,
    Rotate { angle: f32 },
    Scale { factor: f32 },
    Shift { dx: f32, dy: f32 },
    MoveTo { x: f32, y: f32 },
    Transform { target_id: String },
    Flash,
    Indicate,
    Wiggle,
    Wait,
    Unwrite,
    FadeTransform { target_id: String },
    ReplacementTransform { target_id: String },
    ShrinkToCenter,
    GrowFromPoint { point: [f32; 3] },
    GrowFromEdge { edge: [f32; 3] },
    GrowArrow,
    CounterclockwiseTransform { target_id: String },
    ClockwiseTransform { target_id: String },
    ApplyWave,
    Circumscribe,
    ShowPassingFlash,
    SpiralIn,
    MoveAlongPath { path_points: Vec<[f32; 2]> },
    Homotopy,
    PhaseFlow,
    Succession { animations: Vec<String> },
    AnimationGroup { animations: Vec<String> },
}

impl AnimType {
    pub fn name(&self) -> &str {
        match self {
            Self::Create => "Create",
            Self::Uncreate => "Uncreate",
            Self::Write => "Write",
            Self::FadeIn => "FadeIn",
            Self::FadeOut => "FadeOut",
            Self::GrowFromCenter => "GrowFromCenter",
            Self::DrawBorderThenFill => "DrawBorderThenFill",
            Self::SpinInFromNothing => "SpinInFromNothing",
            Self::Rotate { .. } => "Rotate",
            Self::Scale { .. } => "Scale",
            Self::Shift { .. } => "Shift",
            Self::MoveTo { .. } => "MoveTo",
            Self::Transform { .. } => "Transform",
            Self::Flash => "Flash",
            Self::Indicate => "Indicate",
            Self::Wiggle => "Wiggle",
            Self::Wait => "Wait",
            Self::Unwrite => "Unwrite",
            Self::FadeTransform { .. } => "FadeTransform",
            Self::ReplacementTransform { .. } => "ReplacementTransform",
            Self::ShrinkToCenter => "ShrinkToCenter",
            Self::GrowFromPoint { .. } => "GrowFromPoint",
            Self::GrowFromEdge { .. } => "GrowFromEdge",
            Self::GrowArrow => "GrowArrow",
            Self::CounterclockwiseTransform { .. } => "CounterclockwiseTransform",
            Self::ClockwiseTransform { .. } => "ClockwiseTransform",
            Self::ApplyWave => "ApplyWave",
            Self::Circumscribe => "Circumscribe",
            Self::ShowPassingFlash => "ShowPassingFlash",
            Self::SpiralIn => "SpiralIn",
            Self::MoveAlongPath { .. } => "MoveAlongPath",
            Self::Homotopy => "Homotopy",
            Self::PhaseFlow => "PhaseFlow",
            Self::Succession { .. } => "Succession",
            Self::AnimationGroup { .. } => "AnimationGroup",
        }
    }

    pub fn default_duration(&self) -> f32 {
        match self {
            Self::Write => 2.0,
            Self::FadeIn | Self::FadeOut => 0.5,
            Self::Transform { .. } => 1.5,
            Self::Wait => 1.0,
            Self::Unwrite => 2.0,
            Self::FadeTransform { .. } => 1.0,
            Self::ReplacementTransform { .. } => 1.0,
            Self::ShrinkToCenter => 1.0,
            Self::GrowFromPoint { .. } => 1.0,
            Self::GrowFromEdge { .. } => 1.0,
            Self::GrowArrow => 1.0,
            Self::CounterclockwiseTransform { .. } => 1.5,
            Self::ClockwiseTransform { .. } => 1.5,
            Self::ApplyWave => 1.0,
            Self::Circumscribe => 1.0,
            Self::ShowPassingFlash => 1.0,
            Self::SpiralIn => 1.0,
            Self::MoveAlongPath { .. } => 2.0,
            Self::Homotopy => 1.5,
            Self::PhaseFlow => 1.5,
            Self::Succession { .. } => 2.0,
            Self::AnimationGroup { .. } => 1.5,
            _ => 1.0,
        }
    }

    /// Colour used in timeline tracks
    pub fn track_color(&self) -> [u8; 3] {
        match self {
            Self::Create | Self::GrowFromCenter | Self::DrawBorderThenFill | Self::SpinInFromNothing
                => [60, 180, 100],
            Self::Write => [80, 150, 230],
            Self::FadeOut | Self::Uncreate => [200, 80, 80],
            Self::FadeIn => [80, 180, 220],
            Self::Rotate { .. } | Self::Scale { .. } | Self::Shift { .. } | Self::MoveTo { .. }
                => [180, 100, 230],
            Self::Transform { .. } => [230, 160, 50],
            Self::Flash | Self::Indicate | Self::Wiggle => [230, 220, 50],
            Self::Wait => [80, 80, 80],
            Self::Unwrite => [80, 150, 230],
            Self::FadeTransform { .. } | Self::ReplacementTransform { .. } => [230, 160, 50],
            Self::ShrinkToCenter => [200, 80, 80],
            Self::GrowFromPoint { .. } | Self::GrowFromEdge { .. } | Self::GrowArrow => [60, 180, 100],
            Self::CounterclockwiseTransform { .. } | Self::ClockwiseTransform { .. } => [230, 160, 50],
            Self::ApplyWave | Self::Circumscribe | Self::ShowPassingFlash => [230, 220, 50],
            Self::SpiralIn => [60, 180, 100],
            Self::MoveAlongPath { .. } | Self::Homotopy | Self::PhaseFlow => [180, 100, 230],
            Self::Succession { .. } | Self::AnimationGroup { .. } => [140, 140, 180],
        }
    }

    pub fn all_variants() -> Vec<Self> {
        vec![
            Self::Create, Self::Write, Self::FadeIn,
            Self::GrowFromCenter, Self::DrawBorderThenFill, Self::SpinInFromNothing,
            Self::Rotate { angle: 90.0 },
            Self::Scale { factor: 2.0 },
            Self::Shift { dx: 2.0, dy: 0.0 },
            Self::MoveTo { x: 0.0, y: 0.0 },
            Self::FadeOut, Self::Uncreate,
            Self::Flash, Self::Indicate, Self::Wiggle,
            Self::Wait,
            Self::Unwrite,
            Self::ShrinkToCenter,
            Self::GrowFromPoint { point: [0.0, 0.0, 0.0] },
            Self::GrowFromEdge { edge: [0.0, -1.0, 0.0] },
            Self::GrowArrow,
            Self::ApplyWave,
            Self::Circumscribe,
            Self::ShowPassingFlash,
            Self::SpiralIn,
            Self::MoveAlongPath { path_points: vec![] },
            Self::Homotopy,
            Self::PhaseFlow,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RateFunc {
    Linear,
    Smooth,
    RushInto,
    RushFrom,
    ThereAndBack,
    Wiggle,
    DoubleSmooth,
    ExponentialDecay,
    EaseInSine,
    EaseOutSine,
    EaseInOutSine,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInExpo,
    EaseOutExpo,
    EaseInOutExpo,
    EaseInBounce,
    EaseOutBounce,
    EaseInOutBounce,
}

impl RateFunc {
    pub fn name(&self) -> &str {
        match self {
            Self::Linear => "linear",
            Self::Smooth => "smooth",
            Self::RushInto => "rush_into",
            Self::RushFrom => "rush_from",
            Self::ThereAndBack => "there_and_back",
            Self::Wiggle => "wiggle",
            Self::DoubleSmooth => "double_smooth",
            Self::ExponentialDecay => "exponential_decay",
            Self::EaseInSine => "rate_functions.ease_in_sine",
            Self::EaseOutSine => "rate_functions.ease_out_sine",
            Self::EaseInOutSine => "rate_functions.ease_in_out_sine",
            Self::EaseInQuad => "rate_functions.ease_in_quad",
            Self::EaseOutQuad => "rate_functions.ease_out_quad",
            Self::EaseInOutQuad => "rate_functions.ease_in_out_quad",
            Self::EaseInCubic => "rate_functions.ease_in_cubic",
            Self::EaseOutCubic => "rate_functions.ease_out_cubic",
            Self::EaseInOutCubic => "rate_functions.ease_in_out_cubic",
            Self::EaseInExpo => "rate_functions.ease_in_expo",
            Self::EaseOutExpo => "rate_functions.ease_out_expo",
            Self::EaseInOutExpo => "rate_functions.ease_in_out_expo",
            Self::EaseInBounce => "rate_functions.ease_in_bounce",
            Self::EaseOutBounce => "rate_functions.ease_out_bounce",
            Self::EaseInOutBounce => "rate_functions.ease_in_out_bounce",
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Linear => "Linear",
            Self::Smooth => "Smooth",
            Self::RushInto => "Rush Into",
            Self::RushFrom => "Rush From",
            Self::ThereAndBack => "There & Back",
            Self::Wiggle => "Wiggle",
            Self::DoubleSmooth => "Double Smooth",
            Self::ExponentialDecay => "Exponential Decay",
            Self::EaseInSine => "Ease In Sine",
            Self::EaseOutSine => "Ease Out Sine",
            Self::EaseInOutSine => "Ease In Out Sine",
            Self::EaseInQuad => "Ease In Quad",
            Self::EaseOutQuad => "Ease Out Quad",
            Self::EaseInOutQuad => "Ease In Out Quad",
            Self::EaseInCubic => "Ease In Cubic",
            Self::EaseOutCubic => "Ease Out Cubic",
            Self::EaseInOutCubic => "Ease In Out Cubic",
            Self::EaseInExpo => "Ease In Expo",
            Self::EaseOutExpo => "Ease Out Expo",
            Self::EaseInOutExpo => "Ease In Out Expo",
            Self::EaseInBounce => "Ease In Bounce",
            Self::EaseOutBounce => "Ease Out Bounce",
            Self::EaseInOutBounce => "Ease In Out Bounce",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Self::Linear, Self::Smooth, Self::RushInto, Self::RushFrom, Self::ThereAndBack, Self::Wiggle,
             Self::DoubleSmooth, Self::ExponentialDecay,
             Self::EaseInSine, Self::EaseOutSine, Self::EaseInOutSine,
             Self::EaseInQuad, Self::EaseOutQuad, Self::EaseInOutQuad,
             Self::EaseInCubic, Self::EaseOutCubic, Self::EaseInOutCubic,
             Self::EaseInExpo, Self::EaseOutExpo, Self::EaseInOutExpo,
             Self::EaseInBounce, Self::EaseOutBounce, Self::EaseInOutBounce]
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Timeline
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeline {
    pub duration: f32,
    pub current_time: f32,
    pub fps: f32,
}

impl Default for Timeline {
    fn default() -> Self {
        Self { duration: 10.0, current_time: 0.0, fps: 60.0 }
    }
}