# 🎬 Manim Studio

**Blender/Premiere-style GUI for creating Manim animations — built in Rust with egui**

```
┌────────────────────────────────────────────────────────────────────────┐
│ 🎬 Manim Studio  File  Edit  Add  Scene  Render  Help              ✅ │
│ 🖱Select  ✋Pan  ⬤ ■ ▬ ▲ T ∑ → ╱ ⬡ More…   ▶ ⏹  3.21s / 10.00s  │
├────────────────┬──────────────────────────────────────┬────────────────┤
│ 🎭 Objects     │                                      │ 📦 Properties  │
│ ⬤ Circle_1    │      [  Manim Viewport (2D/3D)  ]    │ Name: Circle_1 │
│ ■ Square_1    │                                      │ Position X Y Z │
│ ∑ Title       │      (drag to move, scroll=zoom)     │ Scale / Rot    │
│ ➕ Add         │                                      │ Color / Opacity│
│ ┌────────────┐ │                                      │ Type Props…    │
│ │2D  3D Spec │ │                                      │────────────────│
│ └────────────┘ │                                      │ 🎞 Animation   │
│ 🎞 Animations  │                                      │ Start / Dur    │
│ Create 0.00s  │                                      │ Easing curve   │
├────────────────┴──────────────────────────────────────┴────────────────┤
│ ⏱ Timeline          Duration: 10s  Zoom: ─●─                          │
│  Circle ─── [Create████] ──── [Shift████] ── [FadeOut██]              │
│  Square ──────── [Create████] ──── [Rotate████] ── [FadeOut██]        │
│  Title  ─────────────── [Write█████████] ─── [FadeOut██]              │
└────────────────────────────────────────────────────────────────────────┘
```

## ✨ Features

- **Scene Viewport** — Real-time 2D preview of all Manim objects  
  - Drag objects to move them  
  - Scroll to zoom, Select/Pan tool modes  
  - Grid with Manim coordinate system (origin at center)  
  - Click to select, handles shown for selected objects

- **Object Panel** — Full scene hierarchy  
  - Add 14 object types: Circle, Square, Rectangle, Triangle, Text, MathTex, Arrow, Line, Dot, Sphere, Cube, Cylinder, Axes, NumberPlane  
  - Right-click context menu (delete, duplicate, add animation)  
  - Quick-add animation at playhead position

- **Properties Panel** — Edit everything  
  - Position (X Y Z drag values)  
  - Scale, Rotation  
  - Fill color picker, opacity, stroke width  
  - Type-specific fields (radius, LaTeX content, font size, …)  
  - Animation: start time, duration, easing function

- **NLE Timeline** — Premiere-style animation editor  
  - Colour-coded tracks per object  
  - Animations shown as coloured blocks  
  - Click ruler to seek; playback preview  
  - Horizontal scroll & zoom

- **Code Generator** — Outputs clean Manim Python  
  - Groups simultaneous animations into `self.play(A, B, …)`  
  - Handles all object transforms and animation params  
  - Copy to clipboard or edit before rendering

- **Render Integration** — One-click Manim render  
  - 480p / 720p / 1080p / 4K quality presets  
  - Configurable output directory  
  - Status shown in toolbar

## 🛠 Setup

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 2. Install Manim (Python)

```bash
pip install manim
# For MathTex support also install LaTeX:
# macOS:  brew install --cask mactex
# Ubuntu: sudo apt install texlive-full
# Windows: https://miktex.org/
```

### 3. Build & Run

```bash
cd manim-studio
cargo run --release
```

First build downloads dependencies (~200 MB) and takes ~2 min. Subsequent runs are instant.

## 🎮 Usage

1. **Add objects** — click object buttons in toolbar or drag from the left panel
2. **Position objects** — drag in the viewport, or type exact values in Properties
3. **Add animations** — right-click an object → "Add Animation", or use quick-add buttons  
4. **Set timing** — drag animation blocks in the timeline, or edit Start/Duration in Properties
5. **Preview** — click ▶ to see a timeline preview in the viewport
6. **Render** — click 🚀 or go to Render menu → the full Manim render runs automatically

## 🗂 Project Structure

```
manim-studio/
├── Cargo.toml
└── src/
    ├── main.rs          # Entry point
    ├── app.rs           # App state + eframe::App impl
    ├── scene.rs         # All scene data structures (ManimObject, AnimEntry, …)
    ├── codegen.rs       # Python code generator
    ├── renderer.rs      # Manim CLI subprocess wrapper
    └── ui/
        ├── mod.rs
        ├── viewport.rs  # 2D scene preview
        ├── timeline.rs  # NLE timeline editor
        ├── toolbar.rs   # Menu bar + toolbar
        ├── objects_panel.rs  # Left panel
        ├── properties.rs     # Right panel
        └── windows.rs        # Floating windows (code, settings, about)
```

## 🚀 Extending

### Add a new object type
1. Add variant to `ObjType` enum in `scene.rs`
2. Add drawing code in `viewport.rs` `draw_object()` match arm
3. Add Python constructor in `codegen.rs` `build_object_code()` match arm
4. Add to `object_templates()` in `scene.rs`

### Add a new animation type
1. Add variant to `AnimType` enum in `scene.rs` with `track_color()` and `default_duration()`
2. Add Python code in `codegen.rs` `build_anim_arg()` match arm
3. Add type-specific UI in `properties.rs` `show_anim_props()`

## 🛣 Roadmap

- [ ] wgpu 3D viewport (true 3D preview using Three.js-style rendering)
- [ ] Keyframe editor for property interpolation
- [ ] Save/Load scenes as JSON
- [ ] Video scrubbing (render frames on the fly)
- [ ] Bezier path editor
- [ ] VGroup support and object parenting
- [ ] Plugin system for custom Manim animations
- [ ] Undo/Redo history
