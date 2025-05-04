use crate::home::home;
use anyhow::Context;
use std::fs;
use std::path::{Path, PathBuf};

/// Represents a path relative from the user's home
#[derive(Debug)]
pub struct PathFromHome {
    absolute: PathBuf,
    from_home: PathBuf,
}

impl PathFromHome {
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        fs::create_dir_all(&path)?;
        let absolute = path
            .canonicalize()
            .with_context(|| format!("failed to calculate absolute path for {}", path.display()))?;

        let home = home()?;
        let from_home = absolute
            .strip_prefix(&home)
            .with_context(|| {
                format!(
                    "path {} is not relative to {}",
                    absolute.display(),
                    home.display()
                )
            })?
            .to_owned();

        Ok(PathFromHome {
            absolute,
            from_home,
        })
    }

    pub fn join(&self, more: impl AsRef<Path>) -> Self {
        PathFromHome {
            absolute: self.absolute.join(more.as_ref()),
            from_home: self.from_home.join(more.as_ref()),
        }
    }

    pub fn to_systemd_string(&self) -> anyhow::Result<String> {
        let from_home = self
            .from_home
            .to_str()
            .context("failed to convert path to string")?;
        Ok(format!("%h/{}", from_home))
    }
}

impl AsRef<Path> for PathFromHome {
    fn as_ref(&self) -> &Path {
        &self.absolute
    }
}
