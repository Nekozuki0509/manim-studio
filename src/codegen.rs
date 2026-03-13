use crate::scene::{AnimEntry, AnimType, ManimObject, ObjType, Scene};

/// Generate a complete Manim Python script from the current scene.
pub fn generate(scene: &Scene) -> String {
    let mut lines: Vec<String> = vec![];

    let base_class = if scene.is_3d { "ThreeDScene" } else { "Scene" };

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
        lines.push(build_object_code(obj, scene.require_latex));
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

fn build_object_code(obj: &ManimObject, require_latex: bool) -> String {
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
                // No LaTeX installed — fall back to Text with a comment
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
        AnimType::Rotate { angle } =>
            format!("{}.animate.rotate({}*DEGREES)", var, angle),
        AnimType::Scale { factor } =>
            format!("{}.animate.scale({})", var, factor),
        AnimType::Shift { dx, dy } =>
            format!("{}.animate.shift(np.array([{:.2}, {:.2}, 0]))", var, dx, dy),
        AnimType::MoveTo { x, y } =>
            format!("{}.animate.move_to(np.array([{:.2}, {:.2}, 0]))", var, x, y),
        AnimType::Transform { target_id } => {
            let tgt = scene.get_object(target_id)
                .map(|o| o.var_name())
                .unwrap_or_else(|| "target".into());
            format!("Transform({}, {})", var, tgt)
        }
        AnimType::Flash => format!("Flash({})", var),
        AnimType::Indicate => format!("Indicate({})", var),
        AnimType::Wiggle => format!("Wiggle({})", var),
        AnimType::Wait => "Wait()  # should use self.wait() instead".into(),
    }
}

fn escape_str(s: &str) -> String {
    s.replace('"', "\\\"").replace('\n', "\\n")
}