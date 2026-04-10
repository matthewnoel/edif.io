use core::{GameAdapter, OptionField, OptionFieldKind, SelectChoice};

const STATES: &[(&str, &str)] = &[
    ("Alabama", "AL"),
    ("Alaska", "AK"),
    ("Arizona", "AZ"),
    ("Arkansas", "AR"),
    ("California", "CA"),
    ("Colorado", "CO"),
    ("Connecticut", "CT"),
    ("Delaware", "DE"),
    ("Florida", "FL"),
    ("Georgia", "GA"),
    ("Hawaii", "HI"),
    ("Idaho", "ID"),
    ("Illinois", "IL"),
    ("Indiana", "IN"),
    ("Iowa", "IA"),
    ("Kansas", "KS"),
    ("Kentucky", "KY"),
    ("Louisiana", "LA"),
    ("Maine", "ME"),
    ("Maryland", "MD"),
    ("Massachusetts", "MA"),
    ("Michigan", "MI"),
    ("Minnesota", "MN"),
    ("Mississippi", "MS"),
    ("Missouri", "MO"),
    ("Montana", "MT"),
    ("Nebraska", "NE"),
    ("Nevada", "NV"),
    ("New Hampshire", "NH"),
    ("New Jersey", "NJ"),
    ("New Mexico", "NM"),
    ("New York", "NY"),
    ("North Carolina", "NC"),
    ("North Dakota", "ND"),
    ("Ohio", "OH"),
    ("Oklahoma", "OK"),
    ("Oregon", "OR"),
    ("Pennsylvania", "PA"),
    ("Rhode Island", "RI"),
    ("South Carolina", "SC"),
    ("South Dakota", "SD"),
    ("Tennessee", "TN"),
    ("Texas", "TX"),
    ("Utah", "UT"),
    ("Vermont", "VT"),
    ("Virginia", "VA"),
    ("Washington", "WA"),
    ("West Virginia", "WV"),
    ("Wisconsin", "WI"),
    ("Wyoming", "WY"),
];

#[derive(Debug, Clone, Default)]
pub struct StateAbbreviationsAdapter;

fn parse_str_option<'a>(options: &'a serde_json::Value, key: &str, default: &'a str) -> &'a str {
    options.get(key).and_then(|v| v.as_str()).unwrap_or(default)
}

fn splitmix64(val: u64) -> u64 {
    let mut z = val.wrapping_add(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

fn lookup_abbr_for_name(name: &str) -> Option<&'static str> {
    STATES
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, a)| *a)
}

fn lookup_name_for_abbr(abbr: &str) -> Option<&'static str> {
    STATES
        .iter()
        .find(|(_, a)| a.eq_ignore_ascii_case(abbr))
        .map(|(n, _)| *n)
}

impl GameAdapter for StateAbbreviationsAdapter {
    fn game_key(&self) -> &'static str {
        "state-abbreviations"
    }

    fn game_label(&self) -> &'static str {
        "US State Abbreviations"
    }

    fn option_schema(&self) -> Vec<OptionField> {
        vec![OptionField {
            key: "direction".to_string(),
            label: "Prompt Direction".to_string(),
            kind: OptionFieldKind::Select {
                choices: vec![
                    SelectChoice {
                        value: "nameToAbbr".to_string(),
                        label: "State Name \u{2192} Abbreviation".to_string(),
                    },
                    SelectChoice {
                        value: "abbrToName".to_string(),
                        label: "Abbreviation \u{2192} State Name".to_string(),
                    },
                    SelectChoice {
                        value: "both".to_string(),
                        label: "Both Directions".to_string(),
                    },
                ],
                default: "nameToAbbr".to_string(),
            },
            visible_when: None,
        }]
    }

    fn next_prompt(&self, seed: u64, options: &serde_json::Value) -> String {
        let direction = parse_str_option(options, "direction", "nameToAbbr");
        let idx = (seed as usize) % STATES.len();
        let (name, abbr) = STATES[idx];

        match direction {
            "abbrToName" => abbr.to_string(),
            "both" => {
                if splitmix64(seed.wrapping_add(1)) & 1 == 0 {
                    name.to_string()
                } else {
                    abbr.to_string()
                }
            }
            _ => name.to_string(),
        }
    }

    fn is_correct(&self, prompt: &str, attempt: &str) -> bool {
        let answer = attempt.trim();
        if let Some(expected) = lookup_abbr_for_name(prompt) {
            return expected.eq_ignore_ascii_case(answer);
        }
        if let Some(expected) = lookup_name_for_abbr(prompt) {
            return expected.eq_ignore_ascii_case(answer);
        }
        false
    }

    fn normalize_progress(&self, raw_input: &str) -> String {
        raw_input.trim().to_string()
    }

    fn score_for_prompt(&self, _prompt: &str) -> f32 {
        5.0
    }

    fn input_placeholder(&self) -> &'static str {
        "Type the state name or abbreviation; press return."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_fifty_states_present() {
        assert_eq!(STATES.len(), 50);
    }

    #[test]
    fn every_abbreviation_is_two_uppercase_letters() {
        for (_, abbr) in STATES {
            assert_eq!(abbr.len(), 2, "abbreviation {abbr} should be 2 chars");
            assert!(
                abbr.chars().all(|c| c.is_ascii_uppercase()),
                "abbreviation {abbr} should be uppercase ASCII"
            );
        }
    }

    #[test]
    fn is_correct_accepts_exact_match() {
        let adapter = StateAbbreviationsAdapter;
        assert!(adapter.is_correct("California", "CA"));
        assert!(adapter.is_correct("CA", "California"));
    }

    #[test]
    fn is_correct_is_case_insensitive() {
        let adapter = StateAbbreviationsAdapter;
        assert!(adapter.is_correct("California", "ca"));
        assert!(adapter.is_correct("California", "Ca"));
        assert!(adapter.is_correct("california", "CA"));
        assert!(adapter.is_correct("CA", "california"));
        assert!(adapter.is_correct("ca", "CALIFORNIA"));
        assert!(adapter.is_correct("New York", "ny"));
        assert!(adapter.is_correct("ny", "new york"));
    }

    #[test]
    fn is_correct_trims_whitespace() {
        let adapter = StateAbbreviationsAdapter;
        assert!(adapter.is_correct("Texas", "  TX  "));
        assert!(adapter.is_correct("TX", "  Texas  "));
    }

    #[test]
    fn is_correct_rejects_wrong_answer() {
        let adapter = StateAbbreviationsAdapter;
        assert!(!adapter.is_correct("California", "TX"));
        assert!(!adapter.is_correct("CA", "Texas"));
        assert!(!adapter.is_correct("California", "Cali"));
        assert!(!adapter.is_correct("CA", ""));
    }

    #[test]
    fn is_correct_rejects_unknown_prompt() {
        let adapter = StateAbbreviationsAdapter;
        assert!(!adapter.is_correct("Atlantis", "AT"));
    }

    #[test]
    fn name_to_abbr_prompts_are_state_names() {
        let adapter = StateAbbreviationsAdapter;
        let opts = serde_json::json!({"direction": "nameToAbbr"});
        for seed in 0..100u64 {
            let prompt = adapter.next_prompt(seed, &opts);
            assert!(
                STATES.iter().any(|(n, _)| *n == prompt),
                "prompt {prompt} should be a state name"
            );
        }
    }

    #[test]
    fn default_direction_is_name_to_abbr() {
        let adapter = StateAbbreviationsAdapter;
        for seed in 0..100u64 {
            let prompt = adapter.next_prompt(seed, &serde_json::Value::Null);
            assert!(
                STATES.iter().any(|(n, _)| *n == prompt),
                "default prompt {prompt} should be a state name"
            );
        }
    }

    #[test]
    fn abbr_to_name_prompts_are_abbreviations() {
        let adapter = StateAbbreviationsAdapter;
        let opts = serde_json::json!({"direction": "abbrToName"});
        for seed in 0..100u64 {
            let prompt = adapter.next_prompt(seed, &opts);
            assert!(
                STATES.iter().any(|(_, a)| *a == prompt),
                "prompt {prompt} should be a state abbreviation"
            );
        }
    }

    #[test]
    fn both_direction_produces_mix() {
        let adapter = StateAbbreviationsAdapter;
        let opts = serde_json::json!({"direction": "both"});
        let mut saw_name = false;
        let mut saw_abbr = false;
        for seed in 0..500u64 {
            let prompt = adapter.next_prompt(seed, &opts);
            if STATES.iter().any(|(n, _)| *n == prompt) {
                saw_name = true;
            } else if STATES.iter().any(|(_, a)| *a == prompt) {
                saw_abbr = true;
            } else {
                panic!("prompt {prompt} is neither a state name nor an abbreviation");
            }
            if saw_name && saw_abbr {
                break;
            }
        }
        assert!(saw_name, "expected at least one state-name prompt");
        assert!(saw_abbr, "expected at least one abbreviation prompt");
    }

    #[test]
    fn both_direction_answers_match_prompt_type() {
        let adapter = StateAbbreviationsAdapter;
        let opts = serde_json::json!({"direction": "both"});
        for seed in 0..200u64 {
            let prompt = adapter.next_prompt(seed, &opts);
            if let Some((_, abbr)) = STATES.iter().find(|(n, _)| *n == prompt) {
                assert!(adapter.is_correct(&prompt, abbr));
            } else if let Some((name, _)) = STATES.iter().find(|(_, a)| *a == prompt) {
                assert!(adapter.is_correct(&prompt, name));
            } else {
                panic!("unexpected prompt: {prompt}");
            }
        }
    }

    #[test]
    fn deterministic_for_same_seed() {
        let adapter = StateAbbreviationsAdapter;
        let opts = serde_json::json!({"direction": "both"});
        for seed in 0..50u64 {
            let a = adapter.next_prompt(seed, &opts);
            let b = adapter.next_prompt(seed, &opts);
            assert_eq!(a, b);
        }
    }

    #[test]
    fn option_schema_has_direction_select() {
        let adapter = StateAbbreviationsAdapter;
        let schema = adapter.option_schema();
        assert_eq!(schema.len(), 1);
        assert_eq!(schema[0].key, "direction");
        match &schema[0].kind {
            OptionFieldKind::Select { choices, default } => {
                assert_eq!(default, "nameToAbbr");
                assert_eq!(choices.len(), 3);
                let values: Vec<&str> = choices.iter().map(|c| c.value.as_str()).collect();
                assert!(values.contains(&"nameToAbbr"));
                assert!(values.contains(&"abbrToName"));
                assert!(values.contains(&"both"));
            }
            _ => panic!("expected Select option for direction"),
        }
    }
}
