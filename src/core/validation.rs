use crate::core::soundcourse::{EffectNode, Soundcourse};
use crate::error::{AmiximaError, Result};

#[derive(Debug, Clone, Copy)]
pub enum ValidationMode {
    Strict,
    UiFriendly,
}

#[derive(Debug, Default, Clone)]
pub struct ValidationReport {
    pub warnings: Vec<String>,
}

pub fn validate_soundcourse(
    soundcourse: &Soundcourse,
    mode: ValidationMode,
) -> Result<ValidationReport> {
    let mut report = ValidationReport::default();

    if matches!(mode, ValidationMode::Strict) && soundcourse.is_empty() {
        return Err(AmiximaError::Validation(
            "Soundcourse has no effects; add at least one effect before processing".to_string(),
        ));
    }

    if let Some(sample_rate) = soundcourse.sample_rate {
        if !(8_000.0..=192_000.0).contains(&sample_rate) {
            return Err(AmiximaError::Validation(format!(
                "sample rate {sample_rate} is outside 8000..=192000 Hz"
            )));
        }
    } else if matches!(mode, ValidationMode::UiFriendly) {
        report
            .warnings
            .push("sample rate is missing; processing will use decoded input rate".to_string());
    }

    for (index, effect) in soundcourse.sequence.iter().enumerate() {
        validate_effect(index, effect)?;
    }

    Ok(report)
}

fn validate_effect(index: usize, effect: &EffectNode) -> Result<()> {
    match effect {
        EffectNode::Gain { gain_db } => {
            check_range(index, "Gain", "gain_db", *gain_db, -24.0, 24.0)
        }
        EffectNode::EQ { frequency, gain } => {
            check_range(index, "EQ", "frequency", *frequency, 20.0, 20_000.0)?;
            check_range(index, "EQ", "gain", *gain, -24.0, 24.0)
        }
        EffectNode::Delay { delay_ms, feedback } => {
            check_range(index, "Delay", "delay_ms", *delay_ms, 0.0, 2_000.0)?;
            check_range(index, "Delay", "feedback", *feedback, 0.0, 0.95)
        }
        EffectNode::Reverb { room_size, dry_wet } => {
            check_range(index, "Reverb", "room_size", *room_size, 0.0, 1.0)?;
            check_range(index, "Reverb", "dry_wet", *dry_wet, 0.0, 1.0)
        }
        EffectNode::Compressor { threshold, ratio } => {
            check_range(index, "Compressor", "threshold", *threshold, -60.0, 0.0)?;
            check_range(index, "Compressor", "ratio", *ratio, 1.0, 20.0)
        }
    }
}

fn check_range(
    index: usize,
    effect: &str,
    parameter: &str,
    value: f32,
    min: f32,
    max: f32,
) -> Result<()> {
    if value.is_finite() && (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(AmiximaError::Validation(format!(
            "effect {} ({effect}) parameter {parameter}={value} is outside {min}..={max}",
            index + 1
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_of_each() -> Soundcourse {
        let mut soundcourse = Soundcourse::new("Tester");
        soundcourse.sequence.push(EffectNode::Gain { gain_db: 0.0 });
        soundcourse.sequence.push(EffectNode::EQ {
            frequency: 1_000.0,
            gain: 0.0,
        });
        soundcourse.sequence.push(EffectNode::Delay {
            delay_ms: 100.0,
            feedback: 0.5,
        });
        soundcourse.sequence.push(EffectNode::Reverb {
            room_size: 0.5,
            dry_wet: 0.25,
        });
        soundcourse.sequence.push(EffectNode::Compressor {
            threshold: -18.0,
            ratio: 4.0,
        });
        soundcourse
    }

    #[test]
    fn accepts_valid_soundcourse() {
        assert!(validate_soundcourse(&one_of_each(), ValidationMode::Strict).is_ok());
    }

    #[test]
    fn rejects_empty_strict_soundcourse() {
        let soundcourse = Soundcourse::new("Tester");
        assert!(validate_soundcourse(&soundcourse, ValidationMode::Strict).is_err());
    }

    #[test]
    fn rejects_invalid_delay_feedback() {
        let mut soundcourse = Soundcourse::new("Tester");
        soundcourse.sequence.push(EffectNode::Delay {
            delay_ms: 100.0,
            feedback: 1.0,
        });
        assert!(validate_soundcourse(&soundcourse, ValidationMode::Strict).is_err());
    }
}
