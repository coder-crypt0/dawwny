use crate::studio::EditorTab;
use std::{ffi::OsString, path::PathBuf};

pub struct StudioOptions {
    pub project: PathBuf,
    pub no_audio: bool,
    pub view: EditorTab,
    pub expanded: bool,
    pub soundfont: Option<PathBuf>,
}

pub enum Launch {
    Studio(StudioOptions),
    Mcp { project: PathBuf, exports: PathBuf },
    Help,
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> anyhow::Result<Launch> {
    let mut project = None;
    let mut exports = None;
    let mut mcp = false;
    let mut no_audio = false;
    let mut view = EditorTab::Piano;
    let mut expanded = false;
    let mut soundfont = None;
    let mut studio_options = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--mcp") => mcp = true,
            Some("--project" | "--export-dir" | "--soundfont") => {
                let value = args
                    .next()
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| anyhow::anyhow!("{} requires a path", arg.to_string_lossy()))?;
                anyhow::ensure!(
                    !value.to_string_lossy().starts_with("--"),
                    "{} requires a path, not another option",
                    arg.to_string_lossy()
                );
                match arg.to_str().unwrap() {
                    "--project" => project = Some(PathBuf::from(value)),
                    "--export-dir" => exports = Some(PathBuf::from(value)),
                    _ => {
                        soundfont = Some(PathBuf::from(value));
                        studio_options = true;
                    }
                }
            }
            Some("--no-audio") => {
                no_audio = true;
                studio_options = true;
            }
            Some("--expanded") => {
                expanded = true;
                studio_options = true;
            }
            Some("--view") => {
                view = match args.next().as_deref().and_then(|v| v.to_str()) {
                    Some("piano") => EditorTab::Piano,
                    Some("sound") => EditorTab::Sound,
                    Some("mixer") => EditorTab::Mixer,
                    _ => anyhow::bail!("--view must be piano, sound, or mixer"),
                };
                studio_options = true;
            }
            Some("--help" | "-h") => return Ok(Launch::Help),
            _ => anyhow::bail!("Unknown option: {}", arg.to_string_lossy()),
        }
    }
    anyhow::ensure!(
        !mcp || !studio_options,
        "Studio options cannot be used with --mcp"
    );
    anyhow::ensure!(mcp || exports.is_none(), "--export-dir requires --mcp");
    let project = match project {
        Some(path) => path,
        None => dawwny_core::default_data_directory()?.join("sessions/untitled.dawwny.json"),
    };
    if mcp {
        let exports = match exports {
            Some(path) => path,
            None => dawwny_core::default_data_directory()?.join("exports"),
        };
        Ok(Launch::Mcp { project, exports })
    } else {
        Ok(Launch::Studio(StudioOptions {
            project,
            no_audio,
            view,
            expanded,
            soundfont,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn single_executable_mcp_accepts_paths_with_spaces() {
        let launch = parse(args(&[
            "--project",
            "C:/Music/नया session.json",
            "--mcp",
            "--export-dir",
            "C:/My exports",
        ]))
        .unwrap();
        let Launch::Mcp { project, exports } = launch else {
            panic!("Expected MCP mode")
        };
        assert_eq!(project, PathBuf::from("C:/Music/नया session.json"));
        assert_eq!(exports, PathBuf::from("C:/My exports"));
    }

    #[test]
    fn invalid_modes_and_missing_values_fail() {
        for values in [
            vec!["--mcp", "--no-audio"],
            vec!["--mcp", "--view", "mixer"],
            vec!["--export-dir", "exports"],
            vec!["--project"],
            vec!["--project", "--mcp"],
            vec!["--unknown"],
        ] {
            assert!(parse(args(&values)).is_err(), "Accepted {values:?}");
        }
    }
}
