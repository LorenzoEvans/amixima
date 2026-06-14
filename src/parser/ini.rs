use crate::core::soundcourse::{EffectNode, Soundcourse};
use crate::error::{AmiximaError, Result};
use configparser::ini::Ini;
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub enum IniParseMode {
    Strict,
    Lenient,
}

pub fn parse_ini(path: &Path, mode: IniParseMode) -> Result<Soundcourse> {
    let mut config = Ini::new();
    config
        .load(path)
        .map_err(|err| AmiximaError::Parser(format!("{}: {err}", path.display())))?;

    let title = config.get("metadata", "title");
    let description = config.get("metadata", "description");
    let sample_rate = get_optional_float(&config, "metadata", "sample_rate")?;

    let mut sequence = Vec::new();
    let mut sections: Vec<String> = config
        .sections()
        .into_iter()
        .filter(|section| section.starts_with("effect"))
        .collect();
    sections.sort_by_key(|section| {
        section
            .strip_prefix("effect")
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(u32::MAX)
    });

    for section in sections {
        let Some(effect_type) = config.get(&section, "type") else {
            if matches!(mode, IniParseMode::Strict) {
                return Err(AmiximaError::Parser(format!(
                    "{section} is missing required type"
                )));
            }
            continue;
        };

        let node = match effect_type.to_lowercase().as_str() {
            "reverb" => EffectNode::Reverb {
                room_size: get_float_with_default(&config, &section, "room_size", 0.5, mode)?,
                dry_wet: get_float_with_default(&config, &section, "dry_wet", 0.3, mode)?,
            },
            "eq" => EffectNode::EQ {
                frequency: get_float_with_default(&config, &section, "frequency", 1000.0, mode)?,
                gain: get_float_with_default(&config, &section, "gain", 0.0, mode)?,
            },
            "delay" => EffectNode::Delay {
                delay_ms: get_float_with_default(&config, &section, "delay_ms", 100.0, mode)?,
                feedback: get_float_with_default(&config, &section, "feedback", 0.5, mode)?,
            },
            "compressor" => EffectNode::Compressor {
                threshold: get_float_with_default(&config, &section, "threshold", -20.0, mode)?,
                ratio: get_float_with_default(&config, &section, "ratio", 4.0, mode)?,
            },
            "gain" => EffectNode::Gain {
                gain_db: get_float_with_default(&config, &section, "gain_db", 0.0, mode)?,
            },
            unknown => {
                return Err(AmiximaError::Parser(format!(
                    "{section} has unknown effect type {unknown}"
                )))
            }
        };
        sequence.push(node);
    }

    let mut sc = Soundcourse::new("DefaultUser");
    sc.title = title;
    sc.description = description;
    sc.sample_rate = sample_rate.map(|v| v as f32);
    sc.sequence = sequence;
    Ok(sc)
}

pub fn serialize_to_ini(soundcourse: &Soundcourse, path: &Path) -> Result<()> {
    let mut config = Ini::new();

    config.set("metadata", "title", soundcourse.title.clone());
    if let Some(desc) = &soundcourse.description {
        config.set("metadata", "description", Some(desc.clone()));
    }
    if let Some(sr) = soundcourse.sample_rate {
        config.set("metadata", "sample_rate", Some(sr.to_string()));
    }

    for (i, node) in soundcourse.sequence.iter().enumerate() {
        let section = format!("effect{}", i + 1);
        match node {
            EffectNode::Reverb { room_size, dry_wet } => {
                config.set(&section, "type", Some("reverb".to_string()));
                config.set(&section, "room_size", Some(room_size.to_string()));
                config.set(&section, "dry_wet", Some(dry_wet.to_string()));
            }
            EffectNode::EQ { frequency, gain } => {
                config.set(&section, "type", Some("eq".to_string()));
                config.set(&section, "frequency", Some(frequency.to_string()));
                config.set(&section, "gain", Some(gain.to_string()));
            }
            EffectNode::Delay { delay_ms, feedback } => {
                config.set(&section, "type", Some("delay".to_string()));
                config.set(&section, "delay_ms", Some(delay_ms.to_string()));
                config.set(&section, "feedback", Some(feedback.to_string()));
            }
            EffectNode::Compressor { threshold, ratio } => {
                config.set(&section, "type", Some("compressor".to_string()));
                config.set(&section, "threshold", Some(threshold.to_string()));
                config.set(&section, "ratio", Some(ratio.to_string()));
            }
            EffectNode::Gain { gain_db } => {
                config.set(&section, "type", Some("gain".to_string()));
                config.set(&section, "gain_db", Some(gain_db.to_string()));
            }
        }
    }

    config
        .write(path)
        .map_err(|err| AmiximaError::io(path, err))
}

fn get_optional_float(config: &Ini, section: &str, key: &str) -> Result<Option<f64>> {
    config
        .getfloat(section, key)
        .map_err(|err| AmiximaError::Parser(format!("{section}.{key}: {err}")))
}

fn get_float_with_default(
    config: &Ini,
    section: &str,
    key: &str,
    default: f32,
    mode: IniParseMode,
) -> Result<f32> {
    match get_optional_float(config, section, key)? {
        Some(value) => Ok(value as f32),
        None if matches!(mode, IniParseMode::Lenient) => Ok(default),
        None => Err(AmiximaError::Parser(format!(
            "{section} is missing required parameter {key}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_effect_type() -> Result<()> {
        let path = std::env::temp_dir().join(format!("unknown_{}.ini", uuid::Uuid::new_v4()));
        std::fs::write(&path, "[effect1]\ntype=warp\n")?;
        let result = parse_ini(&path, IniParseMode::Strict);
        std::fs::remove_file(path)?;
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn rejects_missing_strict_parameter() -> Result<()> {
        let path = std::env::temp_dir().join(format!("missing_{}.ini", uuid::Uuid::new_v4()));
        std::fs::write(&path, "[effect1]\ntype=eq\nfrequency=1000\n")?;
        let result = parse_ini(&path, IniParseMode::Strict);
        std::fs::remove_file(path)?;
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn preserves_effect_section_order() -> Result<()> {
        let path = std::env::temp_dir().join(format!("ordered_{}.ini", uuid::Uuid::new_v4()));
        std::fs::write(
            &path,
            "[effect2]\ntype=gain\ngain_db=1\n[effect1]\ntype=delay\ndelay_ms=10\nfeedback=0.2\n",
        )?;
        let sc = parse_ini(&path, IniParseMode::Strict)?;
        std::fs::remove_file(path)?;
        assert!(matches!(sc.sequence[0], EffectNode::Delay { .. }));
        assert!(matches!(sc.sequence[1], EffectNode::Gain { .. }));
        Ok(())
    }
}
