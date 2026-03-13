use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub enum RenderQuality {
    Low,
    Medium,
    High,
    Ultra,
}

impl RenderQuality {
    pub fn flag(&self) -> &str {
        match self {
            Self::Low    => "-ql",
            Self::Medium => "-qm",
            Self::High   => "-qh",
            Self::Ultra  => "-qk",
        }
    }
    #[allow(dead_code)]
    pub fn label(&self) -> &str {
        match self {
            Self::Low    => "480p  (Low)",
            Self::Medium => "720p  (Medium)",
            Self::High   => "1080p (High)",
            Self::Ultra  => "4K    (Ultra)",
        }
    }
    #[allow(dead_code)]
    pub fn all() -> Vec<Self> {
        vec![Self::Low, Self::Medium, Self::High, Self::Ultra]
    }
}

/// Write the Python source to a temp file, then spawn `manim` CLI.
/// Falls back to `python -m manim` if the `manim` binary is not found.
/// Returns Ok(output_path) or Err(full error message).
pub fn render(
    python_code: &str,
    scene_name: &str,
    quality: &RenderQuality,
    output_dir: &str,
    preview: bool,
) -> Result<PathBuf, String> {
    // Write script
    let tmp_dir = std::env::temp_dir().join("manim_studio");
    fs::create_dir_all(&tmp_dir)
        .map_err(|e| format!("Cannot create temp dir: {}", e))?;

    let script_path = tmp_dir.join(format!("{}.py", scene_name));
    fs::write(&script_path, python_code)
        .map_err(|e| format!("Cannot write script: {}", e))?;

    // Try running with the `manim` binary first, then fall back to `python -m manim`
    let result = try_run_manim(
        &["manim"],
        quality,
        output_dir,
        &script_path,
        scene_name,
        preview,
    );

    let output = match result {
        Ok(o) => o,
        Err(first_err) => {
            // `manim` not on PATH — try python / python3 -m manim
            let python_candidates: &[&str] = if cfg!(windows) {
                &["python", "python3", "py"]
            } else {
                &["python3", "python"]
            };

            let mut last_err = first_err;
            let mut found = None;
            for py in python_candidates {
                match try_run_manim(
                    &[py, "-m", "manim"],
                    quality,
                    output_dir,
                    &script_path,
                    scene_name,
                    preview,
                ) {
                    Ok(o) => { found = Some(o); break; }
                    Err(e) => { last_err = e; }
                }
            }
            match found {
                Some(o) => o,
                None => return Err(format!(
                    "Could not run manim. Make sure it is installed:\n  pip install manim\n\nLast error:\n{}",
                    last_err
                )),
            }
        }
    };

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!(
            "Manim exited with error:\n\nSTDERR:\n{}\n\nSTDOUT:\n{}",
            stderr.trim(),
            stdout.trim()
        ));
    }

    let video_path = guess_output_path(output_dir, scene_name, quality);
    Ok(video_path)
}

fn try_run_manim(
    cmd_parts: &[&str],
    quality: &RenderQuality,
    output_dir: &str,
    script_path: &Path,
    scene_name: &str,
    preview: bool,
) -> Result<std::process::Output, String> {
    let (bin, args) = cmd_parts.split_first().unwrap();
    let mut cmd = Command::new(bin);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.arg(quality.flag())
        .arg("--output_file")
        .arg(scene_name)
        .arg("--media_dir")
        .arg(output_dir)
        .arg(script_path)
        .arg(scene_name);

    if preview {
        cmd.arg("--preview");
    }

    // On Windows: augment PATH with common MiKTeX / TeX Live locations
    // so that Windows Store Python (which runs sandboxed) can find xelatex/latex.
    #[cfg(windows)]
    {
        let mut extra_paths: Vec<String> = vec![];

        // MiKTeX: check per-user and machine-wide install locations
        let miktex_roots: Vec<std::path::PathBuf> = {
            let mut roots = vec![];
            // Per-user (most common)
            if let Ok(local) = std::env::var("LOCALAPPDATA") {
                roots.push(std::path::PathBuf::from(&local).join("Programs").join("MiKTeX").join("miktex").join("bin").join("x64"));
            }
            // Machine-wide
            roots.push(std::path::PathBuf::from(r"C:\Program Files\MiKTeX\miktex\bin\x64"));
            roots.push(std::path::PathBuf::from(r"C:\Program Files (x86)\MiKTeX\miktex\bin"));
            // TeX Live typical locations
            for year in (2020u32..=2030).rev() {
                roots.push(std::path::PathBuf::from(format!(r"C:\texlive\{}\bin\windows", year)));
                roots.push(std::path::PathBuf::from(format!(r"C:\texlive\{}\bin\win32", year)));
            }
            roots
        };

        for p in &miktex_roots {
            if p.join("xelatex.exe").exists() || p.join("latex.exe").exists() {
                extra_paths.push(p.to_string_lossy().into_owned());
            }
        }

        if !extra_paths.is_empty() {
            let current_path = std::env::var("PATH").unwrap_or_default();
            let new_path = format!("{};{}", extra_paths.join(";"), current_path);
            cmd.env("PATH", new_path);
        }
    }

    cmd.output().map_err(|e| format!("Failed to launch `{}`: {}", bin, e))
}

fn guess_output_path(output_dir: &str, scene_name: &str, quality: &RenderQuality) -> PathBuf {
    let quality_dir = match quality {
        RenderQuality::Low    => "480p15",
        RenderQuality::Medium => "720p30",
        RenderQuality::High   => "1080p60",
        RenderQuality::Ultra  => "2160p60",
    };
    Path::new(output_dir)
        .join("videos")
        .join(scene_name)
        .join(quality_dir)
        .join(format!("{}.mp4", scene_name))
}