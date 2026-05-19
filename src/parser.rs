use crate::ontology::{EffectNode, Soundcourse};
use color_eyre::Result;
use configparser::ini::Ini;

pub struct SoundcourseParser;

impl SoundcourseParser {
    pub fn parse_ini(path: &str) -> Result<Soundcourse> {
        let mut config = Ini::new();
        config.load(path).map_err(|e| color_eyre::eyre::eyre!(e))?;

        // --- Metadata ---
        let title = config.get("metadata", "title");
        let description = config.get("metadata", "description");
        let sample_rate = config
            .getfloat("metadata", "sample_rate")
            .map_err(|e| color_eyre::eyre::eyre!(e))?
            .map(|v| v as f32);

        // --- Effects sequence ---
        let mut sequence = Vec::new();
        let mut sections = config.sections();
        sections.sort_by_key(|s| {
            s.strip_prefix("effect")
                .and_then(|n| n.parse::<u32>().ok())
                .unwrap_or(0)
        });

        for section in sections {
            if let Some(effect_type) = config.get(&section, "type") {
                match effect_type.to_lowercase().as_str() {
                    "reverb" => {
                        let room_size = config
                            .getfloat(&section, "room_size")
                            .map_err(|e| color_eyre::eyre::eyre!(e))?
                            .unwrap_or(0.5) as f32;
                        let dry_wet = config
                            .getfloat(&section, "dry_wet")
                            .map_err(|e| color_eyre::eyre::eyre!(e))?
                            .unwrap_or(0.3) as f32;
                        sequence.push(EffectNode::Reverb { room_size, dry_wet });
                    }
                    "eq" => {
                        let frequency = config
                            .getfloat(&section, "frequency")
                            .map_err(|e| color_eyre::eyre::eyre!(e))?
                            .unwrap_or(1000.0) as f32;
                        let gain = config
                            .getfloat(&section, "gain")
                            .map_err(|e| color_eyre::eyre::eyre!(e))?
                            .unwrap_or(0.0) as f32;
                        sequence.push(EffectNode::EQ { frequency, gain });
                    }
                    "delay" => {
                        let delay_ms = config
                            .getfloat(&section, "delay_ms")
                            .map_err(|e| color_eyre::eyre::eyre!(e))?
                            .unwrap_or(100.0) as f32;
                        let feedback = config
                            .getfloat(&section, "feedback")
                            .map_err(|e| color_eyre::eyre::eyre!(e))?
                            .unwrap_or(0.5) as f32;
                        sequence.push(EffectNode::Delay { delay_ms, feedback });
                    }
                    "compressor" => {
                        let threshold = config
                            .getfloat(&section, "threshold")
                            .map_err(|e| color_eyre::eyre::eyre!(e))?
                            .unwrap_or(-20.0) as f32;
                        let ratio = config
                            .getfloat(&section, "ratio")
                            .map_err(|e| color_eyre::eyre::eyre!(e))?
                            .unwrap_or(4.0) as f32;
                        sequence.push(EffectNode::Compressor { threshold, ratio });
                    }
                    "gain" => {
                        let gain_db = config
                            .getfloat(&section, "gain_db")
                            .map_err(|e| color_eyre::eyre::eyre!(e))?
                            .unwrap_or(0.0) as f32;
                        sequence.push(EffectNode::Gain { gain_db });
                    }
                    _ => {}
                }
            }
        }

        let mut sc = Soundcourse::new("DefaultUser");
        if let Some(t) = title {
            sc.title = Some(t);
        }
        if let Some(d) = description {
            sc.description = Some(d);
        }
        if let Some(sr) = sample_rate {
            sc.sample_rate = Some(sr);
        }
        sc.sequence = sequence;
        Ok(sc)
    }

    pub fn serialize_to_ini(soundcourse: &Soundcourse, path: &str) -> Result<()> {
        let mut config = Ini::new();

        // --- Metadata ---
        config.set("metadata", "title", soundcourse.title.clone());
        if let Some(desc) = &soundcourse.description {
            config.set("metadata", "description", Some(desc.clone()));
        }
        if let Some(sr) = soundcourse.sample_rate {
            config.set("metadata", "sample_rate", Some(sr.to_string()));
        }

        // --- Effects ---
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
        config.write(path).map_err(|e| color_eyre::eyre::eyre!(e))?;
        Ok(())
    }
}

// --- Tests remain unchanged (omitted for brevity) ---
// (Insert the original test code here – it still works)

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

    let path = "test_serialize.ini";
    SoundcourseParser::serialize_to_ini(&sc, path)?;

    let sc2 = SoundcourseParser::parse_ini(path)?;
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
