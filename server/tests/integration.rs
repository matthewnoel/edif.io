//! Full-stack integration tests for the server binary.
//!
//! These tests drive the real axum app over a real TCP socket — HTTP for the
//! REST endpoints and WebSocket for the game protocol — so any breakage in the
//! `axum` / `tokio` / `futures-util` / `serde` / `tokio-tungstenite` layer that
//! a dependency bump might introduce shows up here. They are intentionally
//! end-to-end so that a green build gives confidence to auto-merge dependency
//! version bumps.

use edif_core::{ServerConfig, build_app};
use edif_io_arithmetic_adapter::ArithmeticAdapter;
use edif_io_keyboarding_adapter::KeyboardingAdapter;
use edif_io_state_abbreviations_adapter::StateAbbreviationsAdapter;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

struct TestServer {
    addr: SocketAddr,
    _handle: tokio::task::JoinHandle<()>,
}

async fn start_server() -> TestServer {
    let config = ServerConfig {
        bind_addr: "127.0.0.1:0".to_string(),
        growth_per_round_win: 4.0,
        shrink_per_wrong_answer: 2.0,
        match_duration_secs: 60,
    };
    let app = build_app(
        vec![
            Arc::new(KeyboardingAdapter),
            Arc::new(ArithmeticAdapter),
            Arc::new(StateAbbreviationsAdapter),
        ],
        config,
    )
    .expect("build app");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local_addr");

    let handle = tokio::spawn(async move {
        // If this errors the test task panics; axum::serve returns only on shutdown.
        axum::serve(listener, app).await.expect("axum serve");
    });

    TestServer {
        addr,
        _handle: handle,
    }
}

async fn http_get(addr: SocketAddr, path: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(addr).await.expect("tcp connect");
    let req = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).await.expect("write req");
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.expect("read resp");
    let text = String::from_utf8(raw).expect("utf8 response");
    let (head, body) = text.split_once("\r\n\r\n").expect("http response split");
    let status_line = head.lines().next().expect("status line");
    // HTTP/1.1 200 OK
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("status code");
    // Body may be chunked; we request Connection: close so axum uses content-length.
    // If axum responds with chunked for some reason, peel one chunk.
    let body = if head
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        dechunk(body)
    } else {
        body.to_string()
    };
    (status, body)
}

fn dechunk(raw: &str) -> String {
    let mut out = String::new();
    let mut rest = raw;
    while let Some((size_line, after)) = rest.split_once("\r\n") {
        let size = usize::from_str_radix(size_line.trim(), 16).unwrap_or(0);
        if size == 0 {
            break;
        }
        let (chunk, after_chunk) = after.split_at(size);
        out.push_str(chunk);
        rest = after_chunk.strip_prefix("\r\n").unwrap_or(after_chunk);
    }
    out
}

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn connect_ws(addr: SocketAddr) -> Ws {
    let url = format!("ws://{addr}/ws");
    let (ws, _resp) = connect_async(&url).await.expect("ws connect");
    ws
}

async fn send_json(ws: &mut Ws, value: Value) {
    ws.send(Message::Text(value.to_string().into()))
        .await
        .expect("ws send");
}

/// Wait for the next text-frame JSON message, ignoring pings/pongs and binary frames.
async fn recv_json(ws: &mut Ws) -> Value {
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(5), ws.next())
            .await
            .expect("ws recv timeout")
            .expect("ws stream ended")
            .expect("ws recv error");
        match msg {
            Message::Text(t) => {
                return serde_json::from_str::<Value>(&t).expect("parse json");
            }
            Message::Ping(_) | Message::Pong(_) => continue,
            Message::Close(_) => panic!("ws closed unexpectedly"),
            other => panic!("unexpected ws frame: {other:?}"),
        }
    }
}

/// Wait until a message of a specific `type` field arrives, returning the first match.
/// Any other messages are consumed silently.
async fn recv_type(ws: &mut Ws, type_: &str) -> Value {
    loop {
        let v = recv_json(ws).await;
        if v.get("type").and_then(Value::as_str) == Some(type_) {
            return v;
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn healthz_returns_ok() {
    let server = start_server().await;
    let (status, body) = http_get(server.addr, "/healthz").await;
    assert_eq!(status, 200);
    assert_eq!(body, "ok");
}

#[tokio::test(flavor = "multi_thread")]
async fn readyz_returns_ok() {
    let server = start_server().await;
    let (status, body) = http_get(server.addr, "/readyz").await;
    assert_eq!(status, 200);
    assert_eq!(body, "ok");
}

#[tokio::test(flavor = "multi_thread")]
async fn game_modes_lists_all_adapters() {
    let server = start_server().await;
    let (status, body) = http_get(server.addr, "/api/game-modes").await;
    assert_eq!(status, 200);
    let modes: Value = serde_json::from_str(&body).expect("json body");
    let arr = modes.as_array().expect("array body");
    let keys: Vec<&str> = arr
        .iter()
        .map(|m| m.get("key").and_then(Value::as_str).unwrap_or(""))
        .collect();
    assert_eq!(
        keys,
        vec!["keyboarding", "arithmetic", "state-abbreviations"],
        "game mode ordering must be stable for the client"
    );

    // Spot-check each mode exposes an options array and a label.
    for mode in arr {
        assert!(mode.get("label").and_then(Value::as_str).is_some());
        assert!(mode.get("options").and_then(Value::as_array).is_some());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn ws_join_rejects_invalid_json() {
    let server = start_server().await;
    let mut ws = connect_ws(server.addr).await;

    ws.send(Message::Text("not-json".into()))
        .await
        .expect("send garbage");
    let err = recv_type(&mut ws, "error").await;
    assert_eq!(
        err.get("code").and_then(Value::as_str),
        Some("invalidMessageFormat")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn ws_join_rejects_unknown_game_mode() {
    let server = start_server().await;
    let mut ws = connect_ws(server.addr).await;

    send_json(
        &mut ws,
        json!({
            "type": "joinOrCreateRoom",
            "gameMode": "not-a-real-mode",
        }),
    )
    .await;
    let err = recv_type(&mut ws, "error").await;
    assert_eq!(
        err.get("code").and_then(Value::as_str),
        Some("invalidGameMode")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn ws_join_unknown_room_code() {
    let server = start_server().await;
    let mut ws = connect_ws(server.addr).await;

    send_json(
        &mut ws,
        json!({
            "type": "joinOrCreateRoom",
            "roomCode": "ZZZZ",
        }),
    )
    .await;
    let err = recv_type(&mut ws, "error").await;
    assert_eq!(
        err.get("code").and_then(Value::as_str),
        Some("roomNotFound")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn ws_full_game_flow_keyboarding() {
    let server = start_server().await;
    let mut ws = connect_ws(server.addr).await;

    // 1. Join/create a keyboarding room.
    send_json(
        &mut ws,
        json!({
            "type": "joinOrCreateRoom",
            "gameMode": "keyboarding",
            "matchDurationSecs": 30,
        }),
    )
    .await;

    let welcome = recv_type(&mut ws, "welcome").await;
    assert_eq!(
        welcome.get("gameKey").and_then(Value::as_str),
        Some("keyboarding")
    );
    let player_id = welcome
        .get("playerId")
        .and_then(Value::as_u64)
        .expect("playerId");
    let room_code = welcome
        .get("roomCode")
        .and_then(Value::as_str)
        .expect("roomCode")
        .to_string();
    assert_eq!(room_code.len(), 4);
    assert!(
        welcome
            .get("rejoinToken")
            .and_then(Value::as_str)
            .is_some_and(|t| !t.is_empty())
    );

    // 2. Initial roomState arrives with the single player.
    let state = recv_type(&mut ws, "roomState").await;
    let room = state.get("room").expect("room obj");
    let players = room.get("players").and_then(Value::as_array).unwrap();
    assert_eq!(players.len(), 1);
    assert_eq!(
        players[0].get("id").and_then(Value::as_u64),
        Some(player_id)
    );
    assert_eq!(
        room.get("hostPlayerId").and_then(Value::as_u64),
        Some(player_id)
    );
    assert_eq!(
        room.get("activePowerups")
            .and_then(Value::as_array)
            .map(|a| a.len()),
        Some(0)
    );

    // 3. Start the match — expect a prompt.
    send_json(&mut ws, json!({ "type": "startMatch" })).await;

    let prompt_state = recv_type(&mut ws, "promptState").await;
    let prompt = prompt_state
        .get("prompt")
        .and_then(Value::as_str)
        .expect("prompt string")
        .to_string();
    assert!(!prompt.is_empty());
    assert_eq!(
        prompt_state.get("playerId").and_then(Value::as_u64),
        Some(player_id)
    );

    // 4. Send a wrong answer, expect a wrongAnswer with shrinkApplied.
    send_json(
        &mut ws,
        json!({ "type": "submitAttempt", "text": "wrong-answer-xyz" }),
    )
    .await;
    let wrong = recv_type(&mut ws, "wrongAnswer").await;
    assert_eq!(
        wrong.get("playerId").and_then(Value::as_u64),
        Some(player_id)
    );
    let shrink = wrong
        .get("shrinkApplied")
        .and_then(Value::as_f64)
        .expect("shrinkApplied");
    assert!(shrink > 0.0);

    // 5. Send correct answer, expect a roundResult with growth and a new prompt.
    send_json(
        &mut ws,
        json!({ "type": "submitAttempt", "text": prompt.clone() }),
    )
    .await;
    let result = recv_type(&mut ws, "roundResult").await;
    assert_eq!(
        result.get("winnerPlayerId").and_then(Value::as_u64),
        Some(player_id)
    );
    let growth = result
        .get("growthAwarded")
        .and_then(Value::as_f64)
        .expect("growthAwarded");
    assert!(growth > 0.0);

    // 6. Observe room size change via roomState.
    let post_room = recv_type(&mut ws, "roomState").await;
    let post_players = post_room
        .get("room")
        .and_then(|r| r.get("players"))
        .and_then(Value::as_array)
        .unwrap();
    let self_player = &post_players[0];
    let size = self_player.get("size").and_then(Value::as_f64).unwrap();
    // shrink from wrong answer (2.0), then growth for round (>=4.0)
    assert!(
        size > 10.0 - shrink,
        "size should have grown back above the shrink floor"
    );

    // 7. A new promptState should also have arrived.
    let second_prompt = recv_type(&mut ws, "promptState").await;
    assert!(
        second_prompt
            .get("prompt")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty())
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn ws_second_socket_can_join_existing_room() {
    let server = start_server().await;

    // First socket creates a room.
    let mut host = connect_ws(server.addr).await;
    send_json(
        &mut host,
        json!({
            "type": "joinOrCreateRoom",
            "gameMode": "keyboarding",
        }),
    )
    .await;
    let welcome = recv_type(&mut host, "welcome").await;
    let room_code = welcome
        .get("roomCode")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    let host_id = welcome.get("playerId").and_then(Value::as_u64).unwrap();
    let _ = recv_type(&mut host, "roomState").await;

    // Second socket joins that room.
    let mut guest = connect_ws(server.addr).await;
    send_json(
        &mut guest,
        json!({
            "type": "joinOrCreateRoom",
            "roomCode": room_code.clone(),
        }),
    )
    .await;
    let guest_welcome = recv_type(&mut guest, "welcome").await;
    let guest_id = guest_welcome
        .get("playerId")
        .and_then(Value::as_u64)
        .unwrap();
    assert_ne!(guest_id, host_id);
    assert_eq!(
        guest_welcome.get("roomCode").and_then(Value::as_str),
        Some(room_code.as_str())
    );

    // Host should see the new player in a broadcast roomState.
    let host_update = recv_type(&mut host, "roomState").await;
    let players = host_update
        .get("room")
        .and_then(|r| r.get("players"))
        .and_then(Value::as_array)
        .unwrap();
    assert_eq!(players.len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn ws_rejoin_restores_player_session() {
    let server = start_server().await;

    // Host joins first and stays connected so the room survives the guest disconnect.
    let mut host = connect_ws(server.addr).await;
    send_json(
        &mut host,
        json!({
            "type": "joinOrCreateRoom",
            "gameMode": "keyboarding",
        }),
    )
    .await;
    let host_welcome = recv_type(&mut host, "welcome").await;
    let room_code = host_welcome
        .get("roomCode")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    let _ = recv_type(&mut host, "roomState").await;

    // Guest joins, then drops without a clean close.
    let mut guest = connect_ws(server.addr).await;
    send_json(
        &mut guest,
        json!({
            "type": "joinOrCreateRoom",
            "roomCode": room_code.clone(),
        }),
    )
    .await;
    let guest_welcome = recv_type(&mut guest, "welcome").await;
    let guest_id = guest_welcome
        .get("playerId")
        .and_then(Value::as_u64)
        .unwrap();
    let token = guest_welcome
        .get("rejoinToken")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();

    drop(guest);
    // Give the server a moment to notice the disconnect.
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut rejoin = connect_ws(server.addr).await;
    send_json(
        &mut rejoin,
        json!({ "type": "rejoinRoom", "rejoinToken": token }),
    )
    .await;
    let welcome2 = recv_type(&mut rejoin, "welcome").await;
    assert_eq!(
        welcome2.get("playerId").and_then(Value::as_u64),
        Some(guest_id)
    );
    assert_eq!(
        welcome2.get("roomCode").and_then(Value::as_str),
        Some(room_code.as_str())
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn ws_rejoin_rejects_unknown_token() {
    let server = start_server().await;
    let mut ws = connect_ws(server.addr).await;

    send_json(
        &mut ws,
        json!({ "type": "rejoinRoom", "rejoinToken": "definitely-not-real" }),
    )
    .await;
    let err = recv_type(&mut ws, "error").await;
    assert_eq!(
        err.get("code").and_then(Value::as_str),
        Some("invalidRejoinToken")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn ws_arithmetic_room_uses_arithmetic_prompts() {
    let server = start_server().await;
    let mut ws = connect_ws(server.addr).await;

    send_json(
        &mut ws,
        json!({
            "type": "joinOrCreateRoom",
            "gameMode": "arithmetic",
            "matchDurationSecs": 30,
            "gameOptions": { "operation": "addition" },
        }),
    )
    .await;
    let welcome = recv_type(&mut ws, "welcome").await;
    assert_eq!(
        welcome.get("gameKey").and_then(Value::as_str),
        Some("arithmetic")
    );
    assert_eq!(
        welcome.get("inputMode").and_then(Value::as_str),
        Some("decimal")
    );

    let _ = recv_type(&mut ws, "roomState").await;
    send_json(&mut ws, json!({ "type": "startMatch" })).await;

    let prompt_state = recv_type(&mut ws, "promptState").await;
    let prompt = prompt_state.get("prompt").and_then(Value::as_str).unwrap();
    assert!(
        prompt.contains('+'),
        "arithmetic addition prompt should contain '+': {prompt}"
    );
}
