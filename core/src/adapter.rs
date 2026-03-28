use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionField {
    pub key: String,
    pub label: String,
    #[serde(flatten)]
    pub kind: OptionFieldKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible_when: Option<VisibleWhen>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum OptionFieldKind {
    Select {
        choices: Vec<SelectChoice>,
        default: String,
    },
    Range {
        min: i32,
        max: i32,
        step: i32,
        default: i32,
    },
    Toggle {
        default: bool,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectChoice {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibleWhen {
    pub key: String,
    pub value: String,
}

pub trait GameAdapter: Send + Sync + 'static {
    fn game_key(&self) -> &'static str;
    fn game_label(&self) -> &'static str {
        self.game_key()
    }
    fn option_schema(&self) -> Vec<OptionField> {
        vec![]
    }
    fn next_prompt(&self, seed: u64, options: &serde_json::Value) -> String;
    fn is_correct(&self, prompt: &str, attempt: &str) -> bool;
    fn normalize_progress(&self, raw_input: &str) -> String;
    fn score_for_prompt(&self, prompt: &str) -> f32;
    fn input_placeholder(&self) -> &'static str {
        "Type your answer; press return."
    }
}

pub type AdapterHandle = Arc<dyn GameAdapter>;
pub type AdapterRegistry = HashMap<String, AdapterHandle>;

pub fn build_adapter_registry(adapters: Vec<AdapterHandle>) -> Result<AdapterRegistry, String> {
    if adapters.is_empty() {
        return Err("at least one adapter must be registered".to_string());
    }

    let mut registry = HashMap::new();
    for adapter in adapters {
        let game_key = adapter.game_key().to_string();
        if registry.insert(game_key.clone(), adapter).is_some() {
            return Err(format!("duplicate adapter game key: {game_key}"));
        }
    }
    Ok(registry)
}
