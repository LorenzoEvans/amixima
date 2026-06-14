pub mod ini;
pub mod jsonld;

use crate::core::soundcourse::Soundcourse;
use crate::error::{AmiximaError, Result};
use std::path::Path;

pub struct SoundcourseParser;

impl SoundcourseParser {
    pub fn parse_path(path: &Path) -> Result<Soundcourse> {
        Self::parse_path_with_mode(path, ini::IniParseMode::Strict)
    }

    pub fn parse_path_lenient(path: &Path) -> Result<Soundcourse> {
        Self::parse_path_with_mode(path, ini::IniParseMode::Lenient)
    }

    fn parse_path_with_mode(path: &Path, ini_mode: ini::IniParseMode) -> Result<Soundcourse> {
        match path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_lowercase)
        {
            Some(ext) if ext == "ini" => ini::parse_ini(path, ini_mode),
            Some(ext) if ext == "json" || ext == "jsonld" => Self::parse_json_ld(path),
            Some(ext) => Err(AmiximaError::UnsupportedFormat(format!(
                "{} files are not supported Soundcourses",
                ext
            ))),
            None => Err(AmiximaError::UnsupportedFormat(
                "Soundcourse path has no file extension".to_string(),
            )),
        }
    }

    pub fn parse_json_ld(path: &Path) -> Result<Soundcourse> {
        jsonld::parse_json_ld(path)
    }

    pub fn serialize_to_ini(soundcourse: &Soundcourse, path: &Path) -> Result<()> {
        ini::serialize_to_ini(soundcourse, path)
    }

    pub fn serialize_to_json_ld(soundcourse: &Soundcourse, path: &Path) -> Result<()> {
        jsonld::serialize_to_json_ld(soundcourse, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::soundcourse::EffectNode;

    #[test]
    fn test_serialize_ini() -> Result<()> {
        let mut sc = Soundcourse::new("Tester");
        sc.sequence.push(EffectNode::Reverb {
            room_size: 0.9,
            dry_wet: 0.1,
        });
        sc.sequence.push(EffectNode::EQ {
            frequency: 440.0,
            gain: -3.0,
        });

        let path =
            std::env::temp_dir().join(format!("test_serialize_{}.ini", uuid::Uuid::new_v4()));
        SoundcourseParser::serialize_to_ini(&sc, &path)?;

        let sc2 = SoundcourseParser::parse_path(&path)?;
        assert_eq!(sc2.sequence.len(), 2);

        if let EffectNode::Reverb { room_size, dry_wet } = sc2.sequence[0] {
            assert!((room_size - 0.9).abs() < 1e-6);
            assert!((dry_wet - 0.1).abs() < 1e-6);
        } else {
            panic!("Expected Reverb");
        }

        if let EffectNode::EQ { frequency, gain } = sc2.sequence[1] {
            assert!((frequency - 440.0).abs() < 1e-6);
            assert!((gain - (-3.0)).abs() < 1e-6);
        } else {
            panic!("Expected EQ");
        }

        std::fs::remove_file(path)?;
        Ok(())
    }

    #[test]
    fn test_json_ld_round_trip_all_effects() -> Result<()> {
        let mut sc = Soundcourse::new("Tester");
        sc.title = Some("Full Chain".to_string());
        sc.sequence.push(EffectNode::Reverb {
            room_size: 0.9,
            dry_wet: 0.1,
        });
        sc.sequence.push(EffectNode::EQ {
            frequency: 440.0,
            gain: -3.0,
        });
        sc.sequence.push(EffectNode::Delay {
            delay_ms: 120.0,
            feedback: 0.4,
        });
        sc.sequence.push(EffectNode::Compressor {
            threshold: -18.0,
            ratio: 3.0,
        });
        sc.sequence.push(EffectNode::Gain { gain_db: 1.5 });

        let path =
            std::env::temp_dir().join(format!("test_soundcourse_{}.jsonld", uuid::Uuid::new_v4()));
        SoundcourseParser::serialize_to_json_ld(&sc, &path)?;

        let sc2 = SoundcourseParser::parse_json_ld(&path)?;
        assert_eq!(sc2.sequence.len(), 5);
        assert_eq!(sc2.title.as_deref(), Some("Full Chain"));

        std::fs::remove_file(path)?;
        Ok(())
    }
}
