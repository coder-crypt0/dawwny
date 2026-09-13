use crate::{Command, Project, apply_commands, validate};
use anyhow::{Context, Result, ensure};
use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use tempfile::NamedTempFile;

pub fn new_id() -> String {
    static N: AtomicU64 = AtomicU64::new(0);
    format!(
        "{:x}-{:x}-{:x}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    )
}
fn parent(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}
pub fn load_project(path: &Path) -> Result<Project> {
    let mut data = Vec::new();
    File::open(path)
        .with_context(|| format!("Cannot open {}", path.display()))?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut data)?;
    ensure!(
        data.len() <= 16 * 1024 * 1024,
        "Project exceeds 16 MiB limit"
    );
    let p: Project = serde_json::from_slice(&data).context("Invalid project JSON")?;
    validate(&p)?;
    Ok(p)
}
pub fn save_project(path: &Path, p: &Project) -> Result<()> {
    validate(p)?;
    fs::create_dir_all(parent(path))?;
    let mut temp = NamedTempFile::new_in(parent(path))?;
    serde_json::to_writer_pretty(&mut temp, p)?;
    temp.flush()?;
    temp.as_file().sync_all()?;
    temp.persist(path)
        .map_err(|e| e.error)
        .context("Could not atomically save project")?;
    Ok(())
}

#[derive(Clone)]
pub struct SessionStore {
    path: PathBuf,
}
impl SessionStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn load(&self) -> Result<Project> {
        load_project(&self.path)
    }
    fn lock(&self) -> Result<File> {
        fs::create_dir_all(parent(&self.path))?;
        let mut lock_path = self.path.as_os_str().to_os_string();
        lock_path.push(".lock");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(PathBuf::from(lock_path))?;
        file.try_lock_exclusive()
            .context("Session is busy; retry this operation")?;
        Ok(file)
    }
    pub fn initialize(&self, p: &Project) -> Result<Project> {
        let _lock = self.lock()?;
        if !self.path.exists() {
            save_project(&self.path, p)?;
        }
        self.load()
    }
    pub fn transact(&self, expected_revision: u64, commands: &[Command]) -> Result<Project> {
        let _lock = self.lock()?;
        let p = self.load()?;
        ensure!(
            p.revision == expected_revision,
            "Revision conflict: expected {expected_revision}, current {}. Read the session again.",
            p.revision
        );
        let next = apply_commands(&p, commands)?;
        save_project(&self.path, &next)?;
        Ok(next)
    }
    pub fn replace(&self, expected_revision: u64, project: &Project) -> Result<Project> {
        let _lock = self.lock()?;
        let current = self.load()?;
        ensure!(
            current.revision == expected_revision,
            "Revision conflict: expected {expected_revision}, current {}",
            current.revision
        );
        let mut next = project.clone();
        next.id = current.id;
        next.revision = current
            .revision
            .checked_add(1)
            .context("Revision limit reached")?;
        save_project(&self.path, &next)?;
        Ok(next)
    }
}
