use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Soundcourse {
    #[serde(rename = "@context")]
    pub context: serde_json::Value,
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@type")]
    pub rdf_type: String,
    #[serde(rename = "dc:title", skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "dc:description", skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "mo:sampleRate", skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<f32>,
    pub creator: String,
    pub sequence: Vec<EffectNode>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "aufx:type", content = "aufx:parameters")]
pub enum EffectNode {
    #[serde(rename = "aufx:Reverb")]
    Reverb {
        #[serde(rename = "aufx:roomSize")]
        room_size: f32,
        #[serde(rename = "aufx:dryWet")]
        dry_wet: f32,
    },
    #[serde(rename = "aufx:EQ")]
    EQ {
        #[serde(rename = "aufx:frequency")]
        frequency: f32,
        #[serde(rename = "aufx:gain")]
        gain: f32,
    },
    #[serde(rename = "aufx:Delay")]
    Delay {
        #[serde(rename = "aufx:delayMs")]
        delay_ms: f32,
        #[serde(rename = "aufx:feedback")]
        feedback: f32,
    },
    #[serde(rename = "aufx:Compressor")]
    Compressor {
        #[serde(rename = "aufx:threshold")]
        threshold: f32,
        #[serde(rename = "aufx:ratio")]
        ratio: f32,
    },
    #[serde(rename = "aufx:Gain")]
    Gain {
        #[serde(rename = "aufx:gainDb")]
        gain_db: f32,
    },
}

impl Soundcourse {
    pub fn new(creator: &str) -> Self {
        Self {
            context: Self::default_context(),
            id: format!("amixima:sc:{}", uuid::Uuid::new_v4()),
            rdf_type: "mo:Workflow".to_string(),
            title: None,
            description: None,
            sample_rate: Some(44100.0),
            creator: creator.to_string(),
            sequence: Vec::new(),
        }
    }

    pub fn to_json_ld(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn default_context() -> serde_json::Value {
        serde_json::json!({
            "mo": "http://purl.org/ontology/mo/",
            "aufx": "http://purl.org/ontology/aufx-o/",
            "dc": "http://purl.org/dc/elements/1.1/",
            "xsd": "http://www.w3.org/2001/XMLSchema#",
            "Soundcourse": "mo:Workflow",
            "sequence": {
                "@id": "mo:workflow_step",
                "@container": "@list"
            },
            "mo:sampleRate": {
                "@type": "xsd:float"
            }
        })
    }
}
