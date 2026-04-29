use crate::adapter::{AdapterHandle, AdapterRegistry, OptionField, build_adapter_registry};
use crate::customization::{ADJECTIVES, NOUNS, PALETTE};
use crate::game::{
    DEFAULT_START_SIZE, PlayerId, PlayerState, RoomState, apply_round_win,
    apply_wrong_answer_penalty, deduct_from_top_players, find_top_player_ids,
    resolve_match_by_timer,
};
use crate::powerup::{
    ActivePowerUp, PowerUpKind, PowerUpOffer, cleanup_expired, distribution_interval,
    effect_duration, has_double_points, has_ongoing_score_steal, offer_duration, pick_powerup_kind,
    pick_powerup_recipient,
};
use crate::protocol::{ClientMessage, ErrorCode, ServerMessage};
use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use futures_util::{SinkExt, StreamExt};
use rand::distr::Alphanumeric;
use rand::{Rng, RngExt};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, mpsc};

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub growth_per_round_win: f32,
    pub shrink_per_wrong_answer: f32,
    pub match_duration_secs: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:4000".to_string(),
            growth_per_round_win: 4.0,
            shrink_per_wrong_answer: 2.0,
            match_duration_secs: 60,
        }
    }
}

#[derive(Debug)]
struct RoomConnection {
    sender: mpsc::UnboundedSender<Message>,
}

struct SharedState {
    adapters: AdapterRegistry,
    adapter_order: Vec<String>,
    default_game_key: String,
    config: ServerConfig,
    rooms: Mutex<HashMap<String, RoomState>>,
    connections: Mutex<HashMap<String, HashMap<PlayerId, RoomConnection>>>,
    rejoin_tokens: Mutex<HashMap<String, (String, PlayerId)>>,
    prompt_seed: AtomicU64,
}

pub fn build_app(adapters: Vec<AdapterHandle>, config: ServerConfig) -> Result<Router, String> {
    let default_game_key = adapters
        .first()
        .map(|adapter| adapter.game_key().to_string())
        .ok_or_else(|| "at least one adapter must be registered".to_string())?;
    let adapter_order: Vec<String> = adapters.iter().map(|a| a.game_key().to_string()).collect();
    let adapters = build_adapter_registry(adapters)?;
    let state = Arc::new(SharedState {
        adapters,
        adapter_order,
        default_game_key,
        config,
        rooms: Mutex::new(HashMap::new()),
        connections: Mutex::new(HashMap::new()),
        rejoin_tokens: Mutex::new(HashMap::new()),
        prompt_seed: AtomicU64::new(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
        ),
    });

    Ok(Router::new()
        .route("/healthz", get(health_handler))
        .route("/readyz", get(health_handler))
        .route("/api/game-modes", get(game_modes_handler))
        .route("/ws", get(ws_handler))
        .with_state(state))
}

pub async fn run_server(adapters: Vec<AdapterHandle>, config: ServerConfig) -> Result<(), String> {
    let listener = TcpListener::bind(&config.bind_addr)
        .await
        .map_err(|e| format!("failed to bind {}: {e}", config.bind_addr))?;
    let app = build_app(adapters, config)?;

    axum::serve(listener, app)
        .await
        .map_err(|e| format!("server error: {e}"))
}

async fn health_handler() -> impl IntoResponse {
    "ok"
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GameModeInfo {
    key: String,
    label: String,
    options: Vec<OptionField>,
}

async fn game_modes_handler(State(state): State<Arc<SharedState>>) -> Json<Vec<GameModeInfo>> {
    let modes = state
        .adapter_order
        .iter()
        .filter_map(|key| {
            state.adapters.get(key).map(|a| GameModeInfo {
                key: a.game_key().to_string(),
                label: a.game_label().to_string(),
                options: a.option_schema(),
            })
        })
        .collect();
    Json(modes)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<SharedState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<SharedState>) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (client_tx, mut client_rx) = mpsc::unbounded_channel::<Message>();

    let writer_task = tokio::spawn(async move {
        while let Some(msg) = client_rx.recv().await {
            if ws_tx.send(msg).await.is_err() {
                break;
            }
        }
    });

    let mut player_id: Option<PlayerId> = None;
    let mut room_code: Option<String> = None;

    while let Some(Ok(msg)) = ws_rx.next().await {
        let Message::Text(raw_text) = msg else {
            continue;
        };

        let incoming = match serde_json::from_str::<ClientMessage>(&raw_text) {
            Ok(parsed) => parsed,
            Err(_) => {
                let _ = send_server_message(
                    &client_tx,
                    &ServerMessage::Error {
                        message: "Invalid message format".to_string(),
                        code: Some(ErrorCode::InvalidMessageFormat),
                    },
                );
                continue;
            }
        };

        match incoming {
            ClientMessage::JoinOrCreateRoom {
                room_code: requested_room_code,
                game_mode,
                match_duration_secs,
                game_options,
                player_name,
                player_color,
            } => {
                if player_id.is_some() {
                    continue;
                }

                let result = join_or_create_room(
                    &state,
                    requested_room_code,
                    game_mode,
                    match_duration_secs,
                    game_options,
                    player_name,
                    player_color,
                    client_tx.clone(),
                )
                .await;

                match result {
                    Ok((code, token, assigned_player_id)) => {
                        player_id = Some(assigned_player_id);
                        room_code = Some(code.clone());

                        {
                            let mut tokens = state.rejoin_tokens.lock().await;
                            tokens.insert(token.clone(), (code.clone(), assigned_player_id));
                        }

                        let adapter = adapter_for_room(&state, &code).await;
                        let _ = send_server_message(
                            &client_tx,
                            &ServerMessage::Welcome {
                                player_id: assigned_player_id,
                                room_code: code.clone(),
                                game_key: room_game_key(&state, &code)
                                    .await
                                    .unwrap_or_else(|| state.default_game_key.clone()),
                                input_placeholder: adapter
                                    .as_ref()
                                    .map(|a| a.input_placeholder().to_string())
                                    .unwrap_or_default(),
                                input_mode: adapter
                                    .as_ref()
                                    .map(|a| a.input_mode().to_string())
                                    .unwrap_or_else(|| "text".to_string()),
                                rejoin_token: token,
                            },
                        );

                        let _ = broadcast_room_state(&state, &code).await;
                        // Assign a prompt if the match is already running
                        let _ = ensure_prompt_for_player(&state, &code, assigned_player_id).await;
                    }
                    Err(JoinError::RoomNotFound(code)) => {
                        let _ = send_server_message(
                            &client_tx,
                            &ServerMessage::Error {
                                message: format!("No room found with code {code}"),
                                code: Some(ErrorCode::RoomNotFound),
                            },
                        );
                    }
                    Err(JoinError::InvalidGameMode(mode)) => {
                        let _ = send_server_message(
                            &client_tx,
                            &ServerMessage::Error {
                                message: format!("Game mode '{mode}' is not available"),
                                code: Some(ErrorCode::InvalidGameMode),
                            },
                        );
                    }
                }
            }
            ClientMessage::RejoinRoom { rejoin_token } => {
                if player_id.is_some() {
                    continue;
                }

                let lookup = {
                    let tokens = state.rejoin_tokens.lock().await;
                    tokens.get(&rejoin_token).cloned()
                };

                let Some((found_code, found_pid)) = lookup else {
                    let _ = send_server_message(
                        &client_tx,
                        &ServerMessage::Error {
                            message: "Session expired — please rejoin the room".to_string(),
                            code: Some(ErrorCode::InvalidRejoinToken),
                        },
                    );
                    continue;
                };

                let prompt_snapshot = {
                    let mut rooms = state.rooms.lock().await;
                    let Some(room) = rooms.get_mut(&found_code) else {
                        let mut tokens = state.rejoin_tokens.lock().await;
                        tokens.remove(&rejoin_token);
                        let _ = send_server_message(
                            &client_tx,
                            &ServerMessage::Error {
                                message: "Room no longer exists".to_string(),
                                code: Some(ErrorCode::RoomExpired),
                            },
                        );
                        continue;
                    };
                    let Some(player) = room.players.get_mut(&found_pid) else {
                        let mut tokens = state.rejoin_tokens.lock().await;
                        tokens.remove(&rejoin_token);
                        let _ = send_server_message(
                            &client_tx,
                            &ServerMessage::Error {
                                message: "Player no longer in room".to_string(),
                                code: Some(ErrorCode::PlayerNotInRoom),
                            },
                        );
                        continue;
                    };
                    player.connected = true;

                    if player.prompt.is_empty() {
                        None
                    } else {
                        Some((player.prompt_id, player.prompt.clone()))
                    }
                };

                {
                    let mut connections = state.connections.lock().await;
                    connections.entry(found_code.clone()).or_default().insert(
                        found_pid,
                        RoomConnection {
                            sender: client_tx.clone(),
                        },
                    );
                }

                player_id = Some(found_pid);
                room_code = Some(found_code.clone());

                let adapter = adapter_for_room(&state, &found_code).await;
                let _ = send_server_message(
                    &client_tx,
                    &ServerMessage::Welcome {
                        player_id: found_pid,
                        room_code: found_code.clone(),
                        game_key: room_game_key(&state, &found_code)
                            .await
                            .unwrap_or_else(|| state.default_game_key.clone()),
                        input_placeholder: adapter
                            .as_ref()
                            .map(|a| a.input_placeholder().to_string())
                            .unwrap_or_default(),
                        input_mode: adapter
                            .as_ref()
                            .map(|a| a.input_mode().to_string())
                            .unwrap_or_else(|| "text".to_string()),
                        rejoin_token,
                    },
                );

                let _ = broadcast_room_state(&state, &found_code).await;

                if let Some((round_id, prompt)) = prompt_snapshot {
                    let _ = send_server_message(
                        &client_tx,
                        &ServerMessage::PromptState {
                            room_code: found_code,
                            player_id: found_pid,
                            round_id,
                            prompt,
                        },
                    );
                }
            }
            ClientMessage::InputUpdate { text } => {
                if let (Some(pid), Some(code)) = (player_id, room_code.as_ref()) {
                    handle_progress_update(&state, code, pid, text).await;
                }
            }
            ClientMessage::SubmitAttempt { text } => {
                if let (Some(pid), Some(code)) = (player_id, room_code.as_ref()) {
                    handle_submission(&state, code, pid, text).await;
                }
            }
            ClientMessage::StartMatch => {
                if let (Some(pid), Some(code)) = (player_id, room_code.as_ref()) {
                    handle_start_match(&state, code, pid).await;
                }
            }
            ClientMessage::Rematch => {
                if let (Some(_pid), Some(code)) = (player_id, room_code.as_ref()) {
                    handle_rematch(&state, code).await;
                }
            }
            ClientMessage::UpdateRoomSettings {
                game_mode,
                match_duration_secs,
                game_options,
            } => {
                if let (Some(pid), Some(code)) = (player_id, room_code.as_ref()) {
                    handle_update_room_settings(
                        &state,
                        code,
                        pid,
                        game_mode,
                        match_duration_secs,
                        game_options,
                        &client_tx,
                    )
                    .await;
                }
            }
        }
    }

    if let (Some(pid), Some(code)) = (player_id, room_code) {
        disconnect_player(&state, &code, pid).await;
    }

    writer_task.abort();
}

#[derive(Debug)]
enum JoinError {
    RoomNotFound(String),
    InvalidGameMode(String),
}

#[allow(clippy::too_many_arguments)]
async fn join_or_create_room(
    state: &Arc<SharedState>,
    requested_room_code: Option<String>,
    requested_game_mode: Option<String>,
    requested_match_duration_secs: Option<u64>,
    requested_game_options: Option<serde_json::Value>,
    requested_player_name: Option<String>,
    requested_player_color: Option<String>,
    sender: mpsc::UnboundedSender<Message>,
) -> Result<(String, String, PlayerId), JoinError> {
    let token = generate_rejoin_token();
    let mut rooms = state.rooms.lock().await;
    let mut connections = state.connections.lock().await;

    let room_code = match requested_room_code {
        Some(code) if rooms.contains_key(&code) => code,
        Some(code) => return Err(JoinError::RoomNotFound(code)),
        None => {
            let requested = requested_game_mode
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let room_game_key = match requested {
                Some(game_key) => {
                    if state.adapters.contains_key(game_key) {
                        game_key.to_string()
                    } else {
                        return Err(JoinError::InvalidGameMode(game_key.to_string()));
                    }
                }
                None => state.default_game_key.clone(),
            };
            let generated = generate_room_code(&rooms);
            let duration = requested_match_duration_secs
                .filter(|&s| s > 0)
                .unwrap_or(state.config.match_duration_secs);
            rooms.insert(
                generated.clone(),
                RoomState {
                    room_code: generated.clone(),
                    game_key: room_game_key,
                    game_options: requested_game_options.unwrap_or(serde_json::Value::Null),
                    players: HashMap::new(),
                    match_winner: None,
                    match_deadline: None,
                    match_duration_secs: duration,
                    host_player_id: 1,
                    next_player_id: 1,
                    powerup_offers: Vec::new(),
                    active_powerups: Vec::new(),
                    next_offer_id: 0,
                    match_generation: 0,
                },
            );
            generated
        }
    };

    let room = rooms
        .get_mut(&room_code)
        .expect("room was just verified or inserted");

    let player_id = room.next_player_id;
    room.next_player_id += 1;

    room.players.insert(
        player_id,
        PlayerState {
            id: player_id,
            name: requested_player_name
                .filter(|n| validate_player_name(n))
                .unwrap_or_else(|| generate_player_name(&mut rand::rng())),
            size: DEFAULT_START_SIZE,
            color: requested_player_color
                .filter(|c| validate_player_color(c))
                .unwrap_or_else(|| generate_color(player_id)),
            connected: true,
            progress: String::new(),
            rejoin_token: token.clone(),
            prompt: String::new(),
            prompt_id: 0,
        },
    );

    connections
        .entry(room_code.clone())
        .or_default()
        .insert(player_id, RoomConnection { sender });

    Ok((room_code, token, player_id))
}

async fn handle_progress_update(
    state: &Arc<SharedState>,
    room_code: &str,
    player_id: PlayerId,
    text: String,
) {
    let Some(adapter) = adapter_for_room(state, room_code).await else {
        return;
    };
    let normalized = adapter.normalize_progress(&text);

    {
        let mut rooms = state.rooms.lock().await;
        let Some(room) = rooms.get_mut(room_code) else {
            return;
        };
        let Some(player) = room.players.get_mut(&player_id) else {
            return;
        };
        player.progress = normalized.clone();
    }

    let _ = broadcast_to_room(
        state,
        room_code,
        &ServerMessage::RaceProgress {
            room_code: room_code.to_string(),
            player_id,
            text: normalized,
        },
    )
    .await;
}

async fn handle_submission(
    state: &Arc<SharedState>,
    room_code: &str,
    player_id: PlayerId,
    text: String,
) {
    let Some(adapter) = adapter_for_room(state, room_code).await else {
        return;
    };
    let mut should_advance_prompt = false;
    let mut round_result: Option<ServerMessage> = None;
    let mut wrong_answer_msg: Option<ServerMessage> = None;
    let mut earned_powerups: Vec<ServerMessage> = Vec::new();

    {
        let mut rooms = state.rooms.lock().await;
        let Some(room) = rooms.get_mut(room_code) else {
            return;
        };

        if room.match_winner.is_some() {
            return;
        }

        let (player_prompt, player_prompt_id) = match room.players.get(&player_id) {
            Some(p) if !p.prompt.is_empty() => (p.prompt.clone(), p.prompt_id),
            _ => return,
        };

        if !adapter.is_correct(&player_prompt, &text) {
            let penalty = state.config.shrink_per_wrong_answer;
            if let Some(shrink) = apply_wrong_answer_penalty(room, player_id, penalty) {
                wrong_answer_msg = Some(ServerMessage::WrongAnswer {
                    room_code: room_code.to_string(),
                    player_id,
                    shrink_applied: shrink,
                });
            }
        } else {
            let configured_growth = state.config.growth_per_round_win;
            let mut growth = adapter
                .score_for_prompt(&player_prompt)
                .max(configured_growth);

            if has_double_points(&room.active_powerups, player_id) {
                growth *= 2.0;
            }

            if let Some(resolution) = apply_round_win(room, player_id, growth) {
                round_result = Some(ServerMessage::RoundResult {
                    room_code: room_code.to_string(),
                    round_id: player_prompt_id,
                    winner_player_id: resolution.round_winner,
                    growth_awarded: growth,
                });
                should_advance_prompt = room.match_winner.is_none();

                if has_ongoing_score_steal(&room.active_powerups, player_id) {
                    deduct_from_top_players(room, growth, player_id);
                }

                let now = Instant::now();
                let oldest_idx = room
                    .powerup_offers
                    .iter()
                    .enumerate()
                    .filter(|(_, o)| o.player_id == player_id && o.expires_at > now)
                    .min_by_key(|(_, o)| o.expires_at)
                    .map(|(idx, _)| idx);

                if let Some(idx) = oldest_idx {
                    let offer = room.powerup_offers.swap_remove(idx);
                    let player_count = room.players.values().filter(|p| p.connected).count();
                    let duration = effect_duration(offer.kind, player_count);
                    room.active_powerups.push(ActivePowerUp {
                        kind: offer.kind,
                        source_player_id: player_id,
                        expires_at: now + duration,
                        duration,
                    });
                    earned_powerups.push(ServerMessage::PowerUpActivated {
                        offer_id: offer.offer_id,
                        player_id,
                        kind: offer.kind,
                        duration_ms: duration.as_millis() as u64,
                    });

                    if offer.kind == PowerUpKind::ScoreSteal {
                        let top_ids = find_top_player_ids(&room.players, player_id);
                        if let Some(&first) = top_ids.first() {
                            let top_size = room.players.get(&first).map(|p| p.size).unwrap_or(0.0);
                            let chunk = top_size * 0.10;
                            let actual = deduct_from_top_players(room, chunk, player_id);
                            if let Some(p) = room.players.get_mut(&player_id) {
                                p.size += actual;
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(msg) = wrong_answer_msg {
        let _ = broadcast_to_room(state, room_code, &msg).await;
        let _ = broadcast_room_state(state, room_code).await;
        return;
    }

    let had_round_result = round_result.is_some();
    if let Some(msg) = round_result {
        let _ = broadcast_to_room(state, room_code, &msg).await;
    }

    for msg in &earned_powerups {
        let _ = broadcast_to_room(state, room_code, msg).await;
    }

    if had_round_result || !earned_powerups.is_empty() {
        let _ = broadcast_room_state(state, room_code).await;
    }

    if should_advance_prompt {
        let _ = ensure_prompt_for_player(state, room_code, player_id).await;
    }
}

async fn handle_start_match(state: &Arc<SharedState>, room_code: &str, player_id: PlayerId) {
    let duration_secs;
    {
        let mut rooms = state.rooms.lock().await;
        let Some(room) = rooms.get_mut(room_code) else {
            return;
        };
        if player_id != room.host_player_id || room.match_deadline.is_some() {
            return;
        }
        duration_secs = room.match_duration_secs;
        let deadline = Instant::now() + Duration::from_secs(duration_secs);
        room.match_deadline = Some(deadline);
    }

    let generation = {
        let rooms = state.rooms.lock().await;
        rooms
            .get(room_code)
            .map(|room| room.match_generation)
            .unwrap_or(0)
    };
    start_match_timer(
        state.clone(),
        room_code.to_string(),
        duration_secs,
        generation,
    );
    start_powerup_timer(
        state.clone(),
        room_code.to_string(),
        duration_secs,
        generation,
    );

    let _ = broadcast_room_state(state, room_code).await;
    ensure_prompts_for_all_players(state, room_code).await;
}

async fn handle_update_room_settings(
    state: &Arc<SharedState>,
    room_code: &str,
    player_id: PlayerId,
    requested_game_mode: Option<String>,
    requested_match_duration_secs: Option<u64>,
    requested_game_options: Option<serde_json::Value>,
    sender: &mpsc::UnboundedSender<Message>,
) {
    let validated_game_mode = requested_game_mode
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    if let Some(ref mode) = validated_game_mode
        && !state.adapters.contains_key(mode)
    {
        let _ = send_server_message(
            sender,
            &ServerMessage::Error {
                message: format!("Game mode '{mode}' is not available"),
                code: Some(ErrorCode::InvalidGameMode),
            },
        );
        return;
    }

    {
        let mut rooms = state.rooms.lock().await;
        let Some(room) = rooms.get_mut(room_code) else {
            return;
        };
        if room.host_player_id != player_id {
            let _ = send_server_message(
                sender,
                &ServerMessage::Error {
                    message: "Only the host can change room settings".to_string(),
                    code: None,
                },
            );
            return;
        }

        if let Some(mode) = validated_game_mode {
            room.game_key = mode;
        }
        if let Some(secs) = requested_match_duration_secs.filter(|&s| s > 0) {
            room.match_duration_secs = secs;
        }
        if let Some(opts) = requested_game_options {
            room.game_options = opts;
        }

        room.reset_for_rematch();
    }

    let _ = broadcast_room_state(state, room_code).await;
}

async fn handle_rematch(state: &Arc<SharedState>, room_code: &str) {
    let duration_secs;
    let generation;
    {
        let mut rooms = state.rooms.lock().await;
        let Some(room) = rooms.get_mut(room_code) else {
            return;
        };
        if room.match_winner.is_none() {
            return;
        }
        room.reset_for_rematch();
        duration_secs = room.match_duration_secs;
        generation = room.match_generation;
        let deadline = Instant::now() + Duration::from_secs(duration_secs);
        room.match_deadline = Some(deadline);
    }

    start_match_timer(
        state.clone(),
        room_code.to_string(),
        duration_secs,
        generation,
    );
    start_powerup_timer(
        state.clone(),
        room_code.to_string(),
        duration_secs,
        generation,
    );

    let _ = broadcast_room_state(state, room_code).await;
    ensure_prompts_for_all_players(state, room_code).await;
}

fn splitmix64(val: u64) -> u64 {
    let mut z = val.wrapping_add(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

async fn ensure_prompt_for_player(
    state: &Arc<SharedState>,
    room_code: &str,
    player_id: PlayerId,
) -> bool {
    let Some(adapter) = adapter_for_room(state, room_code).await else {
        return false;
    };
    let prompt_update;
    {
        let mut rooms = state.rooms.lock().await;
        let Some(room) = rooms.get_mut(room_code) else {
            return false;
        };
        if room.match_winner.is_some() || room.match_deadline.is_none() {
            return false;
        }
        let Some(player) = room.players.get_mut(&player_id) else {
            return false;
        };
        if !player.connected {
            return false;
        }
        let raw_seed = state.prompt_seed.fetch_add(1, Ordering::Relaxed);
        let seed = splitmix64(raw_seed);
        player.prompt_id += 1;
        player.prompt = adapter.next_prompt(seed, &room.game_options);
        player.progress.clear();
        prompt_update = (player.prompt_id, player.prompt.clone());
    }

    let (round_id, prompt) = prompt_update;
    let _ = send_to_player(
        state,
        room_code,
        player_id,
        &ServerMessage::PromptState {
            room_code: room_code.to_string(),
            player_id,
            round_id,
            prompt,
        },
    )
    .await;

    true
}

async fn ensure_prompts_for_all_players(state: &Arc<SharedState>, room_code: &str) {
    let Some(adapter) = adapter_for_room(state, room_code).await else {
        return;
    };
    let prompt_updates;
    {
        let mut rooms = state.rooms.lock().await;
        let Some(room) = rooms.get_mut(room_code) else {
            return;
        };
        if room.match_winner.is_some() || room.players.is_empty() || room.match_deadline.is_none() {
            return;
        }
        let mut updates: Vec<(PlayerId, u64, String)> = Vec::new();
        for player in room.players.values_mut() {
            if !player.connected {
                continue;
            }
            let raw_seed = state.prompt_seed.fetch_add(1, Ordering::Relaxed);
            let seed = splitmix64(raw_seed);
            player.prompt_id += 1;
            player.prompt = adapter.next_prompt(seed, &room.game_options);
            player.progress.clear();
            updates.push((player.id, player.prompt_id, player.prompt.clone()));
        }
        prompt_updates = updates;
    }

    for (pid, round_id, prompt) in prompt_updates {
        let _ = send_to_player(
            state,
            room_code,
            pid,
            &ServerMessage::PromptState {
                room_code: room_code.to_string(),
                player_id: pid,
                round_id,
                prompt,
            },
        )
        .await;
    }
}

fn start_match_timer(
    state: Arc<SharedState>,
    room_code: String,
    duration_secs: u64,
    generation: u64,
) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(duration_secs)).await;
        {
            let mut rooms = state.rooms.lock().await;
            let Some(room) = rooms.get_mut(&room_code) else {
                return;
            };
            if room.match_generation != generation {
                return;
            }
            resolve_match_by_timer(room);
        }
        let _ = broadcast_room_state(&state, &room_code).await;
    });
}

fn start_powerup_timer(
    state: Arc<SharedState>,
    room_code: String,
    match_duration_secs: u64,
    generation: u64,
) {
    tokio::spawn(async move {
        let match_end = Instant::now() + Duration::from_secs(match_duration_secs);
        let mut next_interval = distribution_interval(2);
        loop {
            tokio::time::sleep(next_interval).await;

            let now = Instant::now();
            if now >= match_end {
                break;
            }

            let mut expired_offer_notifs: Vec<ServerMessage> = Vec::new();
            let mut expired_effect_broadcasts: Vec<ServerMessage> = Vec::new();
            let mut new_offer_notif: Option<ServerMessage> = None;

            {
                let mut rooms = state.rooms.lock().await;
                let Some(room) = rooms.get_mut(&room_code) else {
                    break;
                };
                if room.match_generation != generation {
                    break;
                }
                if room.match_winner.is_some() {
                    break;
                }

                let expired =
                    cleanup_expired(&mut room.powerup_offers, &mut room.active_powerups, now);

                for offer in &expired.expired_offers {
                    expired_offer_notifs.push(ServerMessage::PowerUpOfferExpired {
                        offer_id: offer.offer_id,
                        player_id: offer.player_id,
                        kind: offer.kind,
                    });
                }
                for effect in &expired.expired_effects {
                    expired_effect_broadcasts.push(ServerMessage::PowerUpEffectEnded {
                        player_id: effect.source_player_id,
                        kind: effect.kind,
                    });
                }

                let players: Vec<(PlayerId, f32)> = room
                    .players
                    .values()
                    .filter(|p| p.connected)
                    .map(|p| (p.id, p.size))
                    .collect();

                let player_count = players.len();
                next_interval = distribution_interval(player_count);

                let mut rng = rand::rng();
                if let Some(recipient) = pick_powerup_recipient(&players, &mut rng) {
                    let kind = pick_powerup_kind(&mut rng);
                    let offer_dur = offer_duration(player_count);
                    let expires_at = now + offer_dur;
                    let offer_id = room.next_offer_id;
                    room.next_offer_id += 1;
                    room.powerup_offers.push(PowerUpOffer {
                        offer_id,
                        kind,
                        player_id: recipient,
                        expires_at,
                    });
                    new_offer_notif = Some(ServerMessage::PowerUpOffered {
                        offer_id,
                        player_id: recipient,
                        kind,
                        expires_in_ms: offer_dur.as_millis() as u64,
                    });
                }
            }

            for msg in expired_offer_notifs {
                let _ = broadcast_to_room(&state, &room_code, &msg).await;
            }
            let had_expired_effects = !expired_effect_broadcasts.is_empty();
            for msg in expired_effect_broadcasts {
                let _ = broadcast_to_room(&state, &room_code, &msg).await;
            }
            if had_expired_effects {
                let _ = broadcast_room_state(&state, &room_code).await;
            }
            if let Some(msg) = new_offer_notif {
                let _ = broadcast_to_room(&state, &room_code, &msg).await;
            }
        }
    });
}

async fn disconnect_player(state: &Arc<SharedState>, room_code: &str, player_id: PlayerId) {
    {
        let mut connections = state.connections.lock().await;
        if let Some(room_connections) = connections.get_mut(room_code) {
            room_connections.remove(&player_id);
            if room_connections.is_empty() {
                connections.remove(room_code);
            }
        }
    }

    let all_disconnected;
    {
        let mut rooms = state.rooms.lock().await;
        if let Some(room) = rooms.get_mut(room_code) {
            if let Some(player) = room.players.get_mut(&player_id) {
                player.connected = false;
            }
            all_disconnected = room.players.values().all(|p| !p.connected);
            if all_disconnected {
                rooms.remove(room_code);
            }
        } else {
            all_disconnected = true;
        }
    }

    if all_disconnected {
        let mut tokens = state.rejoin_tokens.lock().await;
        tokens.retain(|_, (rc, _)| rc != room_code);
    } else {
        let _ = broadcast_room_state(state, room_code).await;
    }
}

async fn send_to_player(
    state: &Arc<SharedState>,
    room_code: &str,
    player_id: PlayerId,
    message: &ServerMessage,
) -> bool {
    let connections = state.connections.lock().await;
    let Some(room_connections) = connections.get(room_code) else {
        return false;
    };
    let Some(conn) = room_connections.get(&player_id) else {
        return false;
    };
    drop(send_server_message(&conn.sender, message));
    true
}

async fn broadcast_room_state(state: &Arc<SharedState>, room_code: &str) -> bool {
    let mut snapshot = {
        let rooms = state.rooms.lock().await;
        let Some(room) = rooms.get(room_code) else {
            return false;
        };
        room.to_snapshot()
    };

    if let Some(adapter) = state.adapters.get(&snapshot.game_key) {
        snapshot.input_mode = adapter.input_mode().to_string();
        snapshot.input_placeholder = adapter.input_placeholder().to_string();
    }

    broadcast_to_room(
        state,
        room_code,
        &ServerMessage::RoomState { room: snapshot },
    )
    .await
}

async fn broadcast_to_room(
    state: &Arc<SharedState>,
    room_code: &str,
    message: &ServerMessage,
) -> bool {
    let connections = state.connections.lock().await;
    let Some(room_connections) = connections.get(room_code) else {
        return false;
    };

    room_connections
        .values()
        .for_each(|conn| drop(send_server_message(&conn.sender, message)));
    true
}

fn send_server_message<T: Serialize>(
    sender: &mpsc::UnboundedSender<Message>,
    message: &T,
) -> Result<(), String> {
    let encoded = serde_json::to_string(message).map_err(|e| format!("encode error: {e}"))?;
    sender
        .send(Message::Text(encoded.into()))
        .map_err(|e| format!("send error: {e}"))
}

async fn room_game_key(state: &Arc<SharedState>, room_code: &str) -> Option<String> {
    let rooms = state.rooms.lock().await;
    rooms.get(room_code).map(|room| room.game_key.clone())
}

async fn adapter_for_room(state: &Arc<SharedState>, room_code: &str) -> Option<AdapterHandle> {
    let game_key = room_game_key(state, room_code).await?;
    state.adapters.get(&game_key).cloned()
}

fn generate_rejoin_token() -> String {
    rand::rng()
        .sample_iter(Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

fn generate_room_code(rooms: &HashMap<String, RoomState>) -> String {
    let mut rng = rand::rng();
    loop {
        let code = (0..4)
            .map(|_| (b'A' + rng.random_range(0..26)) as char)
            .collect::<String>();
        if !rooms.contains_key(&code) {
            return code;
        }
    }
}

fn generate_player_name(rng: &mut impl Rng) -> String {
    let adj = ADJECTIVES[rng.random_range(0..ADJECTIVES.len())];
    let noun = NOUNS[rng.random_range(0..NOUNS.len())];
    format!("{adj} {noun}")
}

fn generate_color(player_id: PlayerId) -> String {
    PALETTE[(player_id as usize) % PALETTE.len()].to_string()
}

fn validate_player_name(name: &str) -> bool {
    let mut parts = name.splitn(2, ' ');
    let (Some(adj), Some(noun)) = (parts.next(), parts.next()) else {
        return false;
    };
    ADJECTIVES.contains(&adj) && NOUNS.contains(&noun)
}

fn validate_player_color(color: &str) -> bool {
    PALETTE.contains(&color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::GameAdapter;

    #[derive(Debug)]
    struct TestAdapter {
        key: &'static str,
        prompt_prefix: &'static str,
        score: f32,
    }

    impl GameAdapter for TestAdapter {
        fn game_key(&self) -> &'static str {
            self.key
        }

        fn next_prompt(&self, seed: u64, _options: &serde_json::Value) -> String {
            format!("{}-{seed}", self.prompt_prefix)
        }

        fn is_correct(&self, prompt: &str, attempt: &str) -> bool {
            prompt == attempt.trim()
        }

        fn normalize_progress(&self, raw_input: &str) -> String {
            raw_input.trim().to_string()
        }

        fn score_for_prompt(&self, _prompt: &str) -> f32 {
            self.score
        }
    }

    fn test_state() -> Arc<SharedState> {
        let adapters = build_adapter_registry(vec![
            Arc::new(TestAdapter {
                key: "keyboarding",
                prompt_prefix: "kbd",
                score: 3.0,
            }),
            Arc::new(TestAdapter {
                key: "arithmetic",
                prompt_prefix: "math",
                score: 9.0,
            }),
        ])
        .expect("adapter registry");

        Arc::new(SharedState {
            adapters,
            adapter_order: vec!["keyboarding".to_string(), "arithmetic".to_string()],
            default_game_key: "keyboarding".to_string(),
            config: ServerConfig::default(),
            rooms: Mutex::new(HashMap::new()),
            connections: Mutex::new(HashMap::new()),
            rejoin_tokens: Mutex::new(HashMap::new()),
            prompt_seed: AtomicU64::new(1),
        })
    }

    #[tokio::test]
    async fn creates_room_with_requested_game_mode() {
        let state = test_state();
        let (sender, _) = mpsc::unbounded_channel::<Message>();

        let (room_code, _token, _pid) = join_or_create_room(
            &state,
            None,
            Some("arithmetic".to_string()),
            None,
            None,
            None,
            None,
            sender,
        )
        .await
        .expect("room created");

        let rooms = state.rooms.lock().await;
        let room = rooms.get(&room_code).expect("room exists");
        assert_eq!(room.game_key, "arithmetic");
    }

    #[tokio::test]
    async fn rejects_unknown_game_mode_on_room_create() {
        let state = test_state();
        let (sender, _) = mpsc::unbounded_channel::<Message>();

        let result = join_or_create_room(
            &state,
            None,
            Some("unknown-mode".to_string()),
            None,
            None,
            None,
            None,
            sender,
        )
        .await;

        assert!(result.is_err());
        assert!(state.rooms.lock().await.is_empty());
    }

    #[tokio::test]
    async fn join_existing_room_ignores_requested_game_mode() {
        let state = test_state();
        let (sender_1, _) = mpsc::unbounded_channel::<Message>();
        let (sender_2, _) = mpsc::unbounded_channel::<Message>();

        let (room_code, _token, _pid) = join_or_create_room(
            &state,
            None,
            Some("keyboarding".to_string()),
            None,
            None,
            None,
            None,
            sender_1,
        )
        .await
        .expect("room created");

        let (joined_room_code, _token, _pid) = join_or_create_room(
            &state,
            Some(room_code.clone()),
            Some("arithmetic".to_string()),
            None,
            None,
            None,
            None,
            sender_2,
        )
        .await
        .expect("joined room");

        assert_eq!(joined_room_code, room_code);
        let rooms = state.rooms.lock().await;
        let room = rooms.get(&room_code).expect("room exists");
        assert_eq!(room.game_key, "keyboarding");
        assert_eq!(room.players.len(), 2);
    }

    #[tokio::test]
    async fn uses_room_adapter_for_prompt_and_scoring() {
        let state = test_state();
        let (sender, _) = mpsc::unbounded_channel::<Message>();
        let (room_code, _token, pid) = join_or_create_room(
            &state,
            None,
            Some("arithmetic".to_string()),
            None,
            None,
            None,
            None,
            sender,
        )
        .await
        .expect("room created");

        handle_start_match(&state, &room_code, pid).await;
        let has_prompt = {
            let rooms = state.rooms.lock().await;
            let room = rooms.get(&room_code).expect("room exists");
            assert!(room.match_deadline.is_some());
            room.players
                .get(&pid)
                .map(|p| !p.prompt.is_empty())
                .unwrap_or(false)
        };
        assert!(has_prompt);
        let prompt = {
            let rooms = state.rooms.lock().await;
            rooms
                .get(&room_code)
                .and_then(|r| r.players.get(&pid))
                .expect("player exists")
                .prompt
                .clone()
        };
        assert!(prompt.starts_with("math-"));

        handle_submission(&state, &room_code, pid, prompt).await;
        let rooms = state.rooms.lock().await;
        let player = rooms
            .get(&room_code)
            .and_then(|room| room.players.get(&pid))
            .expect("player exists");
        assert_eq!(player.size, DEFAULT_START_SIZE + 9.0);
    }

    #[tokio::test]
    async fn double_points_doubles_growth() {
        use crate::powerup::PowerUpKind;

        let state = test_state();
        let (sender, _) = mpsc::unbounded_channel::<Message>();
        let (room_code, _token, pid) = join_or_create_room(
            &state,
            None,
            Some("arithmetic".to_string()),
            None,
            None,
            None,
            None,
            sender,
        )
        .await
        .expect("room created");

        handle_start_match(&state, &room_code, pid).await;

        {
            let mut rooms = state.rooms.lock().await;
            let room = rooms.get_mut(&room_code).expect("room exists");
            room.active_powerups.push(ActivePowerUp {
                kind: PowerUpKind::DoublePoints,
                source_player_id: pid,
                expires_at: Instant::now() + Duration::from_secs(30),
                duration: Duration::from_secs(30),
            });
        }

        let prompt = {
            let rooms = state.rooms.lock().await;
            rooms
                .get(&room_code)
                .and_then(|r| r.players.get(&pid))
                .expect("player exists")
                .prompt
                .clone()
        };
        handle_submission(&state, &room_code, pid, prompt).await;

        let rooms = state.rooms.lock().await;
        let player = rooms
            .get(&room_code)
            .and_then(|r| r.players.get(&pid))
            .expect("player exists");
        assert_eq!(
            player.size,
            DEFAULT_START_SIZE + 9.0 * 2.0,
            "growth should be doubled with double-points active"
        );
    }

    #[tokio::test]
    async fn winning_round_earns_pending_powerup() {
        use crate::powerup::PowerUpKind;

        let state = test_state();
        let (sender, _) = mpsc::unbounded_channel::<Message>();
        let (room_code, _token, pid) =
            join_or_create_room(&state, None, None, None, None, None, None, sender)
                .await
                .expect("room created");

        handle_start_match(&state, &room_code, pid).await;

        {
            let mut rooms = state.rooms.lock().await;
            let room = rooms.get_mut(&room_code).expect("room exists");
            let offer_id = room.next_offer_id;
            room.next_offer_id += 1;
            room.powerup_offers.push(PowerUpOffer {
                offer_id,
                kind: PowerUpKind::DoublePoints,
                player_id: pid,
                expires_at: Instant::now() + Duration::from_secs(30),
            });
        }

        let prompt = {
            let rooms = state.rooms.lock().await;
            rooms
                .get(&room_code)
                .and_then(|r| r.players.get(&pid))
                .expect("player exists")
                .prompt
                .clone()
        };
        handle_submission(&state, &room_code, pid, prompt).await;

        let rooms = state.rooms.lock().await;
        let room = rooms.get(&room_code).expect("room exists");
        assert!(room.powerup_offers.is_empty(), "offer should be consumed");
        assert_eq!(room.active_powerups.len(), 1, "one active power-up");
        assert_eq!(room.active_powerups[0].kind, PowerUpKind::DoublePoints);
        assert_eq!(room.active_powerups[0].source_player_id, pid);
    }

    #[tokio::test]
    async fn host_can_update_room_settings() {
        let state = test_state();
        let (sender, _) = mpsc::unbounded_channel::<Message>();
        let (host_sender, _) = mpsc::unbounded_channel::<Message>();
        let (room_code, _token, host_pid) = join_or_create_room(
            &state,
            None,
            Some("keyboarding".to_string()),
            None,
            None,
            None,
            None,
            sender,
        )
        .await
        .expect("room created");

        let initial_generation = {
            let rooms = state.rooms.lock().await;
            rooms.get(&room_code).expect("room exists").match_generation
        };

        handle_update_room_settings(
            &state,
            &room_code,
            host_pid,
            Some("arithmetic".to_string()),
            Some(120),
            Some(serde_json::json!({"operation": "addition"})),
            &host_sender,
        )
        .await;

        let rooms = state.rooms.lock().await;
        let room = rooms.get(&room_code).expect("room exists");
        assert_eq!(room.game_key, "arithmetic");
        assert_eq!(room.match_duration_secs, 120);
        assert_eq!(
            room.game_options,
            serde_json::json!({"operation": "addition"})
        );
        assert!(room.match_deadline.is_none());
        assert!(room.match_winner.is_none());
        assert!(room.match_generation > initial_generation);
    }

    #[tokio::test]
    async fn non_host_cannot_update_room_settings() {
        let state = test_state();
        let (host_sender, _) = mpsc::unbounded_channel::<Message>();
        let (joiner_sender, _) = mpsc::unbounded_channel::<Message>();
        let (joiner_reply, mut joiner_rx) = mpsc::unbounded_channel::<Message>();

        let (room_code, _token, _host_pid) = join_or_create_room(
            &state,
            None,
            Some("keyboarding".to_string()),
            None,
            None,
            None,
            None,
            host_sender,
        )
        .await
        .expect("room created");
        let (_, _, joiner_pid) = join_or_create_room(
            &state,
            Some(room_code.clone()),
            None,
            None,
            None,
            None,
            None,
            joiner_sender,
        )
        .await
        .expect("joined");

        handle_update_room_settings(
            &state,
            &room_code,
            joiner_pid,
            Some("arithmetic".to_string()),
            None,
            None,
            &joiner_reply,
        )
        .await;

        let received = joiner_rx.try_recv().expect("error message sent");
        let Message::Text(text) = received else {
            panic!("expected text message");
        };
        assert!(text.contains("Only the host"));

        let rooms = state.rooms.lock().await;
        let room = rooms.get(&room_code).expect("room exists");
        assert_eq!(room.game_key, "keyboarding");
    }

    #[tokio::test]
    async fn update_room_settings_rejects_unknown_game_mode() {
        let state = test_state();
        let (sender, _) = mpsc::unbounded_channel::<Message>();
        let (reply_tx, mut reply_rx) = mpsc::unbounded_channel::<Message>();
        let (room_code, _token, host_pid) = join_or_create_room(
            &state,
            None,
            Some("keyboarding".to_string()),
            None,
            None,
            None,
            None,
            sender,
        )
        .await
        .expect("room created");

        handle_update_room_settings(
            &state,
            &room_code,
            host_pid,
            Some("does-not-exist".to_string()),
            None,
            None,
            &reply_tx,
        )
        .await;

        let received = reply_rx.try_recv().expect("error message sent");
        let Message::Text(text) = received else {
            panic!("expected text message");
        };
        assert!(text.contains("invalidGameMode"));

        let rooms = state.rooms.lock().await;
        let room = rooms.get(&room_code).expect("room exists");
        assert_eq!(room.game_key, "keyboarding");
    }

    #[tokio::test]
    async fn update_room_settings_resets_in_progress_match_to_lobby() {
        let state = test_state();
        let (sender, _) = mpsc::unbounded_channel::<Message>();
        let (host_sender, _) = mpsc::unbounded_channel::<Message>();
        let (room_code, _token, host_pid) = join_or_create_room(
            &state,
            None,
            Some("arithmetic".to_string()),
            None,
            None,
            None,
            None,
            sender,
        )
        .await
        .expect("room created");

        handle_start_match(&state, &room_code, host_pid).await;
        let started_generation = {
            let rooms = state.rooms.lock().await;
            let room = rooms.get(&room_code).expect("room exists");
            assert!(room.match_deadline.is_some());
            room.match_generation
        };

        handle_update_room_settings(
            &state,
            &room_code,
            host_pid,
            Some("keyboarding".to_string()),
            None,
            None,
            &host_sender,
        )
        .await;

        let rooms = state.rooms.lock().await;
        let room = rooms.get(&room_code).expect("room exists");
        assert_eq!(room.game_key, "keyboarding");
        assert!(room.match_deadline.is_none());
        assert!(room.match_winner.is_none());
        assert!(room.match_generation > started_generation);
        for player in room.players.values() {
            assert_eq!(player.size, DEFAULT_START_SIZE);
            assert!(player.prompt.is_empty());
        }
    }

    #[test]
    fn validate_player_name_accepts_predefined_pairs() {
        assert!(validate_player_name("Brave Panda"));
        assert!(validate_player_name("Zippy Zebra"));
        assert!(validate_player_name("Gentle Otter"));
    }

    #[test]
    fn validate_player_color_accepts_palette() {
        for color in PALETTE {
            assert!(validate_player_color(color));
        }
    }

    /// Coverage for the adversarial scenario from the feature spec: a malicious
    /// client crafts a `JoinOrCreateRoom` payload containing a hand-picked name
    /// or color that isn't in the predefined sets. The server must silently
    /// fall back to the generated default — never honor the spoofed value, and
    /// never error out (so the attack is indistinguishable from a normal join
    /// where the user didn't pick anything).
    ///
    /// Two layers of assertions for every malicious input:
    ///   1. `validate_player_*` rejects it.
    ///   2. `join_or_create_room` produces a `PlayerState` whose name/color
    ///      passes the validator and is not the spoofed value.
    ///
    /// The second layer is what really matters — (1) could be correct while
    /// (2) is broken if the wiring around the validator regresses.
    mod adversarial {
        use super::*;

        // Each tuple: (malicious input, scenario label used in failure messages).
        const ADVERSARIAL_NAMES: &[(&str, &str)] = &[
            // Free-text names not in the predefined sets.
            ("L33T H4XOR", "leet-speak free-text"),
            ("Sneaky Hacker", "free-text words not in either set"),
            ("My Custom Name", "arbitrary free-text"),
            ("Admin Mod", "social-engineering name"),
            // Wrong shape — not exactly two space-separated words.
            ("Brave", "single word, missing noun"),
            ("Panda", "single word, just noun"),
            ("", "empty string"),
            (" ", "single space"),
            ("  ", "two spaces only"),
            ("Brave Panda Extra", "three words"),
            ("Brave Panda Extra Word", "four words"),
            // Case mutations — the predefined sets are case-sensitive.
            ("brave panda", "all lowercase"),
            ("BRAVE PANDA", "all uppercase"),
            ("Brave PANDA", "noun all uppercase"),
            ("brave Panda", "adjective lowercase"),
            ("BraVe PaNda", "alternating case"),
            // Whitespace tricks around or between the words.
            (" Brave Panda", "leading space"),
            ("Brave Panda ", "trailing space"),
            ("Brave  Panda", "two spaces between"),
            ("\tBrave Panda", "leading tab"),
            ("Brave Panda\n", "trailing newline"),
            ("Brave\tPanda", "tab as separator"),
            (
                "Brave\u{00a0}Panda",
                "non-breaking space (U+00A0) as separator",
            ),
            // Cross-set: real words used in the wrong slot.
            ("Panda Brave", "noun-then-adjective swap"),
            ("Brave Brave", "adjective in noun slot"),
            ("Panda Panda", "noun in adjective slot"),
            // Near-misses (one character off a real word).
            ("Bravex Panda", "adjective with extra trailing letter"),
            ("Brave Pandaa", "noun with extra trailing letter"),
            ("Brav Panda", "adjective truncated"),
            ("Brave Pand", "noun truncated"),
            // Homoglyph attacks — Cyrillic 'а' (U+0430) mimicking Latin 'a'.
            ("Br\u{0430}ve Panda", "Cyrillic homoglyph in adjective"),
            ("Brave Pand\u{0430}", "Cyrillic homoglyph in noun"),
            // Punctuation / symbols instead of a space separator.
            ("Brave-Panda", "hyphen separator"),
            ("Brave_Panda", "underscore separator"),
            ("Brave.Panda", "period separator"),
            ("Brave/Panda", "slash separator"),
            // Plausibly hostile string-length / control content.
            ("Brave\0Panda", "NUL byte separator"),
            ("Brave\nPanda", "newline separator"),
        ];

        const ADVERSARIAL_COLORS: &[(&str, &str)] = &[
            // Hex outside the palette.
            ("#000000", "black"),
            ("#ffffff", "white"),
            ("#ff0000", "red, outside palette"),
            ("#123456", "arbitrary hex"),
            // Case mutations — palette values are lowercase only.
            ("#38BDF8", "uppercase hex of a palette color"),
            ("#38Bdf8", "mixed-case hex of a palette color"),
            // Wrong format.
            ("38bdf8", "missing # prefix"),
            ("rgb(56, 189, 248)", "rgb() form"),
            ("rgba(56, 189, 248, 1)", "rgba() form"),
            ("hsl(199, 92%, 60%)", "hsl() form"),
            ("transparent", "named CSS keyword"),
            ("red", "named CSS keyword"),
            // Wrong length.
            ("#38bdf8aa", "palette hex with alpha appended"),
            ("#38bdf80", "palette hex with extra trailing char"),
            ("#38bd", "truncated hex"),
            ("#3", "single-char hex"),
            // Empty / whitespace.
            ("", "empty string"),
            (" ", "single space"),
            ("#38bdf8 ", "palette hex with trailing space"),
            (" #38bdf8", "palette hex with leading space"),
            ("\t#38bdf8", "palette hex with leading tab"),
        ];

        // ----- Validator-level (pure) -----

        #[test]
        fn validator_rejects_every_adversarial_name() {
            for (input, scenario) in ADVERSARIAL_NAMES {
                assert!(
                    !validate_player_name(input),
                    "validator accepted adversarial name {input:?} ({scenario})",
                );
            }
        }

        #[test]
        fn validator_rejects_every_adversarial_color() {
            for (input, scenario) in ADVERSARIAL_COLORS {
                assert!(
                    !validate_player_color(input),
                    "validator accepted adversarial color {input:?} ({scenario})",
                );
            }
        }

        // ----- End-to-end via join_or_create_room -----

        /// Joins a fresh room with the given (optional) requested name/color
        /// and returns the resulting player's stored name and color. This is
        /// what the client would observe after a `welcome` + `roomState`
        /// exchange — i.e. the values the server actually committed.
        async fn join_with(
            state: &Arc<SharedState>,
            requested_name: Option<&str>,
            requested_color: Option<&str>,
        ) -> (String, String) {
            let (sender, _) = mpsc::unbounded_channel::<Message>();
            let (room_code, _, pid) = join_or_create_room(
                state,
                None,
                None,
                None,
                None,
                requested_name.map(str::to_string),
                requested_color.map(str::to_string),
                sender,
            )
            .await
            .expect("room created");
            let rooms = state.rooms.lock().await;
            let player = rooms
                .get(&room_code)
                .expect("room exists")
                .players
                .get(&pid)
                .expect("player exists");
            (player.name.clone(), player.color.clone())
        }

        #[tokio::test]
        async fn join_with_adversarial_name_falls_back_to_generated() {
            let state = test_state();
            for (input, scenario) in ADVERSARIAL_NAMES {
                let (name, _) = join_with(&state, Some(input), None).await;
                assert_ne!(
                    name, *input,
                    "server honored adversarial name {input:?} ({scenario})",
                );
                assert!(
                    validate_player_name(&name),
                    "fallback name {name:?} for adversarial input {input:?} \
                     ({scenario}) does not pass validator",
                );
            }
        }

        #[tokio::test]
        async fn join_with_adversarial_color_falls_back_to_generated() {
            let state = test_state();
            for (input, scenario) in ADVERSARIAL_COLORS {
                let (_, color) = join_with(&state, None, Some(input)).await;
                assert_ne!(
                    color, *input,
                    "server honored adversarial color {input:?} ({scenario})",
                );
                assert!(
                    validate_player_color(&color),
                    "fallback color {color:?} for adversarial input {input:?} \
                     ({scenario}) is not in palette",
                );
            }
        }

        /// An attacker who sends *both* fields adversarially still gets
        /// nothing they asked for.
        #[tokio::test]
        async fn join_with_both_fields_adversarial_falls_back_independently() {
            let state = test_state();
            for ((bad_name, name_scenario), (bad_color, color_scenario)) in ADVERSARIAL_NAMES
                .iter()
                .zip(ADVERSARIAL_COLORS.iter().cycle())
            {
                let (name, color) = join_with(&state, Some(bad_name), Some(bad_color)).await;
                assert_ne!(
                    name, *bad_name,
                    "server honored adversarial name {bad_name:?} ({name_scenario}) \
                     when paired with color {bad_color:?}",
                );
                assert_ne!(
                    color, *bad_color,
                    "server honored adversarial color {bad_color:?} ({color_scenario}) \
                     when paired with name {bad_name:?}",
                );
                assert!(validate_player_name(&name));
                assert!(validate_player_color(&color));
            }
        }

        /// Sanity: a payload that picks a real predefined pair *is* honored,
        /// so the fallback isn't just unconditional rejection.
        #[tokio::test]
        async fn join_with_valid_name_and_color_is_honored() {
            let state = test_state();
            let (name, color) = join_with(&state, Some("Brave Panda"), Some("#38bdf8")).await;
            assert_eq!(name, "Brave Panda");
            assert_eq!(color, "#38bdf8");
        }

        /// Validation happens per-field — a valid pick on one side is not
        /// dropped just because the other side is adversarial.
        #[tokio::test]
        async fn invalid_field_does_not_drag_down_the_valid_one() {
            let state = test_state();

            // Valid name + adversarial color → name kept, color regenerated.
            let (name, color) = join_with(&state, Some("Gentle Otter"), Some("#000000")).await;
            assert_eq!(name, "Gentle Otter");
            assert_ne!(color, "#000000");
            assert!(validate_player_color(&color));

            // Adversarial name + valid color → color kept, name regenerated.
            let (name, color) = join_with(&state, Some("Hax0r Pwnd"), Some("#a78bfa")).await;
            assert_ne!(name, "Hax0r Pwnd");
            assert!(validate_player_name(&name));
            assert_eq!(color, "#a78bfa");
        }

        /// Sanity: when the client doesn't request anything at all, the
        /// server-generated defaults still pass the validators. This keeps
        /// the validator contract honest — `generate_player_*` must produce
        /// values that `validate_player_*` accepts, otherwise the fallback
        /// path itself would be a vulnerability.
        #[tokio::test]
        async fn server_defaults_always_satisfy_the_validator() {
            let state = test_state();
            for _ in 0..32 {
                let (name, color) = join_with(&state, None, None).await;
                assert!(
                    validate_player_name(&name),
                    "server-generated name {name:?} does not pass validator",
                );
                assert!(
                    validate_player_color(&color),
                    "server-generated color {color:?} does not pass validator",
                );
            }
        }
    }
}
