use core::{GameAdapter, OptionField, OptionFieldKind, SelectChoice, VisibleWhen};

#[derive(Debug, Clone, Default)]
pub struct ArithmeticAdapter;

fn splitmix64(val: u64) -> u64 {
    let mut z = val.wrapping_add(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

fn rand_in_range(seed: u64, salt: u64, lo: i64, hi: i64) -> i64 {
    let mixed = splitmix64(seed.wrapping_add(salt.wrapping_mul(0x517cc1b727220a95)));
    let range = (hi - lo + 1) as u64;
    lo + (mixed % range) as i64
}

fn generate_term(seed: u64, term_salt: u64, min_digits: u32, max_digits: u32) -> i64 {
    let digit_count =
        rand_in_range(seed, term_salt * 2, min_digits as i64, max_digits as i64) as u32;
    let lo = if digit_count <= 1 {
        1
    } else {
        10_i64.pow(digit_count - 1)
    };
    let hi = 10_i64.pow(digit_count) - 1;
    rand_in_range(seed, term_salt * 2 + 1, lo, hi)
}

fn parse_str_option<'a>(options: &'a serde_json::Value, key: &str, default: &'a str) -> &'a str {
    options.get(key).and_then(|v| v.as_str()).unwrap_or(default)
}

fn parse_u32_option(options: &serde_json::Value, key: &str, default: u32) -> u32 {
    options
        .get(key)
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(default)
}

fn parse_bool_option(options: &serde_json::Value, key: &str, default: bool) -> bool {
    options
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s == "true")
        .unwrap_or(default)
}

struct TermDigits {
    first_min: u32,
    first_max: u32,
    second_min: u32,
    second_max: u32,
}

fn parse_term_digits(options: &serde_json::Value) -> TermDigits {
    let mut first_min = parse_u32_option(options, "firstTermMinimumDigits", 1).clamp(1, 6);
    let mut first_max = parse_u32_option(options, "firstTermMaximumDigits", 1).clamp(1, 6);
    let mut second_min = parse_u32_option(options, "secondTermMinimumDigits", 1).clamp(1, 6);
    let mut second_max = parse_u32_option(options, "secondTermMaximumDigits", 1).clamp(1, 6);

    if first_min > first_max {
        std::mem::swap(&mut first_min, &mut first_max);
    }
    if second_min > second_max {
        std::mem::swap(&mut second_min, &mut second_max);
    }

    TermDigits {
        first_min,
        first_max,
        second_min,
        second_max,
    }
}

fn make_range_field(key: &str, label: &str, default: i32) -> OptionField {
    OptionField {
        key: key.to_string(),
        label: label.to_string(),
        kind: OptionFieldKind::Range {
            min: 1,
            max: 6,
            step: 1,
            default,
        },
        visible_when: None,
    }
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
                        SelectChoice {
                            value: "multiplication".to_string(),
                            label: "Multiplication".to_string(),
                        },
                        SelectChoice {
                            value: "division".to_string(),
                            label: "Division".to_string(),
                        },
                    ],
                    default: "addition".to_string(),
                },
                visible_when: None,
            },
            make_range_field("firstTermMinimumDigits", "Minimum Digits in First Term", 1),
            make_range_field("firstTermMaximumDigits", "Maximum Digits in First Term", 1),
            make_range_field(
                "secondTermMinimumDigits",
                "Minimum Digits in Second Term",
                1,
            ),
            make_range_field(
                "secondTermMaximumDigits",
                "Maximum Digits in Second Term",
                1,
            ),
            OptionField {
                key: "allowNegativeAnswers".to_string(),
                label: "Allow Negative Answers".to_string(),
                kind: OptionFieldKind::Toggle { default: false },
                visible_when: Some(VisibleWhen {
                    key: "operation".to_string(),
                    value: "subtraction".to_string(),
                }),
            },
        ]
    }

    fn next_prompt(&self, seed: u64, options: &serde_json::Value) -> String {
        let op = parse_str_option(options, "operation", "addition");
        let digits = parse_term_digits(options);
        let allow_negative = parse_bool_option(options, "allowNegativeAnswers", false);

        match op {
            "subtraction" => {
                let mut t1 = generate_term(seed, 0, digits.first_min, digits.first_max);
                let mut t2 = generate_term(seed, 1, digits.second_min, digits.second_max);
                if !allow_negative && t1 < t2 {
                    std::mem::swap(&mut t1, &mut t2);
                }
                format!("{t1} - {t2}")
            }
            "multiplication" => {
                let t1 = generate_term(seed, 0, digits.first_min, digits.first_max);
                let t2 = generate_term(seed, 1, digits.second_min, digits.second_max);
                format!("{t1} \u{00d7} {t2}")
            }
            "division" => generate_division_prompt(seed, &digits),
            _ => {
                let t1 = generate_term(seed, 0, digits.first_min, digits.first_max);
                let t2 = generate_term(seed, 1, digits.second_min, digits.second_max);
                format!("{t1} + {t2}")
            }
        }
    }

    fn is_correct(&self, prompt: &str, attempt: &str) -> bool {
        let expected = eval_prompt(prompt);
        match attempt.trim().parse::<i64>() {
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

    fn input_mode(&self) -> &'static str {
        "decimal"
    }
}

fn generate_division_prompt(seed: u64, digits: &TermDigits) -> String {
    let t1 = generate_term(seed, 0, digits.first_min, digits.first_max);
    let second_lo = if digits.second_min <= 1 {
        1_i64
    } else {
        10_i64.pow(digits.second_min - 1)
    };
    let second_hi = 10_i64.pow(digits.second_max) - 1;

    let potential_divisors: Vec<i64> = (second_lo..=second_hi).filter(|&d| t1 % d == 0).collect();

    if !potential_divisors.is_empty() {
        let idx = splitmix64(seed.wrapping_add(2)) as usize % potential_divisors.len();
        let t2 = potential_divisors[idx];
        format!("{t1} \u{00f7} {t2}")
    } else {
        let t2 = generate_term(seed, 1, digits.second_min, digits.second_max);
        let quotient = generate_term(seed, 2, digits.first_min, digits.first_max);
        let dividend = t2 * quotient;
        format!("{dividend} \u{00f7} {t2}")
    }
}

fn eval_prompt(prompt: &str) -> Option<i64> {
    if let Some((l, r)) = prompt.split_once('\u{00d7}') {
        let left = l.trim().parse::<i64>().ok()?;
        let right = r.trim().parse::<i64>().ok()?;
        Some(left * right)
    } else if let Some((l, r)) = prompt.split_once('\u{00f7}') {
        let left = l.trim().parse::<i64>().ok()?;
        let right = r.trim().parse::<i64>().ok()?;
        Some(left / right)
    } else if let Some((l, r)) = prompt.split_once('+') {
        let left = l.trim().parse::<i64>().ok()?;
        let right = r.trim().parse::<i64>().ok()?;
        Some(left + right)
    } else if let Some((l, r)) = prompt.split_once('-') {
        let left = l.trim().parse::<i64>().ok()?;
        let right = r.trim().parse::<i64>().ok()?;
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
    fn validates_multiplication() {
        let adapter = ArithmeticAdapter;
        assert!(adapter.is_correct("7 \u{00d7} 6", "42"));
        assert!(!adapter.is_correct("7 \u{00d7} 6", "13"));
    }

    #[test]
    fn validates_division() {
        let adapter = ArithmeticAdapter;
        assert!(adapter.is_correct("42 \u{00f7} 7", "6"));
        assert!(!adapter.is_correct("42 \u{00f7} 7", "7"));
    }

    #[test]
    fn default_options_generate_addition() {
        let adapter = ArithmeticAdapter;
        let prompt = adapter.next_prompt(42, &serde_json::Value::Null);
        assert!(prompt.contains('+'), "expected + in: {prompt}");
    }

    #[test]
    fn subtraction_option_generates_subtraction() {
        let adapter = ArithmeticAdapter;
        let opts = serde_json::json!({"operation": "subtraction"});
        let prompt = adapter.next_prompt(42, &opts);
        assert!(prompt.contains('-'), "expected - in: {prompt}");
    }

    #[test]
    fn multiplication_option_generates_multiplication() {
        let adapter = ArithmeticAdapter;
        let opts = serde_json::json!({"operation": "multiplication"});
        let prompt = adapter.next_prompt(42, &opts);
        assert!(
            prompt.contains('\u{00d7}'),
            "expected \u{00d7} in: {prompt}"
        );
    }

    #[test]
    fn division_option_generates_division() {
        let adapter = ArithmeticAdapter;
        let opts = serde_json::json!({"operation": "division"});
        for seed in 0..50 {
            let prompt = adapter.next_prompt(seed, &opts);
            assert!(
                prompt.contains('\u{00f7}'),
                "expected \u{00f7} in: {prompt}"
            );
        }
    }

    #[test]
    fn subtraction_non_negative_by_default() {
        let adapter = ArithmeticAdapter;
        let opts = serde_json::json!({"operation": "subtraction"});
        for seed in 0..200 {
            let prompt = adapter.next_prompt(seed, &opts);
            let result = eval_prompt(&prompt).expect("valid prompt");
            assert!(
                result >= 0,
                "subtraction result should be non-negative: {prompt} = {result}"
            );
        }
    }

    #[test]
    fn subtraction_allows_negative_when_toggled() {
        let adapter = ArithmeticAdapter;
        let opts = serde_json::json!({
            "operation": "subtraction",
            "allowNegativeAnswers": "true",
            "firstTermMinimumDigits": "1",
            "firstTermMaximumDigits": "1",
            "secondTermMinimumDigits": "1",
            "secondTermMaximumDigits": "1",
        });
        let mut found_negative = false;
        for seed in 0..500 {
            let prompt = adapter.next_prompt(seed, &opts);
            let result = eval_prompt(&prompt).expect("valid prompt");
            if result < 0 {
                found_negative = true;
                break;
            }
        }
        assert!(
            found_negative,
            "expected at least one negative result with allowNegativeAnswers"
        );
    }

    #[test]
    fn division_always_produces_integer_result() {
        let adapter = ArithmeticAdapter;
        let opts = serde_json::json!({"operation": "division"});
        for seed in 0..200 {
            let prompt = adapter.next_prompt(seed, &opts);
            let (l, r) = prompt.split_once('\u{00f7}').expect("division prompt");
            let left: i64 = l.trim().parse().unwrap();
            let right: i64 = r.trim().parse().unwrap();
            assert!(
                right != 0 && left % right == 0,
                "division should produce integer: {prompt}"
            );
        }
    }

    #[test]
    fn two_digit_operands_in_range() {
        let adapter = ArithmeticAdapter;
        let opts = serde_json::json!({
            "firstTermMinimumDigits": "2",
            "firstTermMaximumDigits": "2",
            "secondTermMinimumDigits": "2",
            "secondTermMaximumDigits": "2",
        });
        for seed in 0..50 {
            let prompt = adapter.next_prompt(seed, &opts);
            let parts: Vec<&str> = prompt.split('+').collect();
            assert_eq!(parts.len(), 2);
            let left: i64 = parts[0].trim().parse().unwrap();
            let right: i64 = parts[1].trim().parse().unwrap();
            assert!((10..=99).contains(&left), "left {left} should be 2-digit");
            assert!(
                (10..=99).contains(&right),
                "right {right} should be 2-digit"
            );
        }
    }

    #[test]
    fn mixed_digit_ranges() {
        let adapter = ArithmeticAdapter;
        let opts = serde_json::json!({
            "operation": "multiplication",
            "firstTermMinimumDigits": "2",
            "firstTermMaximumDigits": "3",
            "secondTermMinimumDigits": "1",
            "secondTermMaximumDigits": "1",
        });
        for seed in 0..50 {
            let prompt = adapter.next_prompt(seed, &opts);
            let (l, r) = prompt
                .split_once('\u{00d7}')
                .expect("multiplication prompt");
            let left: i64 = l.trim().parse().unwrap();
            let right: i64 = r.trim().parse().unwrap();
            assert!(
                (10..=999).contains(&left),
                "left {left} should be 2-3 digits"
            );
            assert!((1..=9).contains(&right), "right {right} should be 1 digit");
        }
    }

    #[test]
    fn three_digit_subtraction() {
        let adapter = ArithmeticAdapter;
        let opts = serde_json::json!({
            "operation": "subtraction",
            "firstTermMinimumDigits": "3",
            "firstTermMaximumDigits": "3",
            "secondTermMinimumDigits": "3",
            "secondTermMaximumDigits": "3",
        });
        for seed in 0..50 {
            let prompt = adapter.next_prompt(seed, &opts);
            assert!(prompt.contains('-'));
            let result = eval_prompt(&prompt).expect("valid prompt");
            assert!(result >= 0);
        }
    }

    #[test]
    fn min_max_swap_on_invalid_input() {
        let adapter = ArithmeticAdapter;
        let opts = serde_json::json!({
            "firstTermMinimumDigits": "3",
            "firstTermMaximumDigits": "1",
        });
        let prompt = adapter.next_prompt(42, &opts);
        let parts: Vec<&str> = prompt.split('+').collect();
        let left: i64 = parts[0].trim().parse().unwrap();
        assert!(
            (1..=999).contains(&left),
            "swapped min/max should still produce valid operand: {left}"
        );
    }

    #[test]
    fn option_schema_has_expected_keys() {
        let adapter = ArithmeticAdapter;
        let schema = adapter.option_schema();
        assert_eq!(schema.len(), 6);
        assert_eq!(schema[0].key, "operation");
        assert_eq!(schema[1].key, "firstTermMinimumDigits");
        assert_eq!(schema[2].key, "firstTermMaximumDigits");
        assert_eq!(schema[3].key, "secondTermMinimumDigits");
        assert_eq!(schema[4].key, "secondTermMaximumDigits");
        assert_eq!(schema[5].key, "allowNegativeAnswers");
        assert!(schema[5].visible_when.is_some());
    }
}
