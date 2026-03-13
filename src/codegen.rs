use crate::scene::{AnimEntry, AnimType, ManimObject, ObjType, Scene, SceneType};

/// Generate a complete Manim Python script from the current scene.
pub fn generate(scene: &Scene) -> String {
    let mut lines: Vec<String> = vec![];

    let base_class = match &scene.scene_type {
        SceneType::ThreeDScene => "ThreeDScene",
        SceneType::MovingCameraScene => "MovingCameraScene",
        SceneType::ZoomedScene => "ZoomedScene",
        SceneType::Scene => {
            if scene.is_3d { "ThreeDScene" } else { "Scene" }
        }
    };

    // ── Header ──────────────────────────────────────────────────────────────
    lines.push("from manim import *".into());
    lines.push("".into());
    lines.push(format!("class {}({}):", scene.name, base_class));
    lines.push("    def construct(self):".into());

    // ── Background ──────────────────────────────────────────────────────────
    let bg = scene.bg_color;
    if bg != [0.05, 0.05, 0.05] {
        lines.push(format!(
            "        self.camera.background_color = rgb_to_color(({:.2}, {:.2}, {:.2}))",
            bg[0], bg[1], bg[2]
        ));
    }

    // ── 3D Camera ───────────────────────────────────────────────────────────
    if scene.is_3d {
        lines.push(format!(
            "        self.set_camera_orientation(phi={}*DEGREES, theta={}*DEGREES)",
            scene.camera_phi, scene.camera_theta
        ));
    }

    if !scene.objects.is_empty() || !scene.animations.is_empty() {
        lines.push("".into());
        lines.push("        # ── Object definitions ─────────────────────────────────".into());
    }

    // ── Object construction code ─────────────────────────────────────────────
    for obj in &scene.objects {
        lines.push(build_object_code(obj, scene.require_latex, scene));
    }

    // ── Collect all time-sorted animations ──────────────────────────────────
    if !scene.animations.is_empty() {
        lines.push("".into());
        lines.push("        # ── Animations ────────────────────────────────────────────".into());

        let mut sorted = scene.animations.clone();
        sorted.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());

        let mut cursor: f32 = 0.0;
        let mut i = 0;
        while i < sorted.len() {
            let t = sorted[i].start_time;

            // Insert wait if there's a gap
            if t > cursor + 0.01 {
                lines.push(format!(
                    "        self.wait({:.2})",
                    t - cursor
                ));
            }

            // Collect all anims starting at the same time
            let mut group: Vec<&AnimEntry> = vec![];
            while i < sorted.len() && (sorted[i].start_time - t).abs() < 0.01 {
                group.push(&sorted[i]);
                i += 1;
            }

            let max_dur = group
                .iter()
                .map(|a| a.duration)
                .fold(0.0_f32, f32::max);

            let play_args: Vec<String> = group
                .iter()
                .map(|a| build_anim_arg(a, scene))
                .collect();

            let rate_func = group
                .first()
                .map(|a| a.rate_func.name())
                .unwrap_or("smooth");

            if play_args.len() == 1 {
                lines.push(format!(
                    "        self.play({}, run_time={:.2}, rate_func={})",
                    play_args[0], max_dur, rate_func
                ));
            } else {
                let args = play_args.join(", ");
                lines.push(format!(
                    "        self.play({}, run_time={:.2}, rate_func={})",
                    args, max_dur, rate_func
                ));
            }

            cursor = t + max_dur;
        }

        // Final wait
        if scene.timeline.duration > cursor + 0.1 {
            lines.push(format!("        self.wait({:.2})", scene.timeline.duration - cursor));
        }
    }

    lines.push("".into());
    lines.join("\n")
}

// ─────────────────────────────────────────────────────────────────────────────

fn build_object_code(obj: &ManimObject, require_latex: bool, scene: &Scene) -> String {
    let var = obj.var_name();
    let color = format!(
        "rgb_to_color(({:.3}, {:.3}, {:.3}))",
        obj.color[0], obj.color[1], obj.color[2]
    );

    let constructor = match &obj.object_type {
        ObjType::Circle { radius } =>
            format!("Circle(radius={}, color={}, fill_opacity={:.2})", radius, color, obj.fill_opacity),
        ObjType::Square { side_length } =>
            format!("Square(side_length={}, color={}, fill_opacity={:.2})", side_length, color, obj.fill_opacity),
        ObjType::Rectangle { width, height } =>
            format!("Rectangle(width={}, height={}, color={}, fill_opacity={:.2})", width, height, color, obj.fill_opacity),
        ObjType::Triangle { side_length } =>
            format!("Triangle(color={}, fill_opacity={:.2}).scale({:.3})", color, obj.fill_opacity, side_length / 2.0),
        ObjType::Text { content, font_size } =>
            format!("Text(\"{}\", font_size={:.0}, color={})", escape_str(content), font_size, color),
        ObjType::MathTex { content } => {
            if require_latex {
                format!("MathTex(r\"{}\", color={})", content, color)
            } else {
                format!(
                    "Text(r\"{}\", color={})  # MathTex fallback (LaTeX not required)",
                    escape_str(content), color
                )
            }
        }
        ObjType::Line { start, end } =>
            format!(
                "Line(start=np.array([{:.2}, {:.2}, {:.2}]), end=np.array([{:.2}, {:.2}, {:.2}]), color={})",
                start[0], start[1], start[2], end[0], end[1], end[2], color
            ),
        ObjType::Arrow { start, end } =>
            format!(
                "Arrow(start=np.array([{:.2}, {:.2}, {:.2}]), end=np.array([{:.2}, {:.2}, {:.2}]), color={})",
                start[0], start[1], start[2], end[0], end[1], end[2], color
            ),
        ObjType::Dot =>
            format!("Dot(color={})", color),
        ObjType::Sphere { radius } =>
            format!("Sphere(radius={}, color={}, fill_opacity={:.2})", radius, color, obj.fill_opacity),
        ObjType::Cube { side_length } =>
            format!("Cube(side_length={}, fill_color={}, fill_opacity={:.2})", side_length, color, obj.fill_opacity),
        ObjType::Cylinder { radius, height } =>
            format!("Cylinder(radius={}, height={}, fill_color={}, fill_opacity={:.2})", radius, height, color, obj.fill_opacity),
        ObjType::Axes =>
            format!("Axes(x_range=[-5, 5, 1], y_range=[-3, 3, 1], color={})  # type: ignore", color),
        ObjType::NumberPlane =>
            "NumberPlane()".into(),
        ObjType::Polygon { points } => {
            let pts: Vec<String> = points
                .iter()
                .map(|p| format!("[{:.2}, {:.2}, 0]", p[0], p[1]))
                .collect();
            format!(
                "Polygon({}, color={}, fill_opacity={:.2})",
                pts.join(", "), color, obj.fill_opacity
            )
        }
        ObjType::Arc { radius, start_angle, angle } =>
            format!("Arc(radius={}, start_angle={}*DEGREES, angle={}*DEGREES, color={})", radius, start_angle, angle, color),
        ObjType::ArcBetweenPoints { start, end, angle } =>
            format!("ArcBetweenPoints(start=np.array([{:.2}, {:.2}, {:.2}]), end=np.array([{:.2}, {:.2}, {:.2}]), angle={}*DEGREES, color={})",
                start[0], start[1], start[2], end[0], end[1], end[2], angle, color),
        ObjType::Ellipse { width, height } =>
            format!("Ellipse(width={}, height={}, color={}, fill_opacity={:.2})", width, height, color, obj.fill_opacity),
        ObjType::Annulus { inner_radius, outer_radius } =>
            format!("Annulus(inner_radius={}, outer_radius={}, color={}, fill_opacity={:.2})", inner_radius, outer_radius, color, obj.fill_opacity),
        ObjType::Sector { radius, start_angle, angle } =>
            format!("Sector(outer_radius={}, start_angle={}*DEGREES, angle={}*DEGREES, color={}, fill_opacity={:.2})", radius, start_angle, angle, color, obj.fill_opacity),
        ObjType::RegularPolygon { n, radius } =>
            format!("RegularPolygon(n={}, color={}, fill_opacity={:.2}).scale({:.3})", n, color, obj.fill_opacity, radius),
        ObjType::Star { n, outer_radius, inner_radius } =>
            format!("Star(n={}, outer_radius={}, inner_radius={}, color={}, fill_opacity={:.2})", n, outer_radius, inner_radius, color, obj.fill_opacity),
        ObjType::RoundedRectangle { width, height, corner_radius } =>
            format!("RoundedRectangle(width={}, height={}, corner_radius={}, color={}, fill_opacity={:.2})", width, height, corner_radius, color, obj.fill_opacity),
        ObjType::DashedLine { start, end, dash_length } =>
            format!("DashedLine(start=np.array([{:.2}, {:.2}, {:.2}]), end=np.array([{:.2}, {:.2}, {:.2}]), dash_length={}, color={})",
                start[0], start[1], start[2], end[0], end[1], end[2], dash_length, color),
        ObjType::DoubleArrow { start, end } =>
            format!("DoubleArrow(start=np.array([{:.2}, {:.2}, {:.2}]), end=np.array([{:.2}, {:.2}, {:.2}]), color={})",
                start[0], start[1], start[2], end[0], end[1], end[2], color),
        ObjType::Vector { direction } =>
            format!("Vector(direction=np.array([{:.2}, {:.2}, {:.2}]), color={})", direction[0], direction[1], direction[2], color),
        ObjType::CurvedArrow { start, end } =>
            format!("CurvedArrow(np.array([{:.2}, {:.2}, {:.2}]), np.array([{:.2}, {:.2}, {:.2}]), color={})",
                start[0], start[1], start[2], end[0], end[1], end[2], color),
        ObjType::CurvedDoubleArrow { start, end } =>
            format!("CurvedDoubleArrow(np.array([{:.2}, {:.2}, {:.2}]), np.array([{:.2}, {:.2}, {:.2}]), color={})",
                start[0], start[1], start[2], end[0], end[1], end[2], color),
        ObjType::Elbow =>
            format!("Elbow(color={})", color),
        ObjType::TangentLine { length, angle } =>
            format!("Line(ORIGIN, RIGHT*{}, color={}).rotate({}*DEGREES)", length, color, angle),
        ObjType::Brace { direction, length } =>
            format!("BraceLabel(Dot(), text=\"\", brace_direction=np.array([{:.2}, {:.2}, {:.2}]), color={}).scale({:.3})", direction[0], direction[1], direction[2], color, length),
        ObjType::BraceBetweenPoints { start, end } =>
            format!("BraceBetweenPoints(np.array([{:.2}, {:.2}, {:.2}]), np.array([{:.2}, {:.2}, {:.2}]), color={})",
                start[0], start[1], start[2], end[0], end[1], end[2], color),
        ObjType::BraceLabel { direction, length, label } =>
            format!("BraceLabel(Dot(), text=\"{}\", brace_direction=np.array([{:.2}, {:.2}, {:.2}]), color={}).scale({:.3})", escape_str(label), direction[0], direction[1], direction[2], color, length),
        ObjType::Angle { radius, start_angle, angle } =>
            format!("Angle(radius={}, start_angle={}*DEGREES, angle={}*DEGREES, color={})", radius, start_angle, angle, color),
        ObjType::RightAngle { size } =>
            format!("RightAngle(length={}, color={})", size, color),
        ObjType::LabeledDot { label, radius } =>
            format!("LabeledDot(label=\"{}\", radius={}, color={})", escape_str(label), radius, color),
        ObjType::LabeledLine { label, start, end } =>
            format!("LabeledLine(label=\"{}\", start=np.array([{:.2}, {:.2}, {:.2}]), end=np.array([{:.2}, {:.2}, {:.2}]), color={})",
                escape_str(label), start[0], start[1], start[2], end[0], end[1], end[2], color),
        ObjType::SurroundingRectangle { buff } =>
            format!("SurroundingRectangle(Dot(), buff={}, color={})", buff, color),
        ObjType::BackgroundRectangle =>
            format!("BackgroundRectangle(Dot(), color={})", color),
        ObjType::Underline =>
            format!("Underline(Dot(), color={})", color),
        ObjType::Cross { scale } =>
            format!("Cross(Dot(), scale_factor={}, color={})", scale, color),
        ObjType::NumberLine { x_min, x_max, step } =>
            format!("NumberLine(x_range=[{}, {}, {}], color={})", x_min, x_max, step, color),
        ObjType::BarChart { values, bar_width } => {
            let vals: Vec<String> = values.iter().map(|v| format!("{:.2}", v)).collect();
            format!("BarChart(values=[{}], bar_width={}, bar_colors=[{}])", vals.join(", "), bar_width, color)
        }
        ObjType::FunctionGraph { expression, x_min, x_max } =>
            format!("FunctionGraph({}, x_range=[{}, {}], color={})", expression, x_min, x_max, color),
        ObjType::ParametricFunction { expression, t_min, t_max } =>
            format!("ParametricFunction({}, t_range=[{}, {}], color={})", expression, t_min, t_max, color),
        ObjType::ImplicitFunction { expression } =>
            format!("ImplicitFunction({}, color={})", expression, color),
        ObjType::ComplexPlane =>
            "ComplexPlane()".into(),
        ObjType::PolarPlane =>
            "PolarPlane()".into(),
        ObjType::CoordinateSystem =>
            format!("Axes(x_range=[-5, 5, 1], y_range=[-5, 5, 1], color={})", color),
        ObjType::Title { content } =>
            format!("Title(\"{}\", color={})", escape_str(content), color),
        ObjType::MarkupText { content } =>
            format!("MarkupText(\"{}\", color={})", escape_str(content), color),
        ObjType::BulletedList { items } => {
            let it: Vec<String> = items.iter().map(|i| format!("\"{}\"", escape_str(i))).collect();
            format!("BulletedList({}, color={})", it.join(", "), color)
        }
        ObjType::Paragraph { content } =>
            format!("Paragraph(\"{}\", color={})", escape_str(content), color),
        ObjType::Code { code, language } =>
            format!("Code(code=\"{}\", language=\"{}\", background=\"window\")", escape_str(code), escape_str(language)),
        ObjType::Table { rows, cols } =>
            format!("Table([[\"\" for _ in range({})] for _ in range({})], col_labels=[Text(str(i)) for i in range({})])", cols, rows, cols),
        ObjType::Matrix { rows, cols } =>
            format!("Matrix([[0 for _ in range({})] for _ in range({})])", cols, rows),
        ObjType::DecimalNumber { number, num_decimal_places } =>
            format!("DecimalNumber({}, num_decimal_places={}, color={})", number, num_decimal_places, color),
        ObjType::Integer { number } =>
            format!("Integer({}, color={})", number, color),
        ObjType::VGroup { children } => {
            if children.is_empty() {
                "VGroup()".into()
            } else {
                let vars: Vec<String> = children.iter().filter_map(|cid| {
                    scene.get_object(cid).map(|o| o.var_name())
                }).collect();
                format!("VGroup({})", vars.join(", "))
            }
        }
        ObjType::Dot3D =>
            format!("Dot3D(color={})", color),
        ObjType::Cone { radius, height } =>
            format!("Cone(base_radius={}, height={}, fill_color={}, fill_opacity={:.2})", radius, height, color, obj.fill_opacity),
        ObjType::Torus { major_radius, minor_radius } =>
            format!("Torus(major_radius={}, minor_radius={}, fill_color={}, fill_opacity={:.2})", major_radius, minor_radius, color, obj.fill_opacity),
        ObjType::Prism { width, height, depth } =>
            format!("Prism(dimensions=[{}, {}, {}], fill_color={}, fill_opacity={:.2})", width, height, depth, color, obj.fill_opacity),
        ObjType::Arrow3D { start, end } =>
            format!("Arrow3D(start=np.array([{:.2}, {:.2}, {:.2}]), end=np.array([{:.2}, {:.2}, {:.2}]), color={})",
                start[0], start[1], start[2], end[0], end[1], end[2], color),
        ObjType::Line3D { start, end } =>
            format!("Line3D(start=np.array([{:.2}, {:.2}, {:.2}]), end=np.array([{:.2}, {:.2}, {:.2}]), color={})",
                start[0], start[1], start[2], end[0], end[1], end[2], color),
        ObjType::Surface =>
            format!("Surface(lambda u, v: np.array([u, v, 0]), u_range=[-2, 2], v_range=[-2, 2], fill_color={}, fill_opacity={:.2})", color, obj.fill_opacity),
        ObjType::Icosahedron { radius } =>
            format!("Icosahedron(edge_length={:.2}, fill_color={}, fill_opacity={:.2})", radius, color, obj.fill_opacity),
        ObjType::Dodecahedron { radius } =>
            format!("Dodecahedron(edge_length={:.2}, fill_color={}, fill_opacity={:.2})", radius, color, obj.fill_opacity),
        ObjType::TracedPath =>
            format!("TracedPath(Dot(color={}).get_center)", color),
        ObjType::PointCloudDot =>
            format!("PointCloudDot(color={})", color),
    };

    let mut code = format!("        {} = {}", var, constructor);

    // Transform
    if obj.position != [0.0, 0.0, 0.0] {
        code.push_str(&format!(
            "\n        {}.move_to(np.array([{:.3}, {:.3}, {:.3}]))",
            var, obj.position[0], obj.position[1], obj.position[2]
        ));
    }
    if (obj.scale - 1.0).abs() > 0.001 {
        code.push_str(&format!("\n        {}.scale({:.3})", var, obj.scale));
    }
    if obj.rotation.abs() > 0.01 {
        code.push_str(&format!("\n        {}.rotate({}*DEGREES)", var, obj.rotation));
    }
    if (obj.opacity - 1.0).abs() > 0.001 {
        code.push_str(&format!("\n        {}.set_opacity({:.3})", var, obj.opacity));
    }

    code
}

fn build_anim_arg(anim: &AnimEntry, scene: &Scene) -> String {
    let obj = scene.get_object(&anim.object_id);
    let var = obj.map(|o| o.var_name()).unwrap_or_else(|| "obj".into());

    match &anim.anim_type {
        AnimType::Create => format!("Create({})", var),
        AnimType::Uncreate => format!("Uncreate({})", var),
        AnimType::Write => format!("Write({})", var),
        AnimType::FadeIn => format!("FadeIn({})", var),
        AnimType::FadeOut => format!("FadeOut({})", var),
        AnimType::GrowFromCenter => format!("GrowFromCenter({})", var),
        AnimType::DrawBorderThenFill => format!("DrawBorderThenFill({})", var),
        AnimType::SpinInFromNothing => format!("SpinInFromNothing({})", var),
        AnimType::GrowFromPoint { point } =>
            format!("GrowFromPoint({}, point=np.array([{:.2}, {:.2}, {:.2}]))", var, point[0], point[1], point[2]),
        AnimType::GrowFromEdge { edge } =>
            format!("GrowFromEdge({}, edge=np.array([{:.2}, {:.2}, {:.2}]))", var, edge[0], edge[1], edge[2]),
        AnimType::GrowArrow => format!("GrowArrow({})", var),
        AnimType::SpiralIn => format!("SpiralIn({})", var),
        AnimType::FadeInFromPoint { point } =>
            format!("FadeIn({}, target_position=np.array([{:.2}, {:.2}, {:.2}]))", var, point[0], point[1], point[2]),
        AnimType::FadeInFrom { direction } =>
            format!("FadeIn({}, shift=np.array([{:.2}, {:.2}, {:.2}]))", var, direction[0], direction[1], direction[2]),
        AnimType::FadeOutToPoint { point } =>
            format!("FadeOut({}, target_position=np.array([{:.2}, {:.2}, {:.2}]))", var, point[0], point[1], point[2]),
        AnimType::FadeOutAndShift { direction } =>
            format!("FadeOut({}, shift=np.array([{:.2}, {:.2}, {:.2}]))", var, direction[0], direction[1], direction[2]),
        AnimType::VFadeIn => format!("VFadeIn({})", var),
        AnimType::VFadeOut => format!("VFadeOut({})", var),
        AnimType::ShowCreationThenFadeOut =>
            format!("ShowCreationThenFadeOut({})", var),
        AnimType::AddTextLetterByLetter =>
            format!("AddTextLetterByLetter({})", var),
        AnimType::RemoveTextLetterByLetter =>
            format!("RemoveTextLetterByLetter({})", var),
        AnimType::Unwrite => format!("Unwrite({})", var),
        AnimType::ShrinkToCenter => format!("ShrinkToCenter({})", var),
        AnimType::Shift { dx, dy } =>
            format!("{}.animate.shift(np.array([{:.2}, {:.2}, 0]))", var, dx, dy),
        AnimType::MoveTo { x, y } =>
            format!("{}.animate.move_to(np.array([{:.2}, {:.2}, 0]))", var, x, y),
        AnimType::MoveAlongPath { path_points } => {
            let pts: Vec<String> = path_points.iter().map(|p| format!("[{:.2}, {:.2}, 0]", p[0], p[1])).collect();
            format!("MoveAlongPath({}, VMobject().set_points_as_corners([{}]))", var, pts.join(", "))
        }
        AnimType::Rotate { angle } =>
            format!("{}.animate.rotate({}*DEGREES)", var, angle),
        AnimType::Scale { factor } =>
            format!("{}.animate.scale({})", var, factor),
        AnimType::ScaleInPlace { factor } =>
            format!("ScaleInPlace({}, {})", var, factor),
        AnimType::Transform { target_id } => {
            let tgt = scene.get_object(target_id)
                .map(|o| o.var_name())
                .unwrap_or_else(|| "target".into());
            format!("Transform({}, {})", var, tgt)
        }
        AnimType::FadeTransform { target_id } => {
            let tgt = scene.get_object(target_id)
                .map(|o| o.var_name())
                .unwrap_or_else(|| "target".into());
            format!("FadeTransform({}, {})", var, tgt)
        }
        AnimType::ReplacementTransform { target_id } => {
            let tgt = scene.get_object(target_id)
                .map(|o| o.var_name())
                .unwrap_or_else(|| "target".into());
            format!("ReplacementTransform({}, {})", var, tgt)
        }
        AnimType::TransformFromCopy { target_id } => {
            let tgt = scene.get_object(target_id)
                .map(|o| o.var_name())
                .unwrap_or_else(|| "target".into());
            format!("TransformFromCopy({}, {})", var, tgt)
        }
        AnimType::CounterclockwiseTransform { target_id } => {
            let tgt = scene.get_object(target_id)
                .map(|o| o.var_name())
                .unwrap_or_else(|| "target".into());
            format!("CounterclockwiseTransform({}, {})", var, tgt)
        }
        AnimType::ClockwiseTransform { target_id } => {
            let tgt = scene.get_object(target_id)
                .map(|o| o.var_name())
                .unwrap_or_else(|| "target".into());
            format!("ClockwiseTransform({}, {})", var, tgt)
        }
        AnimType::CyclicReplace { object_ids } => {
            let vars: Vec<String> = object_ids.iter().filter_map(|id| {
                scene.get_object(id).map(|o| o.var_name())
            }).collect();
            format!("CyclicReplace({})", vars.join(", "))
        }
        AnimType::Swap { object_ids } => {
            let vars: Vec<String> = object_ids.iter().filter_map(|id| {
                scene.get_object(id).map(|o| o.var_name())
            }).collect();
            format!("Swap({})", vars.join(", "))
        }
        AnimType::ApplyMethod { method } =>
            format!("{}.animate.{}()", var, method),
        AnimType::Restore =>
            format!("Restore({})", var),
        AnimType::ChangeDecimalToValue { value } =>
            format!("ChangeDecimalToValue({}, {})", var, value),
        AnimType::Flash => format!("Flash({})", var),
        AnimType::Indicate => format!("Indicate({})", var),
        AnimType::Wiggle => format!("Wiggle({})", var),
        AnimType::ApplyWave => format!("ApplyWave({})", var),
        AnimType::Circumscribe => format!("Circumscribe({})", var),
        AnimType::ShowPassingFlash => format!("ShowPassingFlash({})", var),
        AnimType::FocusOn => format!("FocusOn({})", var),
        AnimType::Broadcast => format!("Broadcast({})", var),
        AnimType::ShowIncreasingSubsets => format!("ShowIncreasingSubsets({})", var),
        AnimType::ShowSubmobjectsOneByOne => format!("ShowSubmobjectsOneByOne({})", var),
        AnimType::AnimatedBoundary => format!("AnimatedBoundary({})", var),
        AnimType::MoveCamera { phi, theta, zoom, frame_center } => {
            let mut parts = vec![];
            if let Some(p) = phi { parts.push(format!("phi={}*DEGREES", p)); }
            if let Some(t) = theta { parts.push(format!("theta={}*DEGREES", t)); }
            if let Some(z) = zoom { parts.push(format!("zoom={}", z)); }
            if let Some(fc) = frame_center {
                parts.push(format!("frame_center=np.array([{:.2}, {:.2}, {:.2}])", fc[0], fc[1], fc[2]));
            }
            format!("self.move_camera({})", parts.join(", "))
        }
        AnimType::CameraRotate { angle } =>
            format!("self.move_camera(theta={}*DEGREES)", angle),
        AnimType::CameraZoom { factor } =>
            format!("self.camera.frame.animate.scale({})", factor),
        AnimType::CameraPan { dx, dy } =>
            format!("self.camera.frame.animate.shift(np.array([{:.2}, {:.2}, 0]))", dx, dy),
        AnimType::Succession { animations } =>
            format!("Succession({})", animations.join(", ")),
        AnimType::AnimationGroup { animations } =>
            format!("AnimationGroup({})", animations.join(", ")),
        AnimType::LaggedStart { lag_ratio } =>
            format!("LaggedStart(Create({}), lag_ratio={})", var, lag_ratio),
        AnimType::LaggedStartMap { lag_ratio } =>
            format!("LaggedStartMap(FadeIn, {}, lag_ratio={})", var, lag_ratio),
        AnimType::Wait => "Wait()  # should use self.wait() instead".into(),
        AnimType::Homotopy => format!("Homotopy(lambda x, y, z, t: (x, y, z), {})", var),
        AnimType::PhaseFlow => format!("PhaseFlow(lambda p: p, {})", var),
        AnimType::UpdateFromFunc => format!("UpdateFromFunc({}, lambda m: m)", var),
        AnimType::UpdateFromAlphaFunc => format!("UpdateFromAlphaFunc({}, lambda m, alpha: m)", var),
    }
}

fn escape_str(s: &str) -> String {
    s.replace('"', "\\\"").replace('\n', "\\n")
}