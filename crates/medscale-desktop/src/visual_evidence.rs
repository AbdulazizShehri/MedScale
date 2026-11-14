//! Opt-in, bounded snapshots of the actual native desktop (Spec 095).

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const ROUTES: [&str; 25] = [
    "Home",
    "Patients",
    "Documents",
    "Projects",
    "Data",
    "Insights",
    "Models",
    "Evidence",
    "Workflows",
    "Tasks",
    "Messages",
    "Collaboration",
    "MedAgent",
    "Model Fleet",
    "Browse",
    "Audio",
    "Analytics",
    "Knowledge",
    "Research OS",
    "Privacy",
    "Audit Trail",
    "Exports",
    "Integrations",
    "Settings",
    "About",
];

#[derive(Debug, PartialEq, Eq)]
pub struct RenderOptions {
    pub output: PathBuf,
    pub route: String,
    pub theme: i32,
    pub width: u32,
    pub height: u32,
    pub compact: bool,
}

impl RenderOptions {
    pub fn parse(args: &[String]) -> Result<Option<Self>, &'static str> {
        let mut output = None;
        let mut route = "Home".to_owned();
        let mut theme = 1;
        let mut size = (1440, 900);
        let mut compact = false;
        let mut has_render_option = false;
        let mut index = 0;
        while index < args.len() {
            let flag = args[index].as_str();
            if flag == "--render-compact" {
                if compact {
                    return Err("duplicate render option");
                }
                compact = true;
                has_render_option = true;
            } else if matches!(
                flag,
                "--render-evidence" | "--render-route" | "--render-theme" | "--render-size"
            ) {
                let value = args
                    .get(index + 1)
                    .ok_or("render option requires a value")?;
                if value.starts_with("--") {
                    return Err("render option requires a value");
                }
                if args[..index].iter().any(|previous| previous == flag) {
                    return Err("duplicate render option");
                }
                has_render_option = true;
                match flag {
                    "--render-evidence" => {
                        let path = PathBuf::from(value);
                        if !path.is_absolute() || path.extension().is_none_or(|ext| ext != "ppm") {
                            return Err("render output must be an absolute .ppm file path");
                        }
                        if path.exists() {
                            return Err("render output already exists; use a new evidence path");
                        }
                        output = Some(path);
                    }
                    "--render-route" => {
                        if !ROUTES.contains(&value.as_str()) {
                            return Err("unrecognized render route");
                        }
                        route.clone_from(value);
                    }
                    "--render-theme" => {
                        theme = match value.as_str() {
                            "light" => 1,
                            "dark" => 2,
                            _ => return Err("render theme must be light or dark"),
                        };
                    }
                    "--render-size" => {
                        let (width, height) = value
                            .split_once('x')
                            .ok_or("render size must be WIDTHxHEIGHT")?;
                        let width: u32 = width.parse().map_err(|_| "invalid render width")?;
                        let height: u32 = height.parse().map_err(|_| "invalid render height")?;
                        if !(1100..=1920).contains(&width) || !(720..=1200).contains(&height) {
                            return Err("render size must be between 1100x720 and 1920x1200");
                        }
                        size = (width, height);
                    }
                    _ => unreachable!(),
                }
                index += 1;
            } else if flag.starts_with("--render-") {
                return Err("unrecognized render option");
            }
            index += 1;
        }
        let Some(output) = output else {
            return if has_render_option {
                Err("render options require --render-evidence")
            } else {
                Ok(None)
            };
        };
        if args
            .iter()
            .any(|arg| arg == "--smoke" || arg == "--perf-idle-ms")
        {
            return Err("render evidence cannot be combined with smoke or performance probes");
        }
        Ok(Some(Self {
            output,
            route,
            theme,
            width: size.0,
            height: size.1,
            compact,
        }))
    }

    /// Never use the normal user's vault or a synced workspace for capture.
    pub fn synthetic_vault_root(&self) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time must follow Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "medscale-native-capture-095-{}-{nonce}",
            std::process::id()
        ))
    }
}

pub fn write_ppm(
    path: &Path,
    pixels: &slint::SharedPixelBuffer<slint::Rgba8Pixel>,
) -> io::Result<()> {
    let file = File::create_new(path)?;
    let mut writer = BufWriter::new(file);
    write!(writer, "P6\n{} {}\n255\n", pixels.width(), pixels.height())?;
    for pixel in pixels.as_slice() {
        writer.write_all(&[pixel.r, pixel.g, pixel.b])?;
    }
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn output() -> String {
        std::env::temp_dir()
            .join("medscale-render-options-095.ppm")
            .display()
            .to_string()
    }

    #[test]
    fn render_requires_explicit_bounded_options_and_known_route() {
        assert_eq!(RenderOptions::parse(&args(&[])), Ok(None));
        for invalid in [
            args(&["--render-route", "Home"]),
            args(&["--render-evidence", "relative.ppm"]),
            args(&["--render-evidence"]),
            args(&["--render-evidence", &output(), "--render-size", "1099x720"]),
            args(&["--render-evidence", &output(), "--render-size", "1440x1201"]),
            args(&["--render-evidence", &output(), "--render-route", "Made up"]),
            args(&["--render-evidence", &output(), "--render-theme", "purple"]),
            args(&["--render-evidence", &output(), "--smoke"]),
            args(&[
                "--render-evidence",
                &output(),
                "--render-theme",
                "dark",
                "--render-theme",
                "light",
            ]),
            args(&["--render-evidence", &output(), "--render-unknown"]),
        ] {
            assert!(
                RenderOptions::parse(&invalid).is_err(),
                "accepted {invalid:?}"
            );
        }
        for route in ROUTES {
            let parsed = RenderOptions::parse(&args(&[
                "--render-evidence",
                &output(),
                "--render-route",
                route,
                "--render-theme",
                "dark",
                "--render-size",
                "1100x720",
                "--render-compact",
            ]))
            .unwrap()
            .unwrap();
            assert_eq!(
                (parsed.width, parsed.height, parsed.theme, parsed.compact),
                (1100, 720, 2, true)
            );
            assert_eq!(parsed.route, route);
            let synthetic_root = parsed.synthetic_vault_root();
            assert_ne!(synthetic_root, parsed.output);
            assert!(synthetic_root.starts_with(std::env::temp_dir()));
            assert_ne!(
                synthetic_root,
                parsed.output.parent().unwrap().join("synthetic-vault-095")
            );
            assert!(
                synthetic_root
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("medscale-native-capture-095-")
            );
        }
    }

    #[test]
    fn native_pixel_export_preserves_rgb_dimensions_and_refuses_overwrite() {
        let path =
            std::env::temp_dir().join(format!("medscale-pixels-095-{}.ppm", std::process::id()));
        let mut pixels = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(2, 1);
        pixels.make_mut_slice()[0] = slint::Rgba8Pixel::new(0, 0, 0, 255);
        pixels.make_mut_slice()[1] = slint::Rgba8Pixel::new(255, 255, 255, 255);
        write_ppm(&path, &pixels).unwrap();
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"P6\n2 1\n255\n\0\0\0\xff\xff\xff"
        );
        assert_eq!(
            write_ppm(&path, &pixels).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        std::fs::remove_file(path).unwrap();
    }
}
