use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub fn root() -> Result<PathBuf> {
    let root = if let Some(value) = std::env::var_os("STEAMPROJECT_HOME") {
        PathBuf::from(value)
    } else {
        dirs::data_local_dir()
            .context("Cannot locate user data directory; set STEAMPROJECT_HOME")?
            .join("steamproject-update")
    };
    private_dir(&root)?;
    Ok(root)
}
pub fn private_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
pub fn runtime() -> Result<PathBuf> {
    let p = root()?.join("runtime");
    private_dir(&p)?;
    Ok(p)
}
pub fn config(app_id: u32) -> Result<PathBuf> {
    let p = root()?.join("apps").join(app_id.to_string());
    private_dir(&p)?;
    Ok(p.join("store.toml"))
}
