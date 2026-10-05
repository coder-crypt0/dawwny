use std::path::PathBuf;

/// Persistent user data must not depend on where the executable was launched.
pub fn default_data_directory() -> anyhow::Result<PathBuf> {
    let variable = if cfg!(windows) {
        "LOCALAPPDATA"
    } else {
        "HOME"
    };
    let base = std::env::var_os(variable)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("{variable} is unavailable; choose --project explicitly"))?;
    let directory = if cfg!(windows) {
        base.join("dawwny")
    } else if cfg!(target_os = "macos") {
        base.join("Library/Application Support/dawwny")
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| base.join(".local/share"))
            .join("dawwny")
    };
    anyhow::ensure!(
        directory.is_absolute(),
        "User data directory must be absolute"
    );
    Ok(directory)
}
