use std::env;
use std::path::PathBuf;

pub const PROTOCOL_VERSION: u32 = 1;

pub fn runtime_dir() -> Result<PathBuf, RuntimePathError> {
    let runtime_dir = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or(RuntimePathError::MissingEnvironment("XDG_RUNTIME_DIR"))?;
    Ok(runtime_dir.join("hyprbole"))
}

pub fn log_path(name: &str) -> Result<PathBuf, RuntimePathError> {
    Ok(runtime_dir()?.join(format!("{name}.log")))
}

pub fn ensure_runtime_dir() -> Result<PathBuf, RuntimePathError> {
    let path = runtime_dir()?;
    std::fs::create_dir_all(&path).map_err(|source| RuntimePathError::CreateDir {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

#[derive(Debug)]
pub enum RuntimePathError {
    MissingEnvironment(&'static str),
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for RuntimePathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEnvironment(name) => write!(f, "{name} is not set"),
            Self::CreateDir { path, source } => {
                write!(f, "create runtime dir {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for RuntimePathError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_path_uses_hyprbole_runtime_dir() {
        let path = log_path("daemon").unwrap();
        assert!(path.ends_with("hyprbole/daemon.log"));
    }
}
