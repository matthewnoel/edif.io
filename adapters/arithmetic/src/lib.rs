use core::{GameAdapter, OptionField, OptionFieldKind, SelectChoice};

#[derive(Debug, Clone, Default)]
pub struct ArithmeticAdapter;

fn parse_operation(options: &serde_json::Value) -> &str {
    options
        .get("operation")
        .and_then(|v| v.as_str())
        .unwrap_or("addition")
}

fn parse_digits(options: &serde_json::Value) -> u32 {
    options
        .get("digits")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1)
}

fn operand_range(digits: u32) -> (i32, i32) {
    if digits <= 1 {
        (1, 9)
    } else {
        let lo = 10_i32.pow(digits - 1);
        let hi = 10_i32.pow(digits) - 1;
        (lo, hi)
    }
}

fn operand_from_seed(seed_part: u64, lo: i32, hi: i32) -> i32 {
    let range = (hi - lo + 1) as u64;
    lo + (seed_part % range) as i32
}

impl GameAdapter for ArithmeticAdapter {
    fn game_key(&self) -> &'static str {
        "arithmetic"
    }

    fn game_label(&self) -> &'static str {
        "Arithmetic"
    }

    fn option_schema(&self) -> Vec<OptionField> {
        vec![
            OptionField {
                key: "operation".to_string(),
                label: "Operation".to_string(),
                kind: OptionFieldKind::Select {
                    choices: vec![
                        SelectChoice {
                            value: "addition".to_string(),
                            label: "Addition".to_string(),
                        },
                        SelectChoice {
                            value: "subtraction".to_string(),
                            label: "Subtraction".to_string(),
                        },
                    ],
                    default: "addition".to_string(),
                },
            },
            OptionField {
                key: "digits".to_string(),
                label: "Number Size".to_string(),
                kind: OptionFieldKind::Select {
                    choices: vec![
                        SelectChoice {
                            value: "1".to_string(),
                            label: "1 Digit (1-9)".to_string(),
                        },
                        SelectChoice {
                            value: "2".to_string(),
                            label: "2 Digits (10-99)".to_string(),
                        },
                        SelectChoice {
                            value: "3".to_string(),
                            label: "3 Digits (100-999)".to_string(),
                        },
                    ],
                    default: "1".to_string(),
                },
            },
        ]
    }

    fn next_prompt(&self, seed: u64, options: &serde_json::Value) -> String {
        let op = parse_operation(options);
        let digits = parse_digits(options);
        let (lo, hi) = operand_range(digits);

        let mut left = operand_from_seed(seed, lo, hi);
        let mut right = operand_from_seed(seed / 7, lo, hi);

        if op == "subtraction" && left < right {
            std::mem::swap(&mut left, &mut right);
        }

        let symbol = if op == "subtraction" { "-" } else { "+" };
        format!("{left} {symbol} {right}")
    }

    fn is_correct(&self, prompt: &str, attempt: &str) -> bool {
        let expected = eval_prompt(prompt);
        match attempt.trim().parse::<i32>() {
            Ok(value) => expected == Some(value),
            Err(_) => false,
        }
    }

    fn normalize_progress(&self, raw_input: &str) -> String {
        raw_input.trim().to_string()
    }

    fn score_for_prompt(&self, _prompt: &str) -> f32 {
        5.0
    }

    fn input_placeholder(&self) -> &'static str {
        "Enter the solution; press return."
    }
}

fn eval_prompt(prompt: &str) -> Option<i32> {
    if let Some((left_str, right_str)) = prompt.split_once('+') {
        let left = left_str.trim().parse::<i32>().ok()?;
        let right = right_str.trim().parse::<i32>().ok()?;
        Some(left + right)
    } else if let Some((left_str, right_str)) = prompt.split_once('-') {
        let left = left_str.trim().parse::<i32>().ok()?;
        let right = right_str.trim().parse::<i32>().ok()?;
        Some(left - right)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_addition() {
        let adapter = ArithmeticAdapter;
        assert!(adapter.is_correct("2 + 9", "11"));
        assert!(!adapter.is_correct("2 + 9", "12"));
    }

    #[test]
    fn validates_subtraction() {
        let adapter = ArithmeticAdapter;
        assert!(adapter.is_correct("9 - 2", "7"));
        assert!(!adapter.is_correct("9 - 2", "11"));
    }

    #[test]
    fn default_options_generate_addition() {
        let adapter = ArithmeticAdapter;
        let prompt = adapter.next_prompt(42, &serde_json::Value::Null);
        assert!(prompt.contains('+'));
    }

    #[test]
    fn subtraction_option_generates_subtraction() {
        let adapter = ArithmeticAdapter;
        let opts = serde_json::json!({"operation": "subtraction"});
        let prompt = adapter.next_prompt(42, &opts);
        assert!(prompt.contains('-'), "prompt should contain minus: {prompt}");
    }

    #[test]
    fn subtraction_left_is_gte_right() {
        let adapter = ArithmeticAdapter;
        let opts = serde_json::json!({"operation": "subtraction"});
        for seed in 0..200 {
            let prompt = adapter.next_prompt(seed, &opts);
            let result = eval_prompt(&prompt).expect("valid prompt");
            assert!(result >= 0, "subtraction result should be non-negative: {prompt} = {result}");
        }
    }

    #[test]
    fn two_digit_option_generates_two_digit_operands() {
        let adapter = ArithmeticAdapter;
        let opts = serde_json::json!({"digits": "2"});
        for seed in 0..50 {
            let prompt = adapter.next_prompt(seed, &opts);
            let parts: Vec<&str> = prompt.split('+').collect();
            assert_eq!(parts.len(), 2);
            let left: i32 = parts[0].trim().parse().unwrap();
            let right: i32 = parts[1].trim().parse().unwrap();
            assert!((10..=99).contains(&left), "left {left} should be 2-digit");
            assert!((10..=99).contains(&right), "right {right} should be 2-digit");
        }
    }

    #[test]
    fn three_digit_subtraction() {
        let adapter = ArithmeticAdapter;
        let opts = serde_json::json!({"operation": "subtraction", "digits": "3"});
        for seed in 0..50 {
            let prompt = adapter.next_prompt(seed, &opts);
            assert!(prompt.contains('-'));
            let result = eval_prompt(&prompt).expect("valid prompt");
            assert!(result >= 0);
        }
    }

    #[test]
    fn option_schema_has_expected_keys() {
        let adapter = ArithmeticAdapter;
        let schema = adapter.option_schema();
        assert_eq!(schema.len(), 2);
        assert_eq!(schema[0].key, "operation");
        assert_eq!(schema[1].key, "digits");
    }
}
