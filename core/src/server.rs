use crate::adapter::{AdapterHandle, AdapterRegistry, OptionField, build_adapter_registry};
use crate::game::{
    DEFAULT_START_SIZE, FREEZE_ESCAPE_REQUIRED, FreezeEscapeState, PlayerId, PlayerState,
    RoomState, apply_round_win, apply_wrong_answer_penalty, deduct_from_top_players,
    find_top_player_ids, resolve_match_by_timer,
};
use crate::powerup::{
    ActivePowerUp, PowerUpKind, PowerUpOffer, cleanup_expired, distribution_interval,
    effect_duration, has_double_points, has_ongoing_score_steal, is_player_frozen, offer_duration,
    pick_powerup_kind, pick_powerup_recipient, remove_player_from_freezes,
};
use crate::protocol::{ClientMessage, ErrorCode, ServerMessage};
use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use rand::distr::Alphanumeric;
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

pub async fn run_server(adapters: Vec<AdapterHandle>, config: ServerConfig) -> Result<(), String> {
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
        config: config.clone(),
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

    let app = Router::new()
        .route("/healthz", get(health_handler))
        .route("/readyz", get(health_handler))
        .route("/api/game-modes", get(game_modes_handler))
        .route("/ws", get(ws_handler))
        .with_state(state);

    let listener = TcpListener::bind(&config.bind_addr)
        .await
        .map_err(|e| format!("failed to bind {}: {e}", config.bind_addr))?;

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

                    if room.prompt.is_empty() {
                        None
                    } else {
                        Some((room.round_id, room.prompt.clone()))
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

async fn join_or_create_room(
    state: &Arc<SharedState>,
    requested_room_code: Option<String>,
    requested_game_mode: Option<String>,
    requested_match_duration_secs: Option<u64>,
    requested_game_options: Option<serde_json::Value>,
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
                    prompt: String::new(),
                    round_id: 0,
                    match_winner: None,
                    match_deadline: None,
                    match_duration_secs: duration,
                    host_player_id: 1,
                    next_player_id: 1,
                    powerup_offers: Vec::new(),
                    active_powerups: Vec::new(),
                    next_offer_id: 0,
                    freeze_escape: HashMap::new(),
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
            name: generate_player_name(&mut rand::rng()),
            size: DEFAULT_START_SIZE,
            color: generate_color(player_id),
            connected: true,
            progress: String::new(),
            rejoin_token: token.clone(),
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

    let frozen;
    {
        let mut rooms = state.rooms.lock().await;
        let Some(room) = rooms.get_mut(room_code) else {
            return;
        };
        frozen = is_player_frozen(&room.active_powerups, player_id);
        let Some(player) = room.players.get_mut(&player_id) else {
            return;
        };
        player.progress = normalized.clone();
    }

    if frozen {
        return;
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

    let frozen = {
        let rooms = state.rooms.lock().await;
        let Some(room) = rooms.get(room_code) else {
            return;
        };
        if room.match_winner.is_some() || room.prompt.is_empty() {
            return;
        }
        is_player_frozen(&room.active_powerups, player_id)
    };

    if frozen {
        handle_freeze_escape(state, room_code, player_id, text, &adapter).await;
        return;
    }

    let mut should_advance_round = false;
    let mut round_result: Option<ServerMessage> = None;
    let mut wrong_answer_msg: Option<ServerMessage> = None;
    let mut earned_powerups: Vec<ServerMessage> = Vec::new();
    let mut freeze_escape_msgs: Vec<(PlayerId, ServerMessage)> = Vec::new();

    {
        let mut rooms = state.rooms.lock().await;
        let Some(room) = rooms.get_mut(room_code) else {
            return;
        };

        if room.match_winner.is_some() || room.prompt.is_empty() {
            return;
        }

        if !adapter.is_correct(&room.prompt, &text) {
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
                .score_for_prompt(&room.prompt)
                .max(configured_growth);

            if has_double_points(&room.active_powerups, player_id) {
                growth *= 2.0;
            }

            if let Some(resolution) = apply_round_win(room, player_id, growth) {
                round_result = Some(ServerMessage::RoundResult {
                    room_code: room_code.to_string(),
                    round_id: room.round_id,
                    winner_player_id: resolution.round_winner,
                    growth_awarded: growth,
                });
                should_advance_round = room.match_winner.is_none();

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
                    let targets = if offer.kind == PowerUpKind::FreezeAllCompetitors {
                        find_top_player_ids(&room.players, player_id)
                    } else {
                        vec![]
                    };
                    room.active_powerups.push(ActivePowerUp {
                        kind: offer.kind,
                        source_player_id: player_id,
                        expires_at: now + duration,
                        duration,
                        target_player_ids: targets.clone(),
                    });
                    earned_powerups.push(ServerMessage::PowerUpActivated {
                        offer_id: offer.offer_id,
                        player_id,
                        kind: offer.kind,
                        duration_ms: duration.as_millis() as u64,
                    });

                    if offer.kind == PowerUpKind::FreezeAllCompetitors {
                        for &target_id in &targets {
                            let escape_seed = state.prompt_seed.fetch_add(1, Ordering::Relaxed);
                            let seed = splitmix64(escape_seed);
                            let escape_prompt = adapter.next_prompt(seed, &room.game_options);
                            room.freeze_escape.insert(
                                target_id,
                                FreezeEscapeState {
                                    prompt: escape_prompt.clone(),
                                    correct_streak: 0,
                                },
                            );
                            freeze_escape_msgs.push((
                                target_id,
                                ServerMessage::FreezeEscapeState {
                                    room_code: room_code.to_string(),
                                    player_id: target_id,
                                    prompt: escape_prompt,
                                    streak: 0,
                                    required: FREEZE_ESCAPE_REQUIRED,
                                },
                            ));
                        }
                    }

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

    for (target_id, msg) in freeze_escape_msgs {
        let _ = send_to_player(state, room_code, target_id, &msg).await;
    }

    if should_advance_round {
        let _ = ensure_prompt_for_room(state, room_code).await;
    }
}

async fn handle_freeze_escape(
    state: &Arc<SharedState>,
    room_code: &str,
    player_id: PlayerId,
    text: String,
    adapter: &AdapterHandle,
) {
    let escape_msg;
    let mut escaped = false;

    {
        let mut rooms = state.rooms.lock().await;
        let Some(room) = rooms.get_mut(room_code) else {
            return;
        };

        let Some(esc) = room.freeze_escape.get_mut(&player_id) else {
            return;
        };

        if adapter.is_correct(&esc.prompt, &text) {
            esc.correct_streak += 1;
            if esc.correct_streak >= FREEZE_ESCAPE_REQUIRED {
                escaped = true;
                room.freeze_escape.remove(&player_id);
                remove_player_from_freezes(&mut room.active_powerups, player_id);
                escape_msg = None;
            } else {
                let raw_seed = state.prompt_seed.fetch_add(1, Ordering::Relaxed);
                let seed = splitmix64(raw_seed);
                let new_prompt = adapter.next_prompt(seed, &room.game_options);
                let streak = esc.correct_streak;
                esc.prompt = new_prompt.clone();
                escape_msg = Some(ServerMessage::FreezeEscapeState {
                    room_code: room_code.to_string(),
                    player_id,
                    prompt: new_prompt,
                    streak,
                    required: FREEZE_ESCAPE_REQUIRED,
                });
            }
        } else {
            esc.correct_streak = 0;
            let raw_seed = state.prompt_seed.fetch_add(1, Ordering::Relaxed);
            let seed = splitmix64(raw_seed);
            let new_prompt = adapter.next_prompt(seed, &room.game_options);
            esc.prompt = new_prompt.clone();
            escape_msg = Some(ServerMessage::FreezeEscapeState {
                room_code: room_code.to_string(),
                player_id,
                prompt: new_prompt,
                streak: 0,
                required: FREEZE_ESCAPE_REQUIRED,
            });
        }
    }

    if escaped {
        let _ = broadcast_room_state(state, room_code).await;
    }

    if let Some(msg) = escape_msg {
        let _ = send_to_player(state, room_code, player_id, &msg).await;
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

    start_match_timer(state.clone(), room_code.to_string(), duration_secs);
    start_powerup_timer(state.clone(), room_code.to_string(), duration_secs);

    let _ = broadcast_room_state(state, room_code).await;
    let _ = ensure_prompt_for_room(state, room_code).await;
}

async fn handle_rematch(state: &Arc<SharedState>, room_code: &str) {
    let duration_secs;
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
        let deadline = Instant::now() + Duration::from_secs(duration_secs);
        room.match_deadline = Some(deadline);
    }

    start_match_timer(state.clone(), room_code.to_string(), duration_secs);
    start_powerup_timer(state.clone(), room_code.to_string(), duration_secs);

    let _ = broadcast_room_state(state, room_code).await;
    let _ = ensure_prompt_for_room(state, room_code).await;
}

fn splitmix64(val: u64) -> u64 {
    let mut z = val.wrapping_add(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

async fn ensure_prompt_for_room(state: &Arc<SharedState>, room_code: &str) -> bool {
    let Some(adapter) = adapter_for_room(state, room_code).await else {
        return false;
    };
    let prompt_update;
    {
        let mut rooms = state.rooms.lock().await;
        let Some(room) = rooms.get_mut(room_code) else {
            return false;
        };
        if room.match_winner.is_some() || room.players.is_empty() || room.match_deadline.is_none() {
            return false;
        }
        let raw_seed = state.prompt_seed.fetch_add(1, Ordering::Relaxed);
        let seed = splitmix64(raw_seed);
        room.round_id += 1;
        room.prompt = adapter.next_prompt(seed, &room.game_options);
        for player in room.players.values_mut() {
            player.progress.clear();
        }
        prompt_update = (room.round_id, room.prompt.clone());
    }

    let (round_id, prompt) = prompt_update;
    let _ = broadcast_to_room(
        state,
        room_code,
        &ServerMessage::PromptState {
            room_code: room_code.to_string(),
            round_id,
            prompt,
        },
    )
    .await;

    true
}

fn start_match_timer(state: Arc<SharedState>, room_code: String, duration_secs: u64) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(duration_secs)).await;
        {
            let mut rooms = state.rooms.lock().await;
            let Some(room) = rooms.get_mut(&room_code) else {
                return;
            };
            resolve_match_by_timer(room);
        }
        let _ = broadcast_room_state(&state, &room_code).await;
    });
}

fn start_powerup_timer(state: Arc<SharedState>, room_code: String, match_duration_secs: u64) {
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
                    if effect.kind == PowerUpKind::FreezeAllCompetitors {
                        for &target_id in &effect.target_player_ids {
                            room.freeze_escape.remove(&target_id);
                        }
                    }
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

async fn broadcast_room_state(state: &Arc<SharedState>, room_code: &str) -> bool {
    let snapshot = {
        let rooms = state.rooms.lock().await;
        let Some(room) = rooms.get(room_code) else {
            return false;
        };
        room.to_snapshot()
    };

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

async fn send_to_player(
    state: &Arc<SharedState>,
    room_code: &str,
    player_id: PlayerId,
    message: &ServerMessage,
) -> bool {
    let connections = state.connections.lock().await;
    if let Some(room_conns) = connections.get(room_code) {
        if let Some(conn) = room_conns.get(&player_id) {
            return send_server_message(&conn.sender, message).is_ok();
        }
    }
    false
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
    const ADJECTIVES: &[&str] = &[
        "Brave", "Clever", "Cosmic", "Daring", "Dizzy", "Eager", "Fancy", "Fizzy", "Fluffy",
        "Funky", "Gentle", "Giddy", "Glossy", "Golden", "Happy", "Hasty", "Jazzy", "Jolly",
        "Lucky", "Mega", "Mighty", "Misty", "Nifty", "Noble", "Peppy", "Plucky", "Polar", "Quick",
        "Rapid", "Rocky", "Royal", "Rusty", "Sandy", "Shiny", "Silly", "Sleek", "Snappy", "Solar",
        "Speedy", "Spicy", "Super", "Swift", "Tiny", "Turbo", "Vivid", "Wacky", "Wild", "Witty",
        "Zappy", "Zippy",
    ];
    const NOUNS: &[&str] = &[
        "Badger", "Banana", "Beetle", "Bison", "Bobcat", "Bunny", "Cactus", "Cloud", "Comet",
        "Cookie", "Corgi", "Dingo", "Dragon", "Eagle", "Falcon", "Ferret", "Fox", "Gecko",
        "Gopher", "Hippo", "Igloo", "Jackal", "Koala", "Lemon", "Llama", "Mango", "Moose",
        "Narwhal", "Newt", "Otter", "Owl", "Panda", "Parrot", "Peach", "Penguin", "Pickle",
        "Puffin", "Quokka", "Raven", "Rocket", "Sloth", "Squid", "Taco", "Tiger", "Toucan",
        "Turtle", "Waffle", "Walrus", "Yeti", "Zebra",
    ];
    let adj = ADJECTIVES[rng.random_range(0..ADJECTIVES.len())];
    let noun = NOUNS[rng.random_range(0..NOUNS.len())];
    format!("{adj} {noun}")
}

fn generate_color(player_id: PlayerId) -> String {
    let palette = [
        "#38bdf8", "#a78bfa", "#34d399", "#f472b6", "#fbbf24", "#fb7185", "#22d3ee",
    ];
    let idx = (player_id as usize) % palette.len();
    palette[idx].to_string()
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
            sender,
        )
        .await
        .expect("room created");

        handle_start_match(&state, &room_code, pid).await;
        let has_prompt = {
            let rooms = state.rooms.lock().await;
            let room = rooms.get(&room_code).expect("room exists");
            assert!(room.match_deadline.is_some());
            !room.prompt.is_empty()
        };
        assert!(has_prompt);
        let prompt = {
            let rooms = state.rooms.lock().await;
            rooms.get(&room_code).expect("room exists").prompt.clone()
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
    async fn frozen_player_cannot_submit() {
        use crate::powerup::PowerUpKind;

        let state = test_state();
        let (sender, _) = mpsc::unbounded_channel::<Message>();
        let (room_code, _token, pid) = join_or_create_room(&state, None, None, None, None, sender)
            .await
            .expect("room created");

        handle_start_match(&state, &room_code, pid).await;

        {
            let mut rooms = state.rooms.lock().await;
            let room = rooms.get_mut(&room_code).expect("room exists");
            room.active_powerups.push(ActivePowerUp {
                kind: PowerUpKind::FreezeAllCompetitors,
                source_player_id: 999,
                expires_at: Instant::now() + Duration::from_secs(15),
                duration: Duration::from_secs(15),
                target_player_ids: vec![pid],
            });
        }

        let prompt = {
            let rooms = state.rooms.lock().await;
            rooms.get(&room_code).expect("room exists").prompt.clone()
        };
        handle_submission(&state, &room_code, pid, prompt).await;

        let rooms = state.rooms.lock().await;
        let player = rooms
            .get(&room_code)
            .and_then(|r| r.players.get(&pid))
            .expect("player exists");
        assert_eq!(
            player.size, DEFAULT_START_SIZE,
            "frozen player should not gain points"
        );
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
                target_player_ids: vec![],
            });
        }

        let prompt = {
            let rooms = state.rooms.lock().await;
            rooms.get(&room_code).expect("room exists").prompt.clone()
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
        let (room_code, _token, pid) = join_or_create_room(&state, None, None, None, None, sender)
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
                kind: PowerUpKind::FreezeAllCompetitors,
                player_id: pid,
                expires_at: Instant::now() + Duration::from_secs(30),
            });
        }

        let prompt = {
            let rooms = state.rooms.lock().await;
            rooms.get(&room_code).expect("room exists").prompt.clone()
        };
        handle_submission(&state, &room_code, pid, prompt).await;

        let rooms = state.rooms.lock().await;
        let room = rooms.get(&room_code).expect("room exists");
        assert!(room.powerup_offers.is_empty(), "offer should be consumed");
        assert_eq!(room.active_powerups.len(), 1, "one active power-up");
        assert_eq!(
            room.active_powerups[0].kind,
            PowerUpKind::FreezeAllCompetitors
        );
        assert_eq!(room.active_powerups[0].source_player_id, pid);
    }

    #[tokio::test]
    async fn freeze_only_targets_leader() {
        use crate::powerup::PowerUpKind;

        let state = test_state();
        let (s1, _) = mpsc::unbounded_channel::<Message>();
        let (s2, _) = mpsc::unbounded_channel::<Message>();
        let (s3, _) = mpsc::unbounded_channel::<Message>();
        let (room_code, _, p1) = join_or_create_room(&state, None, None, None, None, s1)
            .await
            .expect("room");
        let (_, _, p2) = join_or_create_room(&state, Some(room_code.clone()), None, None, None, s2)
            .await
            .expect("room");
        let (_, _, p3) = join_or_create_room(&state, Some(room_code.clone()), None, None, None, s3)
            .await
            .expect("room");

        handle_start_match(&state, &room_code, p1).await;

        {
            let mut rooms = state.rooms.lock().await;
            let room = rooms.get_mut(&room_code).expect("room");
            room.players.get_mut(&p1).unwrap().size = 30.0;
            room.players.get_mut(&p2).unwrap().size = 20.0;
            room.players.get_mut(&p3).unwrap().size = 10.0;
            let offer_id = room.next_offer_id;
            room.next_offer_id += 1;
            room.powerup_offers.push(PowerUpOffer {
                offer_id,
                kind: PowerUpKind::FreezeAllCompetitors,
                player_id: p3,
                expires_at: Instant::now() + Duration::from_secs(30),
            });
        }

        let prompt = {
            let rooms = state.rooms.lock().await;
            rooms.get(&room_code).expect("room").prompt.clone()
        };
        handle_submission(&state, &room_code, p3, prompt).await;

        let rooms = state.rooms.lock().await;
        let room = rooms.get(&room_code).expect("room");
        assert_eq!(room.active_powerups.len(), 1);
        let freeze = &room.active_powerups[0];
        assert_eq!(freeze.target_player_ids, vec![p1]);
        assert!(is_player_frozen(&room.active_powerups, p1));
        assert!(!is_player_frozen(&room.active_powerups, p2));
        assert!(!is_player_frozen(&room.active_powerups, p3));
    }

    #[tokio::test]
    async fn freeze_targets_tied_leaders() {
        use crate::powerup::PowerUpKind;

        let state = test_state();
        let (s1, _) = mpsc::unbounded_channel::<Message>();
        let (s2, _) = mpsc::unbounded_channel::<Message>();
        let (s3, _) = mpsc::unbounded_channel::<Message>();
        let (room_code, _, p1) = join_or_create_room(&state, None, None, None, None, s1)
            .await
            .expect("room");
        let (_, _, p2) = join_or_create_room(&state, Some(room_code.clone()), None, None, None, s2)
            .await
            .expect("room");
        let (_, _, p3) = join_or_create_room(&state, Some(room_code.clone()), None, None, None, s3)
            .await
            .expect("room");

        handle_start_match(&state, &room_code, p1).await;

        {
            let mut rooms = state.rooms.lock().await;
            let room = rooms.get_mut(&room_code).expect("room");
            room.players.get_mut(&p1).unwrap().size = 25.0;
            room.players.get_mut(&p2).unwrap().size = 25.0;
            room.players.get_mut(&p3).unwrap().size = 10.0;
            let offer_id = room.next_offer_id;
            room.next_offer_id += 1;
            room.powerup_offers.push(PowerUpOffer {
                offer_id,
                kind: PowerUpKind::FreezeAllCompetitors,
                player_id: p3,
                expires_at: Instant::now() + Duration::from_secs(30),
            });
        }

        let prompt = {
            let rooms = state.rooms.lock().await;
            rooms.get(&room_code).expect("room").prompt.clone()
        };
        handle_submission(&state, &room_code, p3, prompt).await;

        let rooms = state.rooms.lock().await;
        let room = rooms.get(&room_code).expect("room");
        let freeze = &room.active_powerups[0];
        let mut targets = freeze.target_player_ids.clone();
        targets.sort();
        assert_eq!(targets, vec![p1, p2]);
    }

    #[tokio::test]
    async fn freeze_escape_generates_prompts_for_targets() {
        use crate::powerup::PowerUpKind;

        let state = test_state();
        let (s1, _) = mpsc::unbounded_channel::<Message>();
        let (s2, _) = mpsc::unbounded_channel::<Message>();
        let (room_code, _, p1) = join_or_create_room(&state, None, None, None, None, s1)
            .await
            .expect("room");
        let (_, _, p2) = join_or_create_room(&state, Some(room_code.clone()), None, None, None, s2)
            .await
            .expect("room");

        handle_start_match(&state, &room_code, p1).await;

        {
            let mut rooms = state.rooms.lock().await;
            let room = rooms.get_mut(&room_code).expect("room");
            room.players.get_mut(&p1).unwrap().size = 30.0;
            let offer_id = room.next_offer_id;
            room.next_offer_id += 1;
            room.powerup_offers.push(PowerUpOffer {
                offer_id,
                kind: PowerUpKind::FreezeAllCompetitors,
                player_id: p2,
                expires_at: Instant::now() + Duration::from_secs(30),
            });
        }

        let prompt = {
            let rooms = state.rooms.lock().await;
            rooms.get(&room_code).expect("room").prompt.clone()
        };
        handle_submission(&state, &room_code, p2, prompt).await;

        let rooms = state.rooms.lock().await;
        let room = rooms.get(&room_code).expect("room");
        assert!(room.freeze_escape.contains_key(&p1));
        let esc = &room.freeze_escape[&p1];
        assert_eq!(esc.correct_streak, 0);
        assert!(!esc.prompt.is_empty());
    }

    #[tokio::test]
    async fn freeze_escape_correct_answers_increment_streak() {
        use crate::powerup::PowerUpKind;

        let state = test_state();
        let (s1, _) = mpsc::unbounded_channel::<Message>();
        let (s2, _) = mpsc::unbounded_channel::<Message>();
        let (room_code, _, p1) = join_or_create_room(&state, None, None, None, None, s1)
            .await
            .expect("room");
        let (_, _, p2) = join_or_create_room(&state, Some(room_code.clone()), None, None, None, s2)
            .await
            .expect("room");

        handle_start_match(&state, &room_code, p1).await;

        {
            let mut rooms = state.rooms.lock().await;
            let room = rooms.get_mut(&room_code).expect("room");
            room.players.get_mut(&p1).unwrap().size = 30.0;
            room.active_powerups.push(ActivePowerUp {
                kind: PowerUpKind::FreezeAllCompetitors,
                source_player_id: p2,
                expires_at: Instant::now() + Duration::from_secs(30),
                duration: Duration::from_secs(30),
                target_player_ids: vec![p1],
            });
            room.freeze_escape.insert(
                p1,
                FreezeEscapeState {
                    prompt: "kbd-99".to_string(),
                    correct_streak: 0,
                },
            );
        }

        handle_submission(&state, &room_code, p1, "kbd-99".to_string()).await;

        let rooms = state.rooms.lock().await;
        let room = rooms.get(&room_code).expect("room");
        let esc = room.freeze_escape.get(&p1).expect("escape state");
        assert_eq!(esc.correct_streak, 1);
        assert_ne!(esc.prompt, "kbd-99", "prompt should change after correct answer");

        let player = room.players.get(&p1).unwrap();
        assert_eq!(
            player.size, 30.0,
            "escape answers should not change player size"
        );
    }

    #[tokio::test]
    async fn freeze_escape_wrong_answer_resets_streak() {
        use crate::powerup::PowerUpKind;

        let state = test_state();
        let (s1, _) = mpsc::unbounded_channel::<Message>();
        let (s2, _) = mpsc::unbounded_channel::<Message>();
        let (room_code, _, p1) = join_or_create_room(&state, None, None, None, None, s1)
            .await
            .expect("room");
        let (_, _, p2) = join_or_create_room(&state, Some(room_code.clone()), None, None, None, s2)
            .await
            .expect("room");

        handle_start_match(&state, &room_code, p1).await;

        {
            let mut rooms = state.rooms.lock().await;
            let room = rooms.get_mut(&room_code).expect("room");
            room.players.get_mut(&p1).unwrap().size = 30.0;
            room.active_powerups.push(ActivePowerUp {
                kind: PowerUpKind::FreezeAllCompetitors,
                source_player_id: p2,
                expires_at: Instant::now() + Duration::from_secs(30),
                duration: Duration::from_secs(30),
                target_player_ids: vec![p1],
            });
            room.freeze_escape.insert(
                p1,
                FreezeEscapeState {
                    prompt: "kbd-99".to_string(),
                    correct_streak: 3,
                },
            );
        }

        handle_submission(&state, &room_code, p1, "wrong".to_string()).await;

        let rooms = state.rooms.lock().await;
        let room = rooms.get(&room_code).expect("room");
        let esc = room.freeze_escape.get(&p1).expect("escape state");
        assert_eq!(esc.correct_streak, 0, "streak should reset on wrong answer");
    }

    #[tokio::test]
    async fn freeze_escape_five_correct_unfreezes_player() {
        use crate::game::FREEZE_ESCAPE_REQUIRED;
        use crate::powerup::PowerUpKind;

        let state = test_state();
        let (s1, _) = mpsc::unbounded_channel::<Message>();
        let (s2, _) = mpsc::unbounded_channel::<Message>();
        let (room_code, _, p1) = join_or_create_room(&state, None, None, None, None, s1)
            .await
            .expect("room");
        let (_, _, p2) = join_or_create_room(&state, Some(room_code.clone()), None, None, None, s2)
            .await
            .expect("room");

        handle_start_match(&state, &room_code, p1).await;

        {
            let mut rooms = state.rooms.lock().await;
            let room = rooms.get_mut(&room_code).expect("room");
            room.players.get_mut(&p1).unwrap().size = 30.0;
            room.active_powerups.push(ActivePowerUp {
                kind: PowerUpKind::FreezeAllCompetitors,
                source_player_id: p2,
                expires_at: Instant::now() + Duration::from_secs(30),
                duration: Duration::from_secs(30),
                target_player_ids: vec![p1],
            });
            room.freeze_escape.insert(
                p1,
                FreezeEscapeState {
                    prompt: "kbd-1".to_string(),
                    correct_streak: FREEZE_ESCAPE_REQUIRED - 1,
                },
            );
        }

        handle_submission(&state, &room_code, p1, "kbd-1".to_string()).await;

        let rooms = state.rooms.lock().await;
        let room = rooms.get(&room_code).expect("room");
        assert!(
            !room.freeze_escape.contains_key(&p1),
            "escape state should be cleared"
        );
        assert!(
            !is_player_frozen(&room.active_powerups, p1),
            "player should no longer be frozen"
        );
        assert!(
            room.active_powerups[0].target_player_ids.is_empty(),
            "player removed from target list"
        );
    }

    #[tokio::test]
    async fn freeze_escape_non_target_not_frozen() {
        use crate::powerup::PowerUpKind;

        let state = test_state();
        let (s1, _) = mpsc::unbounded_channel::<Message>();
        let (s2, _) = mpsc::unbounded_channel::<Message>();
        let (s3, _) = mpsc::unbounded_channel::<Message>();
        let (room_code, _, p1) = join_or_create_room(&state, None, None, None, None, s1)
            .await
            .expect("room");
        let (_, _, p2) = join_or_create_room(&state, Some(room_code.clone()), None, None, None, s2)
            .await
            .expect("room");
        let (_, _, p3) = join_or_create_room(&state, Some(room_code.clone()), None, None, None, s3)
            .await
            .expect("room");

        handle_start_match(&state, &room_code, p1).await;

        {
            let mut rooms = state.rooms.lock().await;
            let room = rooms.get_mut(&room_code).expect("room");
            room.players.get_mut(&p1).unwrap().size = 30.0;
            room.players.get_mut(&p2).unwrap().size = 20.0;
            room.players.get_mut(&p3).unwrap().size = 10.0;
            room.active_powerups.push(ActivePowerUp {
                kind: PowerUpKind::FreezeAllCompetitors,
                source_player_id: p3,
                expires_at: Instant::now() + Duration::from_secs(30),
                duration: Duration::from_secs(30),
                target_player_ids: vec![p1],
            });
        }

        let prompt = {
            let rooms = state.rooms.lock().await;
            rooms.get(&room_code).expect("room").prompt.clone()
        };
        let original_size = {
            let rooms = state.rooms.lock().await;
            rooms.get(&room_code).unwrap().players.get(&p2).unwrap().size
        };

        handle_submission(&state, &room_code, p2, prompt).await;

        let rooms = state.rooms.lock().await;
        let player = rooms
            .get(&room_code)
            .unwrap()
            .players
            .get(&p2)
            .unwrap();
        assert!(
            player.size > original_size,
            "non-target player should be able to submit normally"
        );
    }
}
