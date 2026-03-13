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

/// Scene type determines the base class in generated code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SceneType {
    /// Standard Scene
    Scene,
    /// ThreeDScene — 3-D camera support
    ThreeDScene,
    /// MovingCameraScene — allows camera movement
    MovingCameraScene,
    /// ZoomedScene — supports inset zoomed views
    ZoomedScene,
}

impl Default for SceneType {
    fn default() -> Self { Self::Scene }
}

impl SceneType {
    pub fn label(&self) -> &str {
        match self {
            Self::Scene => "Scene",
            Self::ThreeDScene => "ThreeDScene",
            Self::MovingCameraScene => "MovingCameraScene",
            Self::ZoomedScene => "ZoomedScene",
        }
    }
    pub fn all() -> &'static [SceneType] {
        &[SceneType::Scene, SceneType::ThreeDScene, SceneType::MovingCameraScene, SceneType::ZoomedScene]
    }
}

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
    /// Scene type determines the Python base class
    #[serde(default)]
    pub scene_type: SceneType,
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
            scene_type: SceneType::default(),
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
            AnimType::SpiralIn | AnimType::FadeInFromPoint { .. } |
            AnimType::FadeInFrom { .. } | AnimType::VFadeIn |
            AnimType::ShowCreationThenFadeOut | AnimType::AddTextLetterByLetter)
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
            | AnimType::SpinInFromNothing
            | AnimType::AddTextLetterByLetter => {
                state.visible = true;
                state.opacity = p * obj.opacity;
                state.fill_opacity = p * obj.fill_opacity;
            }
            AnimType::FadeIn | AnimType::FadeInFromPoint { .. }
            | AnimType::FadeInFrom { .. } | AnimType::VFadeIn => {
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
            AnimType::ShowCreationThenFadeOut => {
                state.visible = true;
                if raw_progress < 0.5 {
                    let sub_p = smooth(raw_progress * 2.0);
                    state.opacity = sub_p * obj.opacity;
                    state.fill_opacity = sub_p * obj.fill_opacity;
                } else {
                    let sub_p = smooth((raw_progress - 0.5) * 2.0);
                    state.opacity = (1.0 - sub_p) * obj.opacity;
                    state.fill_opacity = (1.0 - sub_p) * obj.fill_opacity;
                }
                if done { state.visible = false; state.opacity = 0.0; }
            }

            // ── Disappearance ───────────────────────────────────────────────
            AnimType::FadeOut | AnimType::Uncreate | AnimType::Unwrite
            | AnimType::FadeOutToPoint { .. } | AnimType::FadeOutAndShift { .. }
            | AnimType::VFadeOut | AnimType::RemoveTextLetterByLetter => {
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
            AnimType::Scale { factor } | AnimType::ScaleInPlace { factor } => {
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
            | AnimType::ShowPassingFlash | AnimType::FocusOn
            | AnimType::Broadcast | AnimType::AnimatedBoundary => {
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
            | AnimType::TransformFromCopy { .. }
            | AnimType::CounterclockwiseTransform { .. } | AnimType::ClockwiseTransform { .. }
            | AnimType::CyclicReplace { .. } | AnimType::Swap { .. } => {
                // These are handled as no-ops for preview (actual transform is complex)
            }

            // Camera animations don't affect object display state
            AnimType::MoveCamera { .. } | AnimType::CameraRotate { .. }
            | AnimType::CameraZoom { .. } | AnimType::CameraPan { .. } => {}

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
        ObjType::FunctionGraph { .. } => [0.20, 0.80, 0.60],
        ObjType::ParametricFunction { .. } => [0.60, 0.40, 0.90],
        ObjType::ComplexPlane => [0.30, 0.40, 0.90],
        ObjType::PolarPlane => [0.30, 0.40, 0.90],
        ObjType::CoordinateSystem => [0.30, 0.40, 0.90],
        ObjType::Title { .. } => [1.0, 1.0, 1.0],
        ObjType::Table { .. } => [0.80, 0.80, 0.80],
        ObjType::Matrix { .. } => [0.80, 0.80, 0.80],
        ObjType::Code { .. } => [0.90, 0.90, 0.90],
        ObjType::BulletedList { .. } => [1.0, 1.0, 1.0],
        ObjType::CurvedArrow { .. } | ObjType::CurvedDoubleArrow { .. } => [1.0, 1.0, 1.0],
        ObjType::Icosahedron { .. } => [0.55, 0.70, 0.95],
        ObjType::Dodecahedron { .. } => [0.70, 0.55, 0.95],
        ObjType::LabeledDot { .. } => [0.22, 0.60, 0.95],
        ObjType::SurroundingRectangle { .. } => [0.95, 0.80, 0.15],
        ObjType::Cross { .. } => [0.95, 0.30, 0.30],
        _ => [1.0, 1.0, 1.0],
    }
}

/// Returns the maximum lane index used by animations for a given object.
pub fn max_lane_for_object(animations: &[AnimEntry], obj_id: &str) -> u32 {
    animations
        .iter()
        .filter(|a| a.object_id == obj_id)
        .map(|a| a.lane)
        .max()
        .unwrap_or(0)
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
    CurvedArrow { start: [f32; 3], end: [f32; 3] },
    CurvedDoubleArrow { start: [f32; 3], end: [f32; 3] },
    Elbow,
    TangentLine { length: f32, angle: f32 },
    // Annotations
    Brace { direction: [f32; 3], length: f32 },
    BraceBetweenPoints { start: [f32; 3], end: [f32; 3] },
    BraceLabel { direction: [f32; 3], length: f32, label: String },
    Angle { radius: f32, start_angle: f32, angle: f32 },
    RightAngle { size: f32 },
    LabeledDot { label: String, radius: f32 },
    LabeledLine { label: String, start: [f32; 3], end: [f32; 3] },
    SurroundingRectangle { buff: f32 },
    BackgroundRectangle,
    Underline,
    Cross { scale: f32 },
    // Graphing
    NumberLine { x_min: f32, x_max: f32, step: f32 },
    BarChart { values: Vec<f32>, bar_width: f32 },
    FunctionGraph { expression: String, x_min: f32, x_max: f32 },
    ParametricFunction { expression: String, t_min: f32, t_max: f32 },
    ImplicitFunction { expression: String },
    ComplexPlane,
    PolarPlane,
    CoordinateSystem,
    // Text & Tables
    Title { content: String },
    MarkupText { content: String },
    BulletedList { items: Vec<String> },
    Paragraph { content: String },
    Code { code: String, language: String },
    Table { rows: u32, cols: u32 },
    Matrix { rows: u32, cols: u32 },
    // Numbers
    DecimalNumber { number: f32, num_decimal_places: u32 },
    Integer { number: i32 },
    // Grouping
    VGroup { children: Vec<String> },
    // 3D objects
    Dot3D,
    Cone { radius: f32, height: f32 },
    Torus { major_radius: f32, minor_radius: f32 },
    Prism { width: f32, height: f32, depth: f32 },
    Arrow3D { start: [f32; 3], end: [f32; 3] },
    Line3D { start: [f32; 3], end: [f32; 3] },
    Surface,
    Icosahedron { radius: f32 },
    Dodecahedron { radius: f32 },
    // Special
    TracedPath,
    PointCloudDot,
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
            Self::CurvedArrow { .. } => "CurvedArrow",
            Self::CurvedDoubleArrow { .. } => "CurvedDoubleArrow",
            Self::Elbow => "Elbow",
            Self::TangentLine { .. } => "TangentLine",
            Self::Brace { .. } => "Brace",
            Self::BraceBetweenPoints { .. } => "BraceBetweenPoints",
            Self::BraceLabel { .. } => "BraceLabel",
            Self::Angle { .. } => "Angle",
            Self::RightAngle { .. } => "RightAngle",
            Self::LabeledDot { .. } => "LabeledDot",
            Self::LabeledLine { .. } => "LabeledLine",
            Self::SurroundingRectangle { .. } => "SurroundingRectangle",
            Self::BackgroundRectangle => "BackgroundRectangle",
            Self::Underline => "Underline",
            Self::Cross { .. } => "Cross",
            Self::NumberLine { .. } => "NumberLine",
            Self::BarChart { .. } => "BarChart",
            Self::FunctionGraph { .. } => "FunctionGraph",
            Self::ParametricFunction { .. } => "ParametricFunction",
            Self::ImplicitFunction { .. } => "ImplicitFunction",
            Self::ComplexPlane => "ComplexPlane",
            Self::PolarPlane => "PolarPlane",
            Self::CoordinateSystem => "CoordinateSystem",
            Self::Title { .. } => "Title",
            Self::MarkupText { .. } => "MarkupText",
            Self::BulletedList { .. } => "BulletedList",
            Self::Paragraph { .. } => "Paragraph",
            Self::Code { .. } => "Code",
            Self::Table { .. } => "Table",
            Self::Matrix { .. } => "Matrix",
            Self::DecimalNumber { .. } => "DecimalNumber",
            Self::Integer { .. } => "Integer",
            Self::VGroup { .. } => "VGroup",
            Self::Dot3D => "Dot3D",
            Self::Cone { .. } => "Cone",
            Self::Torus { .. } => "Torus",
            Self::Prism { .. } => "Prism",
            Self::Arrow3D { .. } => "Arrow3D",
            Self::Line3D { .. } => "Line3D",
            Self::Surface => "Surface",
            Self::Icosahedron { .. } => "Icosahedron",
            Self::Dodecahedron { .. } => "Dodecahedron",
            Self::TracedPath => "TracedPath",
            Self::PointCloudDot => "PointCloudDot",
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
            Self::CurvedArrow { .. } => "↷",
            Self::CurvedDoubleArrow { .. } => "⇌",
            Self::Elbow => "∟",
            Self::TangentLine { .. } => "⟋",
            Self::Brace { .. } => "⏞",
            Self::BraceBetweenPoints { .. } => "⏞",
            Self::BraceLabel { .. } => "⏞",
            Self::Angle { .. } => "∠",
            Self::RightAngle { .. } => "∟",
            Self::LabeledDot { .. } => "◉",
            Self::LabeledLine { .. } => "╱",
            Self::SurroundingRectangle { .. } => "▭",
            Self::BackgroundRectangle => "▬",
            Self::Underline => "▁",
            Self::Cross { .. } => "✕",
            Self::NumberLine { .. } => "├",
            Self::BarChart { .. } => "📊",
            Self::FunctionGraph { .. } => "ƒ",
            Self::ParametricFunction { .. } => "γ",
            Self::ImplicitFunction { .. } => "≡",
            Self::ComplexPlane => "ℂ",
            Self::PolarPlane => "◎",
            Self::CoordinateSystem => "⊞",
            Self::Title { .. } => "H",
            Self::MarkupText { .. } => "M",
            Self::BulletedList { .. } => "•",
            Self::Paragraph { .. } => "¶",
            Self::Code { .. } => "<>",
            Self::Table { .. } => "⊞",
            Self::Matrix { .. } => "[]",
            Self::DecimalNumber { .. } => "🔢",
            Self::Integer { .. } => "🔢",
            Self::VGroup { .. } => "⊕",
            Self::Dot3D => "•",
            Self::Cone { .. } => "▲",
            Self::Torus { .. } => "◍",
            Self::Prism { .. } => "▱",
            Self::Arrow3D { .. } => "➜",
            Self::Line3D { .. } => "╱",
            Self::Surface => "🌊",
            Self::Icosahedron { .. } => "⬡",
            Self::Dodecahedron { .. } => "⬡",
            Self::TracedPath => "~",
            Self::PointCloudDot => "⁘",
        }
    }
}

/// Templates shown in the Add Object panel
pub fn object_templates() -> Vec<(&'static str, &'static str, ObjType)> {
    vec![
        // ── 2D Shapes ────────────────────────────────────────────────
        ("⬤", "Circle",      ObjType::Circle { radius: 1.0 }),
        ("■", "Square",      ObjType::Square { side_length: 2.0 }),
        ("▬", "Rectangle",   ObjType::Rectangle { width: 3.0, height: 2.0 }),
        ("▲", "Triangle",    ObjType::Triangle { side_length: 2.0 }),
        ("⬮", "Ellipse",    ObjType::Ellipse { width: 3.0, height: 2.0 }),
        ("◎", "Annulus",     ObjType::Annulus { inner_radius: 0.5, outer_radius: 1.0 }),
        ("◔", "Sector",     ObjType::Sector { radius: 1.0, start_angle: 0.0, angle: 90.0 }),
        ("⌒", "Arc",        ObjType::Arc { radius: 1.0, start_angle: 0.0, angle: 180.0 }),
        ("⬡", "RegularPolygon", ObjType::RegularPolygon { n: 6, radius: 1.0 }),
        ("★", "Star",       ObjType::Star { n: 5, outer_radius: 1.0, inner_radius: 0.5 }),
        ("▢", "RoundedRectangle", ObjType::RoundedRectangle { width: 3.0, height: 2.0, corner_radius: 0.3 }),
        ("•", "Dot",         ObjType::Dot),
        ("⬡", "Polygon",    ObjType::Polygon { points: vec![[-1.0, -1.0], [1.0, -1.0], [0.0, 1.0]] }),
        // ── Text ─────────────────────────────────────────────────────
        ("T", "Text",        ObjType::Text { content: "Hello".into(), font_size: 48.0 }),
        ("∑", "MathTex",     ObjType::MathTex { content: r"E = mc^2".into() }),
        ("H", "Title",       ObjType::Title { content: "Title".into() }),
        ("M", "MarkupText",  ObjType::MarkupText { content: "Markup <b>Text</b>".into() }),
        ("•", "BulletedList", ObjType::BulletedList { items: vec!["Item 1".into(), "Item 2".into(), "Item 3".into()] }),
        ("¶", "Paragraph",   ObjType::Paragraph { content: "A paragraph of text.".into() }),
        ("<>", "Code",       ObjType::Code { code: "print(\"hello\")".into(), language: "python".into() }),
        // ── Lines & Arrows ───────────────────────────────────────────
        ("→", "Arrow",       ObjType::Arrow { start: [-2.0, 0.0, 0.0], end: [2.0, 0.0, 0.0] }),
        ("╱", "Line",        ObjType::Line { start: [-2.0, 0.0, 0.0], end: [2.0, 0.0, 0.0] }),
        ("┄", "DashedLine", ObjType::DashedLine { start: [-2.0, 0.0, 0.0], end: [2.0, 0.0, 0.0], dash_length: 0.2 }),
        ("↔", "DoubleArrow", ObjType::DoubleArrow { start: [-2.0, 0.0, 0.0], end: [2.0, 0.0, 0.0] }),
        ("⇀", "Vector",     ObjType::Vector { direction: [2.0, 1.0, 0.0] }),
        ("↷", "CurvedArrow", ObjType::CurvedArrow { start: [-2.0, 0.0, 0.0], end: [2.0, 0.0, 0.0] }),
        ("⇌", "CurvedDoubleArrow", ObjType::CurvedDoubleArrow { start: [-2.0, 0.0, 0.0], end: [2.0, 0.0, 0.0] }),
        ("∟", "Elbow",      ObjType::Elbow),
        ("⟋", "TangentLine", ObjType::TangentLine { length: 2.0, angle: 45.0 }),
        // ── Annotations ──────────────────────────────────────────────
        ("⏞", "Brace",      ObjType::Brace { direction: [0.0, 1.0, 0.0], length: 2.0 }),
        ("⏞", "BraceLabel", ObjType::BraceLabel { direction: [0.0, 1.0, 0.0], length: 2.0, label: "label".into() }),
        ("∠", "Angle",      ObjType::Angle { radius: 0.5, start_angle: 0.0, angle: 45.0 }),
        ("∟", "RightAngle", ObjType::RightAngle { size: 0.5 }),
        ("◉", "LabeledDot", ObjType::LabeledDot { label: "A".into(), radius: 0.2 }),
        ("╱", "LabeledLine", ObjType::LabeledLine { label: "d".into(), start: [-2.0, 0.0, 0.0], end: [2.0, 0.0, 0.0] }),
        ("▭", "SurroundingRectangle", ObjType::SurroundingRectangle { buff: 0.2 }),
        ("▬", "BackgroundRectangle", ObjType::BackgroundRectangle),
        ("▁", "Underline",  ObjType::Underline),
        ("✕", "Cross",      ObjType::Cross { scale: 1.0 }),
        // ── Graphing ─────────────────────────────────────────────────
        ("⊞", "Axes",        ObjType::Axes),
        ("⊟", "NumberPlane", ObjType::NumberPlane),
        ("├", "NumberLine",  ObjType::NumberLine { x_min: -5.0, x_max: 5.0, step: 1.0 }),
        ("📊", "BarChart",   ObjType::BarChart { values: vec![3.0, 5.0, 2.0, 4.0, 1.0], bar_width: 0.6 }),
        ("ƒ", "FunctionGraph", ObjType::FunctionGraph { expression: "lambda x: x**2".into(), x_min: -3.0, x_max: 3.0 }),
        ("γ", "ParametricFunction", ObjType::ParametricFunction { expression: "lambda t: np.array([np.cos(t), np.sin(t), 0])".into(), t_min: 0.0, t_max: 6.28 }),
        ("≡", "ImplicitFunction", ObjType::ImplicitFunction { expression: "lambda x, y: x**2 + y**2 - 1".into() }),
        ("ℂ", "ComplexPlane", ObjType::ComplexPlane),
        ("◎", "PolarPlane",  ObjType::PolarPlane),
        ("⊞", "CoordinateSystem", ObjType::CoordinateSystem),
        // ── Tables & Matrices ────────────────────────────────────────
        ("⊞", "Table",      ObjType::Table { rows: 3, cols: 3 }),
        ("[]", "Matrix",     ObjType::Matrix { rows: 2, cols: 2 }),
        // ── Numbers ──────────────────────────────────────────────────
        ("🔢", "DecimalNumber", ObjType::DecimalNumber { number: 3.14, num_decimal_places: 2 }),
        ("🔢", "Integer",    ObjType::Integer { number: 42 }),
        // ── Grouping ─────────────────────────────────────────────────
        ("⊕", "VGroup",     ObjType::VGroup { children: vec![] }),
        // ── 3D ───────────────────────────────────────────────────────
        ("◉", "Sphere",      ObjType::Sphere { radius: 1.0 }),
        ("▣", "Cube",        ObjType::Cube { side_length: 2.0 }),
        ("⊡", "Cylinder",    ObjType::Cylinder { radius: 1.0, height: 2.0 }),
        ("•", "Dot3D",      ObjType::Dot3D),
        ("▲", "Cone",       ObjType::Cone { radius: 1.0, height: 2.0 }),
        ("◍", "Torus",      ObjType::Torus { major_radius: 1.0, minor_radius: 0.3 }),
        ("▱", "Prism",      ObjType::Prism { width: 2.0, height: 2.0, depth: 2.0 }),
        ("➜", "Arrow3D",    ObjType::Arrow3D { start: [0.0, 0.0, 0.0], end: [2.0, 1.0, 1.0] }),
        ("╱", "Line3D",     ObjType::Line3D { start: [-1.0, -1.0, -1.0], end: [1.0, 1.0, 1.0] }),
        ("🌊", "Surface",    ObjType::Surface),
        ("⬡", "Icosahedron", ObjType::Icosahedron { radius: 1.0 }),
        ("⬡", "Dodecahedron", ObjType::Dodecahedron { radius: 1.0 }),
        // ── Special ──────────────────────────────────────────────────
        ("~", "TracedPath",  ObjType::TracedPath),
        ("⁘", "PointCloudDot", ObjType::PointCloudDot),
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
    /// Lane index within the object's track group (0-based).
    /// Multiple lanes allow simultaneous animations on the same object.
    #[serde(default)]
    pub lane: u32,
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
            lane: 0,
        }
    }

    pub fn new_on_lane(object_id: &str, anim_type: AnimType, start_time: f32, lane: u32) -> Self {
        let duration = anim_type.default_duration();
        Self {
            id: new_id(),
            object_id: object_id.into(),
            anim_type,
            start_time,
            duration,
            rate_func: RateFunc::Smooth,
            lane,
        }
    }

    pub fn end_time(&self) -> f32 {
        self.start_time + self.duration
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnimType {
    // ── Creation / Appearance ─────────────────────────────────────────
    Create,
    Uncreate,
    Write,
    FadeIn,
    FadeOut,
    GrowFromCenter,
    DrawBorderThenFill,
    SpinInFromNothing,
    GrowFromPoint { point: [f32; 3] },
    GrowFromEdge { edge: [f32; 3] },
    GrowArrow,
    SpiralIn,
    FadeInFromPoint { point: [f32; 3] },
    FadeInFrom { direction: [f32; 3] },
    FadeOutToPoint { point: [f32; 3] },
    FadeOutAndShift { direction: [f32; 3] },
    VFadeIn,
    VFadeOut,
    ShowCreationThenFadeOut,
    AddTextLetterByLetter,
    RemoveTextLetterByLetter,

    // ── Disappearance ────────────────────────────────────────────────
    Unwrite,
    ShrinkToCenter,

    // ── Movement ─────────────────────────────────────────────────────
    Shift { dx: f32, dy: f32 },
    MoveTo { x: f32, y: f32 },
    MoveAlongPath { path_points: Vec<[f32; 2]> },

    // ── Transformation ───────────────────────────────────────────────
    Rotate { angle: f32 },
    Scale { factor: f32 },
    ScaleInPlace { factor: f32 },
    Transform { target_id: String },
    FadeTransform { target_id: String },
    ReplacementTransform { target_id: String },
    TransformFromCopy { target_id: String },
    CounterclockwiseTransform { target_id: String },
    ClockwiseTransform { target_id: String },
    CyclicReplace { object_ids: Vec<String> },
    Swap { object_ids: Vec<String> },
    ApplyMethod { method: String },
    Restore,
    ChangeDecimalToValue { value: f32 },

    // ── Highlights / Emphasis ────────────────────────────────────────
    Flash,
    Indicate,
    Wiggle,
    ApplyWave,
    Circumscribe,
    ShowPassingFlash,
    FocusOn,
    Broadcast,
    ShowIncreasingSubsets,
    ShowSubmobjectsOneByOne,
    AnimatedBoundary,

    // ── Camera ───────────────────────────────────────────────────────
    MoveCamera { phi: Option<f32>, theta: Option<f32>, zoom: Option<f32>, frame_center: Option<[f32; 3]> },
    CameraRotate { angle: f32 },
    CameraZoom { factor: f32 },
    CameraPan { dx: f32, dy: f32 },

    // ── Composites ───────────────────────────────────────────────────
    Succession { animations: Vec<String> },
    AnimationGroup { animations: Vec<String> },
    LaggedStart { lag_ratio: f32 },
    LaggedStartMap { lag_ratio: f32 },

    // ── Special ──────────────────────────────────────────────────────
    Wait,
    Homotopy,
    PhaseFlow,
    UpdateFromFunc,
    UpdateFromAlphaFunc,
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
            Self::GrowFromPoint { .. } => "GrowFromPoint",
            Self::GrowFromEdge { .. } => "GrowFromEdge",
            Self::GrowArrow => "GrowArrow",
            Self::SpiralIn => "SpiralIn",
            Self::FadeInFromPoint { .. } => "FadeInFromPoint",
            Self::FadeInFrom { .. } => "FadeInFrom",
            Self::FadeOutToPoint { .. } => "FadeOutToPoint",
            Self::FadeOutAndShift { .. } => "FadeOutAndShift",
            Self::VFadeIn => "VFadeIn",
            Self::VFadeOut => "VFadeOut",
            Self::ShowCreationThenFadeOut => "ShowCreationThenFadeOut",
            Self::AddTextLetterByLetter => "AddTextLetterByLetter",
            Self::RemoveTextLetterByLetter => "RemoveTextLetterByLetter",
            Self::Unwrite => "Unwrite",
            Self::ShrinkToCenter => "ShrinkToCenter",
            Self::Shift { .. } => "Shift",
            Self::MoveTo { .. } => "MoveTo",
            Self::MoveAlongPath { .. } => "MoveAlongPath",
            Self::Rotate { .. } => "Rotate",
            Self::Scale { .. } => "Scale",
            Self::ScaleInPlace { .. } => "ScaleInPlace",
            Self::Transform { .. } => "Transform",
            Self::FadeTransform { .. } => "FadeTransform",
            Self::ReplacementTransform { .. } => "ReplacementTransform",
            Self::TransformFromCopy { .. } => "TransformFromCopy",
            Self::CounterclockwiseTransform { .. } => "CounterclockwiseTransform",
            Self::ClockwiseTransform { .. } => "ClockwiseTransform",
            Self::CyclicReplace { .. } => "CyclicReplace",
            Self::Swap { .. } => "Swap",
            Self::ApplyMethod { .. } => "ApplyMethod",
            Self::Restore => "Restore",
            Self::ChangeDecimalToValue { .. } => "ChangeDecimalToValue",
            Self::Flash => "Flash",
            Self::Indicate => "Indicate",
            Self::Wiggle => "Wiggle",
            Self::ApplyWave => "ApplyWave",
            Self::Circumscribe => "Circumscribe",
            Self::ShowPassingFlash => "ShowPassingFlash",
            Self::FocusOn => "FocusOn",
            Self::Broadcast => "Broadcast",
            Self::ShowIncreasingSubsets => "ShowIncreasingSubsets",
            Self::ShowSubmobjectsOneByOne => "ShowSubmobjectsOneByOne",
            Self::AnimatedBoundary => "AnimatedBoundary",
            Self::MoveCamera { .. } => "MoveCamera",
            Self::CameraRotate { .. } => "CameraRotate",
            Self::CameraZoom { .. } => "CameraZoom",
            Self::CameraPan { .. } => "CameraPan",
            Self::Succession { .. } => "Succession",
            Self::AnimationGroup { .. } => "AnimationGroup",
            Self::LaggedStart { .. } => "LaggedStart",
            Self::LaggedStartMap { .. } => "LaggedStartMap",
            Self::Wait => "Wait",
            Self::Homotopy => "Homotopy",
            Self::PhaseFlow => "PhaseFlow",
            Self::UpdateFromFunc => "UpdateFromFunc",
            Self::UpdateFromAlphaFunc => "UpdateFromAlphaFunc",
        }
    }

    pub fn default_duration(&self) -> f32 {
        match self {
            Self::Write | Self::Unwrite
            | Self::AddTextLetterByLetter | Self::RemoveTextLetterByLetter
            | Self::MoveAlongPath { .. } | Self::Succession { .. } => 2.0,
            Self::FadeIn | Self::FadeOut | Self::VFadeIn | Self::VFadeOut => 0.5,
            Self::FadeInFromPoint { .. } | Self::FadeInFrom { .. }
            | Self::FadeOutToPoint { .. } | Self::FadeOutAndShift { .. } => 0.5,
            Self::Transform { .. } | Self::CounterclockwiseTransform { .. }
            | Self::ClockwiseTransform { .. } | Self::Homotopy | Self::PhaseFlow
            | Self::AnimationGroup { .. } => 1.5,
            Self::Wait => 1.0,
            Self::FadeTransform { .. } | Self::ReplacementTransform { .. }
            | Self::TransformFromCopy { .. } | Self::ShrinkToCenter
            | Self::GrowFromPoint { .. } | Self::GrowFromEdge { .. } | Self::GrowArrow
            | Self::ApplyWave | Self::Circumscribe | Self::ShowPassingFlash
            | Self::SpiralIn | Self::FocusOn | Self::Broadcast
            | Self::ShowIncreasingSubsets | Self::ShowSubmobjectsOneByOne
            | Self::AnimatedBoundary | Self::ShowCreationThenFadeOut
            | Self::LaggedStart { .. } | Self::LaggedStartMap { .. }
            | Self::CyclicReplace { .. } | Self::Swap { .. }
            | Self::Restore | Self::ChangeDecimalToValue { .. }
            | Self::ApplyMethod { .. } | Self::ScaleInPlace { .. }
            | Self::UpdateFromFunc | Self::UpdateFromAlphaFunc => 1.0,
            Self::MoveCamera { .. } | Self::CameraRotate { .. }
            | Self::CameraZoom { .. } | Self::CameraPan { .. } => 2.0,
            _ => 1.0,
        }
    }

    /// Colour used in timeline tracks
    pub fn track_color(&self) -> [u8; 3] {
        match self {
            Self::Create | Self::GrowFromCenter | Self::DrawBorderThenFill | Self::SpinInFromNothing
            | Self::GrowFromPoint { .. } | Self::GrowFromEdge { .. } | Self::GrowArrow
            | Self::SpiralIn | Self::ShowCreationThenFadeOut
                => [60, 180, 100],
            Self::Write | Self::Unwrite
            | Self::AddTextLetterByLetter | Self::RemoveTextLetterByLetter
                => [80, 150, 230],
            Self::FadeOut | Self::Uncreate | Self::ShrinkToCenter
            | Self::FadeOutToPoint { .. } | Self::FadeOutAndShift { .. } | Self::VFadeOut
                => [200, 80, 80],
            Self::FadeIn | Self::FadeInFromPoint { .. } | Self::FadeInFrom { .. } | Self::VFadeIn
                => [80, 180, 220],
            Self::Rotate { .. } | Self::Scale { .. } | Self::ScaleInPlace { .. }
            | Self::Shift { .. } | Self::MoveTo { .. }
            | Self::MoveAlongPath { .. } | Self::Homotopy | Self::PhaseFlow
                => [180, 100, 230],
            Self::Transform { .. } | Self::FadeTransform { .. } | Self::ReplacementTransform { .. }
            | Self::TransformFromCopy { .. }
            | Self::CounterclockwiseTransform { .. } | Self::ClockwiseTransform { .. }
            | Self::CyclicReplace { .. } | Self::Swap { .. }
            | Self::ApplyMethod { .. } | Self::Restore | Self::ChangeDecimalToValue { .. }
                => [230, 160, 50],
            Self::Flash | Self::Indicate | Self::Wiggle
            | Self::ApplyWave | Self::Circumscribe | Self::ShowPassingFlash
            | Self::FocusOn | Self::Broadcast
            | Self::ShowIncreasingSubsets | Self::ShowSubmobjectsOneByOne
            | Self::AnimatedBoundary
                => [230, 220, 50],
            Self::MoveCamera { .. } | Self::CameraRotate { .. }
            | Self::CameraZoom { .. } | Self::CameraPan { .. }
                => [100, 200, 200],
            Self::Succession { .. } | Self::AnimationGroup { .. }
            | Self::LaggedStart { .. } | Self::LaggedStartMap { .. }
                => [140, 140, 180],
            Self::Wait => [80, 80, 80],
            Self::UpdateFromFunc | Self::UpdateFromAlphaFunc => [180, 140, 100],
        }
    }

    pub fn all_variants() -> Vec<Self> {
        vec![
            // Creation
            Self::Create, Self::Write, Self::FadeIn,
            Self::GrowFromCenter, Self::DrawBorderThenFill, Self::SpinInFromNothing,
            Self::GrowFromPoint { point: [0.0, 0.0, 0.0] },
            Self::GrowFromEdge { edge: [0.0, -1.0, 0.0] },
            Self::GrowArrow, Self::SpiralIn,
            Self::FadeInFromPoint { point: [0.0, 0.0, 0.0] },
            Self::FadeInFrom { direction: [0.0, -1.0, 0.0] },
            Self::VFadeIn,
            Self::ShowCreationThenFadeOut,
            Self::AddTextLetterByLetter,
            // Disappearance
            Self::FadeOut, Self::Uncreate, Self::Unwrite, Self::ShrinkToCenter,
            Self::FadeOutToPoint { point: [0.0, 0.0, 0.0] },
            Self::FadeOutAndShift { direction: [0.0, -1.0, 0.0] },
            Self::VFadeOut,
            Self::RemoveTextLetterByLetter,
            // Movement
            Self::Shift { dx: 2.0, dy: 0.0 },
            Self::MoveTo { x: 0.0, y: 0.0 },
            Self::MoveAlongPath { path_points: vec![] },
            // Transformation
            Self::Rotate { angle: 90.0 },
            Self::Scale { factor: 2.0 },
            Self::ScaleInPlace { factor: 2.0 },
            Self::ApplyMethod { method: "set_color".into() },
            Self::Restore,
            Self::ChangeDecimalToValue { value: 0.0 },
            // Highlights
            Self::Flash, Self::Indicate, Self::Wiggle,
            Self::ApplyWave, Self::Circumscribe, Self::ShowPassingFlash,
            Self::FocusOn, Self::Broadcast,
            Self::ShowIncreasingSubsets, Self::ShowSubmobjectsOneByOne,
            Self::AnimatedBoundary,
            // Camera
            Self::MoveCamera { phi: None, theta: None, zoom: None, frame_center: None },
            Self::CameraRotate { angle: 45.0 },
            Self::CameraZoom { factor: 1.5 },
            Self::CameraPan { dx: 1.0, dy: 0.0 },
            // Composites
            Self::LaggedStart { lag_ratio: 0.5 },
            Self::LaggedStartMap { lag_ratio: 0.5 },
            // Special
            Self::Wait,
            Self::Homotopy, Self::PhaseFlow,
            Self::UpdateFromFunc, Self::UpdateFromAlphaFunc,
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