use crate::game::{PlayerId, RoomSnapshot};
use crate::powerup::PowerUpKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    RoomNotFound,
    InvalidGameMode,
    InvalidMessageFormat,
    InvalidRejoinToken,
    RoomExpired,
    PlayerNotInRoom,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ClientMessage {
    JoinOrCreateRoom {
        #[serde(rename = "roomCode")]
        room_code: Option<String>,
        #[serde(rename = "gameMode")]
        game_mode: Option<String>,
        #[serde(rename = "matchDurationSecs")]
        match_duration_secs: Option<u64>,
        #[serde(rename = "gameOptions")]
        game_options: Option<serde_json::Value>,
    },
    RejoinRoom {
        #[serde(rename = "rejoinToken")]
        rejoin_token: String,
    },
    InputUpdate {
        text: String,
    },
    SubmitAttempt {
        text: String,
    },
    StartMatch,
    Rematch,
    UpdateRoomSettings {
        #[serde(rename = "gameMode")]
        game_mode: Option<String>,
        #[serde(rename = "matchDurationSecs")]
        match_duration_secs: Option<u64>,
        #[serde(rename = "gameOptions")]
        game_options: Option<serde_json::Value>,
    },
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ServerMessage {
    Welcome {
        #[serde(rename = "playerId")]
        player_id: PlayerId,
        #[serde(rename = "roomCode")]
        room_code: String,
        #[serde(rename = "gameKey")]
        game_key: String,
        #[serde(rename = "inputPlaceholder")]
        input_placeholder: String,
        #[serde(rename = "inputMode")]
        input_mode: String,
        #[serde(rename = "rejoinToken")]
        rejoin_token: String,
    },
    RoomState {
        room: RoomSnapshot,
    },
    PromptState {
        #[serde(rename = "roomCode")]
        room_code: String,
        #[serde(rename = "playerId")]
        player_id: PlayerId,
        #[serde(rename = "roundId")]
        round_id: u64,
        prompt: String,
    },
    RaceProgress {
        #[serde(rename = "roomCode")]
        room_code: String,
        #[serde(rename = "playerId")]
        player_id: PlayerId,
        text: String,
    },
    RoundResult {
        #[serde(rename = "roomCode")]
        room_code: String,
        #[serde(rename = "roundId")]
        round_id: u64,
        #[serde(rename = "winnerPlayerId")]
        winner_player_id: PlayerId,
        #[serde(rename = "growthAwarded")]
        growth_awarded: f32,
    },
    WrongAnswer {
        #[serde(rename = "roomCode")]
        room_code: String,
        #[serde(rename = "playerId")]
        player_id: PlayerId,
        #[serde(rename = "shrinkApplied")]
        shrink_applied: f32,
    },
    Error {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<ErrorCode>,
    },
    PowerUpOffered {
        #[serde(rename = "offerId")]
        offer_id: u64,
        #[serde(rename = "playerId")]
        player_id: PlayerId,
        kind: PowerUpKind,
        #[serde(rename = "expiresInMs")]
        expires_in_ms: u64,
    },
    PowerUpActivated {
        #[serde(rename = "offerId")]
        offer_id: u64,
        #[serde(rename = "playerId")]
        player_id: PlayerId,
        kind: PowerUpKind,
        #[serde(rename = "durationMs")]
        duration_ms: u64,
    },
    PowerUpOfferExpired {
        #[serde(rename = "offerId")]
        offer_id: u64,
        #[serde(rename = "playerId")]
        player_id: PlayerId,
        kind: PowerUpKind,
    },
    PowerUpEffectEnded {
        #[serde(rename = "playerId")]
        player_id: PlayerId,
        kind: PowerUpKind,
    },
}

#[cfg(test)]
mod tests {
    use super::ClientMessage;

    #[test]
    fn parses_all_supported_client_messages() {
        let join = r#"{"type":"joinOrCreateRoom","roomCode":"ABCD","gameMode":"keyboarding","matchDurationSecs":90,"gameOptions":{"operation":"addition"}}"#;
        assert!(serde_json::from_str::<ClientMessage>(join).is_ok());

        let rejoin = r#"{"type":"rejoinRoom","rejoinToken":"abc123"}"#;
        assert!(serde_json::from_str::<ClientMessage>(rejoin).is_ok());

        let update = r#"{"type":"inputUpdate","text":"hel"}"#;
        assert!(serde_json::from_str::<ClientMessage>(update).is_ok());

        let submit = r#"{"type":"submitAttempt","text":"hello"}"#;
        assert!(serde_json::from_str::<ClientMessage>(submit).is_ok());

        let start = r#"{"type":"startMatch"}"#;
        assert!(serde_json::from_str::<ClientMessage>(start).is_ok());

        let rematch = r#"{"type":"rematch"}"#;
        assert!(serde_json::from_str::<ClientMessage>(rematch).is_ok());

        let update = r#"{"type":"updateRoomSettings","gameMode":"arithmetic","matchDurationSecs":90,"gameOptions":{"operation":"addition"}}"#;
        assert!(serde_json::from_str::<ClientMessage>(update).is_ok());

        let update_minimal = r#"{"type":"updateRoomSettings"}"#;
        assert!(serde_json::from_str::<ClientMessage>(update_minimal).is_ok());
    }

    #[test]
    fn rejects_removed_ping_message() {
        let ping = r#"{"type":"ping","sentAtMs":123}"#;
        assert!(serde_json::from_str::<ClientMessage>(ping).is_err());
    }

    #[test]
    fn serializes_wrong_answer_message() {
        let msg = super::ServerMessage::WrongAnswer {
            room_code: "ABCD".to_string(),
            player_id: 1,
            shrink_applied: 2.0,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"wrongAnswer""#));
        assert!(json.contains(r#""roomCode":"ABCD""#));
        assert!(json.contains(r#""playerId":1"#));
        assert!(json.contains(r#""shrinkApplied":2.0"#));
    }

    #[test]
    fn serializes_powerup_offered() {
        let msg = super::ServerMessage::PowerUpOffered {
            offer_id: 5,
            player_id: 2,
            kind: super::PowerUpKind::DoublePoints,
            expires_in_ms: 30000,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"powerUpOffered""#));
        assert!(json.contains(r#""offerId":5"#));
        assert!(json.contains(r#""playerId":2"#));
        assert!(json.contains(r#""kind":"doublePoints""#));
        assert!(json.contains(r#""expiresInMs":30000"#));
    }

    #[test]
    fn serializes_powerup_activated() {
        let msg = super::ServerMessage::PowerUpActivated {
            offer_id: 3,
            player_id: 2,
            kind: super::PowerUpKind::DoublePoints,
            duration_ms: 30000,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"powerUpActivated""#));
        assert!(json.contains(r#""offerId":3"#));
        assert!(json.contains(r#""playerId":2"#));
        assert!(json.contains(r#""kind":"doublePoints""#));
        assert!(json.contains(r#""durationMs":30000"#));
    }

    #[test]
    fn serializes_powerup_offer_expired() {
        let msg = super::ServerMessage::PowerUpOfferExpired {
            offer_id: 7,
            player_id: 3,
            kind: super::PowerUpKind::DoublePoints,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"powerUpOfferExpired""#));
        assert!(json.contains(r#""offerId":7"#));
        assert!(json.contains(r#""playerId":3"#));
        assert!(json.contains(r#""kind":"doublePoints""#));
    }

    #[test]
    fn serializes_powerup_effect_ended() {
        let msg = super::ServerMessage::PowerUpEffectEnded {
            player_id: 1,
            kind: super::PowerUpKind::DoublePoints,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"powerUpEffectEnded""#));
        assert!(json.contains(r#""playerId":1"#));
        assert!(json.contains(r#""kind":"doublePoints""#));
    }

    #[test]
    fn serializes_error_with_code() {
        let msg = super::ServerMessage::Error {
            message: "No room found with code ZZZZ".to_string(),
            code: Some(super::ErrorCode::RoomNotFound),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"error""#));
        assert!(json.contains(r#""message":"No room found with code ZZZZ""#));
        assert!(json.contains(r#""code":"roomNotFound""#));
    }

    #[test]
    fn serializes_error_without_code() {
        let msg = super::ServerMessage::Error {
            message: "Something unexpected".to_string(),
            code: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"error""#));
        assert!(json.contains(r#""message":"Something unexpected""#));
        assert!(!json.contains("code"));
    }

    // --- Wire-format snapshot tests -------------------------------------------------
    //
    // These tests pin the exact JSON shape emitted and accepted by the server.
    // The client parses these payloads in `client/src/lib/game/protocol.ts`,
    // so a dependency bump (e.g. serde, serde_json) must NEVER silently change
    // field names, enum tag casing, or required fields.
    //
    // Comparisons use `serde_json::Value` equality so field order doesn't matter,
    // but key names and types absolutely do.

    use super::{ErrorCode, ServerMessage};
    use crate::game::{PlayerSnapshot, RoomSnapshot};
    use crate::powerup::{ActivePowerUpSnapshot, PowerUpKind};
    use serde_json::{json, to_value};

    fn to_json(value: &impl serde::Serialize) -> serde_json::Value {
        to_value(value).expect("serialize")
    }

    #[test]
    fn wire_welcome_shape() {
        let msg = ServerMessage::Welcome {
            player_id: 4,
            room_code: "ABCD".to_string(),
            game_key: "keyboarding".to_string(),
            input_placeholder: "Type here...".to_string(),
            input_mode: "text".to_string(),
            rejoin_token: "abc123".to_string(),
        };
        assert_eq!(
            to_json(&msg),
            json!({
                "type": "welcome",
                "playerId": 4,
                "roomCode": "ABCD",
                "gameKey": "keyboarding",
                "inputPlaceholder": "Type here...",
                "inputMode": "text",
                "rejoinToken": "abc123",
            })
        );
    }

    #[test]
    fn wire_room_state_shape() {
        let room = RoomSnapshot {
            room_code: "ABCD".to_string(),
            players: vec![PlayerSnapshot {
                id: 1,
                name: "Alice".to_string(),
                size: 14.5,
                color: "#38bdf8".to_string(),
                connected: true,
                progress: "he".to_string(),
            }],
            match_winner: None,
            match_remaining_ms: Some(45000),
            host_player_id: 1,
            active_powerups: vec![ActivePowerUpSnapshot {
                kind: PowerUpKind::DoublePoints,
                source_player_id: 2,
                remaining_ms: 20000,
                duration_ms: 30000,
            }],
            game_key: "keyboarding".to_string(),
            game_options: json!({ "operation": "addition" }),
            match_duration_secs: 60,
            input_mode: "text".to_string(),
            input_placeholder: "Type here...".to_string(),
        };
        let msg = ServerMessage::RoomState { room };
        assert_eq!(
            to_json(&msg),
            json!({
                "type": "roomState",
                "room": {
                    "roomCode": "ABCD",
                    "players": [{
                        "id": 1,
                        "name": "Alice",
                        "size": 14.5,
                        "color": "#38bdf8",
                        "connected": true,
                        "progress": "he",
                    }],
                    "matchWinner": null,
                    "matchRemainingMs": 45000,
                    "hostPlayerId": 1,
                    "activePowerups": [{
                        "kind": "doublePoints",
                        "sourcePlayerId": 2,
                        "remainingMs": 20000,
                        "durationMs": 30000,
                    }],
                    "gameKey": "keyboarding",
                    "gameOptions": { "operation": "addition" },
                    "matchDurationSecs": 60,
                    "inputMode": "text",
                    "inputPlaceholder": "Type here...",
                },
            })
        );
    }

    #[test]
    fn wire_prompt_state_shape() {
        let msg = ServerMessage::PromptState {
            room_code: "ABCD".to_string(),
            player_id: 1,
            round_id: 2,
            prompt: "hello".to_string(),
        };
        assert_eq!(
            to_json(&msg),
            json!({
                "type": "promptState",
                "roomCode": "ABCD",
                "playerId": 1,
                "roundId": 2,
                "prompt": "hello",
            })
        );
    }

    #[test]
    fn wire_race_progress_shape() {
        let msg = ServerMessage::RaceProgress {
            room_code: "ABCD".to_string(),
            player_id: 1,
            text: "hel".to_string(),
        };
        assert_eq!(
            to_json(&msg),
            json!({
                "type": "raceProgress",
                "roomCode": "ABCD",
                "playerId": 1,
                "text": "hel",
            })
        );
    }

    #[test]
    fn wire_round_result_shape() {
        let msg = ServerMessage::RoundResult {
            room_code: "ABCD".to_string(),
            round_id: 2,
            winner_player_id: 1,
            growth_awarded: 4.0,
        };
        assert_eq!(
            to_json(&msg),
            json!({
                "type": "roundResult",
                "roomCode": "ABCD",
                "roundId": 2,
                "winnerPlayerId": 1,
                "growthAwarded": 4.0,
            })
        );
    }

    #[test]
    fn wire_wrong_answer_shape() {
        let msg = ServerMessage::WrongAnswer {
            room_code: "ABCD".to_string(),
            player_id: 1,
            shrink_applied: 2.0,
        };
        assert_eq!(
            to_json(&msg),
            json!({
                "type": "wrongAnswer",
                "roomCode": "ABCD",
                "playerId": 1,
                "shrinkApplied": 2.0,
            })
        );
    }

    #[test]
    fn wire_error_with_code_shape() {
        let msg = ServerMessage::Error {
            message: "No room found".to_string(),
            code: Some(ErrorCode::RoomNotFound),
        };
        assert_eq!(
            to_json(&msg),
            json!({
                "type": "error",
                "message": "No room found",
                "code": "roomNotFound",
            })
        );
    }

    #[test]
    fn wire_error_without_code_omits_field() {
        let msg = ServerMessage::Error {
            message: "Boom".to_string(),
            code: None,
        };
        assert_eq!(
            to_json(&msg),
            json!({
                "type": "error",
                "message": "Boom",
            })
        );
    }

    #[test]
    fn wire_error_codes_serialize_as_camel_case() {
        let cases = [
            (ErrorCode::RoomNotFound, "roomNotFound"),
            (ErrorCode::InvalidGameMode, "invalidGameMode"),
            (ErrorCode::InvalidMessageFormat, "invalidMessageFormat"),
            (ErrorCode::InvalidRejoinToken, "invalidRejoinToken"),
            (ErrorCode::RoomExpired, "roomExpired"),
            (ErrorCode::PlayerNotInRoom, "playerNotInRoom"),
        ];
        for (code, expected) in cases {
            assert_eq!(to_json(&code), serde_json::Value::String(expected.into()));
        }
    }

    #[test]
    fn wire_powerup_kinds_serialize_as_camel_case() {
        let cases = [
            (PowerUpKind::DoublePoints, "doublePoints"),
            (PowerUpKind::ScrambleFont, "scrambleFont"),
            (PowerUpKind::ScoreSteal, "scoreSteal"),
            (PowerUpKind::OngoingScoreSteal, "ongoingScoreSteal"),
        ];
        for (kind, expected) in cases {
            assert_eq!(to_json(&kind), serde_json::Value::String(expected.into()));
        }
    }

    #[test]
    fn wire_powerup_offered_shape() {
        let msg = ServerMessage::PowerUpOffered {
            offer_id: 5,
            player_id: 2,
            kind: PowerUpKind::ScrambleFont,
            expires_in_ms: 30000,
        };
        assert_eq!(
            to_json(&msg),
            json!({
                "type": "powerUpOffered",
                "offerId": 5,
                "playerId": 2,
                "kind": "scrambleFont",
                "expiresInMs": 30000,
            })
        );
    }

    #[test]
    fn wire_powerup_activated_shape() {
        let msg = ServerMessage::PowerUpActivated {
            offer_id: 3,
            player_id: 2,
            kind: PowerUpKind::DoublePoints,
            duration_ms: 30000,
        };
        assert_eq!(
            to_json(&msg),
            json!({
                "type": "powerUpActivated",
                "offerId": 3,
                "playerId": 2,
                "kind": "doublePoints",
                "durationMs": 30000,
            })
        );
    }

    #[test]
    fn wire_powerup_offer_expired_shape() {
        let msg = ServerMessage::PowerUpOfferExpired {
            offer_id: 7,
            player_id: 3,
            kind: PowerUpKind::DoublePoints,
        };
        assert_eq!(
            to_json(&msg),
            json!({
                "type": "powerUpOfferExpired",
                "offerId": 7,
                "playerId": 3,
                "kind": "doublePoints",
            })
        );
    }

    #[test]
    fn wire_powerup_effect_ended_shape() {
        let msg = ServerMessage::PowerUpEffectEnded {
            player_id: 1,
            kind: PowerUpKind::ScrambleFont,
        };
        assert_eq!(
            to_json(&msg),
            json!({
                "type": "powerUpEffectEnded",
                "playerId": 1,
                "kind": "scrambleFont",
            })
        );
    }

    #[test]
    fn wire_client_join_or_create_accepts_full_payload() {
        let raw = json!({
            "type": "joinOrCreateRoom",
            "roomCode": "ABCD",
            "gameMode": "keyboarding",
            "matchDurationSecs": 90,
            "gameOptions": { "operation": "addition" },
        })
        .to_string();
        let parsed: ClientMessage = serde_json::from_str(&raw).expect("parse");
        let ClientMessage::JoinOrCreateRoom {
            room_code,
            game_mode,
            match_duration_secs,
            game_options,
        } = parsed
        else {
            panic!("expected JoinOrCreateRoom");
        };
        assert_eq!(room_code.as_deref(), Some("ABCD"));
        assert_eq!(game_mode.as_deref(), Some("keyboarding"));
        assert_eq!(match_duration_secs, Some(90));
        assert_eq!(game_options, Some(json!({ "operation": "addition" })));
    }

    #[test]
    fn wire_client_join_or_create_accepts_minimal_payload() {
        // All fields of joinOrCreateRoom other than `type` are optional.
        let raw = r#"{"type":"joinOrCreateRoom"}"#;
        let parsed: ClientMessage = serde_json::from_str(raw).expect("parse");
        assert!(matches!(parsed, ClientMessage::JoinOrCreateRoom { .. }));
    }

    #[test]
    fn wire_client_message_tag_casing_is_camel_case() {
        // The tag field must be camelCase to match the client's TypeScript union.
        let cases = [
            (r#"{"type":"rejoinRoom","rejoinToken":"t"}"#, true),
            (r#"{"type":"inputUpdate","text":"a"}"#, true),
            (r#"{"type":"submitAttempt","text":"a"}"#, true),
            (r#"{"type":"startMatch"}"#, true),
            (r#"{"type":"rematch"}"#, true),
            (r#"{"type":"updateRoomSettings"}"#, true),
            // snake_case tags must NOT be accepted.
            (r#"{"type":"rejoin_room","rejoinToken":"t"}"#, false),
            (r#"{"type":"start_match"}"#, false),
            (r#"{"type":"update_room_settings"}"#, false),
        ];
        for (raw, should_parse) in cases {
            let result = serde_json::from_str::<ClientMessage>(raw);
            assert_eq!(
                result.is_ok(),
                should_parse,
                "payload {raw} parse-ok expectation violated"
            );
        }
    }
}
