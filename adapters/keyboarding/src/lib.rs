use core::{GameAdapter, OptionField, OptionFieldKind, SelectChoice};

const WORDS: &[&str] = &[
    "adventure",
    "airplane",
    "alligator",
    "alphabet",
    "amazing",
    "anchor",
    "animal",
    "antelope",
    "apple",
    "astronaut",
    "avalanche",
    "awesome",
    "backpack",
    "balloon",
    "bamboo",
    "banana",
    "barbecue",
    "baseball",
    "basket",
    "beach",
    "bicycle",
    "blanket",
    "blizzard",
    "bonfire",
    "boomerang",
    "breakfast",
    "breeze",
    "bridge",
    "bubble",
    "buffalo",
    "butterfly",
    "cactus",
    "calendar",
    "campfire",
    "canyon",
    "captain",
    "carnival",
    "castle",
    "catapult",
    "champion",
    "chandelier",
    "chapter",
    "cheetah",
    "cherry",
    "chimney",
    "chocolate",
    "cinnamon",
    "circus",
    "climbing",
    "coconut",
    "comet",
    "compass",
    "cookie",
    "cosmic",
    "cottage",
    "cougar",
    "country",
    "crayon",
    "crystal",
    "cupcake",
    "curtain",
    "cyclone",
    "dancing",
    "daylight",
    "desert",
    "diamond",
    "dinosaur",
    "dolphin",
    "donkey",
    "dragon",
    "dragonfly",
    "dream",
    "drumstick",
    "dumplings",
    "eagle",
    "eclipse",
    "elephant",
    "elevator",
    "emerald",
    "enchanted",
    "engine",
    "envelope",
    "espresso",
    "evening",
    "evergreen",
    "falcon",
    "fantastic",
    "feather",
    "festival",
    "firefly",
    "fireworks",
    "flamingo",
    "flashlight",
    "flower",
    "football",
    "forest",
    "fountain",
    "foxhound",
    "freedom",
    "frostbite",
    "galaxy",
    "garden",
    "gazelle",
    "geyser",
    "giraffe",
    "glacier",
    "glitter",
    "goldfish",
    "gorilla",
    "grasshopper",
    "gravity",
    "grizzly",
    "guitar",
    "gumball",
    "hammock",
    "hamster",
    "handshake",
    "happiness",
    "harbor",
    "harvest",
    "hedgehog",
    "helmet",
    "highway",
    "hilltop",
    "homework",
    "honeybee",
    "horizon",
    "horseshoe",
    "hospital",
    "hummingbird",
    "hurricane",
    "iceberg",
    "igloo",
    "imagine",
    "island",
    "jackrabbit",
    "jaguar",
    "jasmine",
    "jelly",
    "journey",
    "juggler",
    "jungle",
    "kangaroo",
    "kayak",
    "kettle",
    "kingdom",
    "kitchen",
    "kitten",
    "kiwi",
    "koala",
    "ladder",
    "lantern",
    "laughter",
    "lemonade",
    "leopard",
    "library",
    "lightning",
    "lizard",
    "lobster",
    "luggage",
    "mammoth",
    "mango",
    "mansion",
    "maple",
    "marathon",
    "meadow",
    "mermaid",
    "meteor",
    "midnight",
    "mitten",
    "monarch",
    "monkey",
    "monster",
    "moonlight",
    "mountain",
    "mushroom",
    "mustang",
    "napkin",
    "narwhal",
    "nectar",
    "noodle",
    "nugget",
    "nutmeg",
    "ocean",
    "octopus",
    "orange",
    "orchard",
    "ostrich",
    "outdoor",
    "overlook",
    "owl",
    "pajamas",
    "palm",
    "pancake",
    "panther",
    "parachute",
    "parrot",
    "passport",
    "peacock",
    "peanut",
    "pelican",
    "penguin",
    "pepperoni",
    "phoenix",
    "picnic",
    "pilgrim",
    "pineapple",
    "pirate",
    "pizza",
    "planet",
    "platypus",
    "popcorn",
    "porcupine",
    "pretzel",
    "pudding",
    "pumpkin",
    "puppet",
    "puzzle",
    "pyramid",
    "quarter",
    "quicksand",
    "rabbit",
    "raccoon",
    "rainbow",
    "rainstorm",
    "raspberry",
    "reindeer",
    "reptile",
    "ribbon",
    "river",
    "roadrunner",
    "robot",
    "rocket",
    "rooftop",
    "saddle",
    "sailboat",
    "salmon",
    "sandwich",
    "sapphire",
    "scarecrow",
    "scooter",
    "seagull",
    "seahorse",
    "shadow",
    "shamrock",
    "shelter",
    "shipwreck",
    "silver",
    "skeleton",
    "skyline",
    "sleigh",
    "slingshot",
    "smoothie",
    "snowflake",
    "sparkle",
    "spider",
    "spinach",
    "squirrel",
    "stadium",
    "stampede",
    "starfish",
    "stingray",
    "strawberry",
    "submarine",
    "sunflower",
    "sunrise",
    "sunshine",
    "surfboard",
    "sushi",
    "swallow",
    "tadpole",
    "tambourine",
    "tangerine",
    "telescope",
    "termite",
    "thunder",
    "tiger",
    "tomato",
    "tornado",
    "toucan",
    "tower",
    "treasure",
    "treehouse",
    "triangle",
    "tricycle",
    "tropical",
    "trumpet",
    "tulip",
    "tunnel",
    "turtle",
    "umbrella",
    "unicorn",
    "universe",
    "vacation",
    "valley",
    "vanilla",
    "velvet",
    "village",
    "violet",
    "volcano",
    "vulture",
    "waffle",
    "wagon",
    "walrus",
    "wanderer",
    "warrior",
    "waterfall",
    "whale",
    "whistle",
    "wildfire",
    "windmill",
    "winter",
    "wizard",
    "wonder",
    "woodpecker",
    "xylophone",
    "yellow",
    "yogurt",
    "zebra",
    "zeppelin",
    "zigzag",
];

#[derive(Debug, Clone, Default)]
pub struct KeyboardingAdapter;

/// Transforms an English word into its leetspeak form using the same
/// substitution table the client uses to generate the `l33t` UI catalog. Applied
/// to the word list, this makes the prompt the player must type leet
/// (e.g. "adventure" -> "4dv3n7ur3"). Output is typeable ASCII, matched verbatim
/// by `is_correct`.
fn leetify(word: &str) -> String {
    word.chars()
        .map(|c| match c.to_ascii_lowercase() {
            'a' => '4',
            'e' => '3',
            'i' => '1',
            'o' => '0',
            's' => '5',
            't' => '7',
            _ => c,
        })
        .collect()
}

impl GameAdapter for KeyboardingAdapter {
    fn game_key(&self) -> &'static str {
        "keyboarding"
    }

    fn game_label(&self) -> &'static str {
        "Keyboarding"
    }

    fn option_schema(&self) -> Vec<OptionField> {
        // Host-selected content language: which word set the player must type.
        // This rides the existing `game_options` plumbing — no protocol change.
        vec![OptionField {
            key: "wordSet".to_string(),
            label: "Word Set".to_string(),
            kind: OptionFieldKind::Select {
                choices: vec![
                    SelectChoice {
                        value: "english".to_string(),
                        label: "English".to_string(),
                    },
                    SelectChoice {
                        value: "l33t".to_string(),
                        label: "L33T".to_string(),
                    },
                ],
                default: "english".to_string(),
            },
            visible_when: None,
        }]
    }

    fn next_prompt(&self, seed: u64, options: &serde_json::Value) -> String {
        let idx = (seed as usize) % WORDS.len();
        let word = WORDS[idx];
        match options.get("wordSet").and_then(|v| v.as_str()) {
            Some("l33t") => leetify(word),
            _ => word.to_string(),
        }
    }

    fn is_correct(&self, prompt: &str, attempt: &str) -> bool {
        prompt == attempt.trim()
    }

    fn normalize_progress(&self, raw_input: &str) -> String {
        raw_input.to_string()
    }

    fn score_for_prompt(&self, prompt: &str) -> f32 {
        (prompt.len() as f32 / 3.0).max(4.0)
    }

    fn input_placeholder(&self) -> &'static str {
        "Type the word; press return."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_exact_word_match() {
        let adapter = KeyboardingAdapter;
        assert!(adapter.is_correct("rust", "rust"));
        assert!(!adapter.is_correct("rust", "Rust"));
    }

    #[test]
    fn option_schema_exposes_word_set() {
        let adapter = KeyboardingAdapter;
        let schema = adapter.option_schema();
        assert_eq!(schema.len(), 1);
        assert_eq!(schema[0].key, "wordSet");
        match &schema[0].kind {
            OptionFieldKind::Select { choices, default } => {
                assert_eq!(default, "english");
                let values: Vec<&str> = choices.iter().map(|c| c.value.as_str()).collect();
                assert_eq!(values, vec!["english", "l33t"]);
            }
            _ => panic!("expected a select option"),
        }
    }

    #[test]
    fn default_word_set_is_plain_english() {
        let adapter = KeyboardingAdapter;
        let prompt = adapter.next_prompt(0, &serde_json::Value::Null);
        assert!(WORDS.contains(&prompt.as_str()));
    }

    #[test]
    fn l33t_word_set_leetifies_the_prompt() {
        let adapter = KeyboardingAdapter;
        let options = serde_json::json!({ "wordSet": "l33t" });
        let plain = adapter.next_prompt(0, &serde_json::Value::Null);
        let leet = adapter.next_prompt(0, &options);
        // Same seed -> same underlying word, leetified.
        assert_eq!(leet, leetify(&plain));
        // The leetified word is exactly what the player must type.
        assert!(adapter.is_correct(&leet, &leet));
    }

    #[test]
    fn leetify_output_is_typeable_ascii() {
        for word in WORDS {
            for ch in leetify(word).chars() {
                assert!(
                    ch.is_ascii_alphanumeric(),
                    "non-typeable char {ch:?} in {word}"
                );
            }
        }
    }

    #[test]
    fn leetify_maps_known_letters() {
        assert_eq!(leetify("adventure"), "4dv3n7ur3");
        assert_eq!(leetify("aeiost"), "431057");
    }
}
