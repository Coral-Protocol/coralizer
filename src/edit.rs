use std::{error::Error, io, path::Path};

pub fn edit_file_str<S, E, F>(path: impl AsRef<Path>, edit_fn: F) -> Result<(), E>
where
    S: AsRef<[u8]>,
    E: Error + From<io::Error>,
    F: FnOnce(String) -> Result<S, E>,
{
    let content = std::fs::read_to_string(&path)?;
    std::fs::write(path, edit_fn(content)?)?;
    Ok(())
}
