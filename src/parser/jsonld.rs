use crate::core::soundcourse::Soundcourse;
use crate::error::{AmiximaError, Result};
use std::path::Path;

pub fn parse_json_ld(path: &Path) -> Result<Soundcourse> {
    let data = std::fs::read_to_string(path).map_err(|err| AmiximaError::io(path, err))?;
    serde_json::from_str(&data).map_err(AmiximaError::from)
}

pub fn serialize_to_json_ld(soundcourse: &Soundcourse, path: &Path) -> Result<()> {
    let data = soundcourse.to_json_ld()?;
    std::fs::write(path, data).map_err(|err| AmiximaError::io(path, err))
}
