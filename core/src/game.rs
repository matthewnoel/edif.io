use crate::powerup::{ActivePowerUp, ActivePowerUpSnapshot, PowerUpOffer};
use serde::Serialize;
use std::collections::HashMap;
use std::time::Instant;

pub const DEFAULT_START_SIZE: f32 = 10.0;
pub const MIN_PLAYER_SIZE: f32 = 1.0;

pub type PlayerId = u64;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSnapshot {
    pub id: PlayerId,
    pub name: String,
    pub size: f32,
    pub color: String,
    pub connected: bool,
    pub progress: String,
}

#[derive(Debug, Clone)]
pub struct PlayerState {
    pub id: PlayerId,
    pub name: String,
    pub size: f32,
    pub color: String,
    pub connected: bool,
    pub progress: String,
    pub rejoin_token: String,
}

impl PlayerState {
    pub fn to_snapshot(&self) -> PlayerSnapshot {
        PlayerSnapshot {
            id: self.id,
            name: self.name.clone(),
            size: self.size,
            color: self.color.clone(),
            connected: self.connected,
            progress: self.progress.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomSnapshot {
    pub room_code: String,
    pub players: Vec<PlayerSnapshot>,
    pub prompt: String,
    pub round_id: u64,
    pub match_winner: Option<PlayerId>,
    pub match_remaining_ms: Option<u64>,
    pub host_player_id: PlayerId,
    pub active_powerups: Vec<ActivePowerUpSnapshot>,
}

#[derive(Debug, Clone)]
pub struct RoomState {
    pub room_code: String,
    pub game_key: String,
    pub game_options: serde_json::Value,
    pub players: HashMap<PlayerId, PlayerState>,
    pub prompt: String,
    pub round_id: u64,
    pub match_winner: Option<PlayerId>,
    pub match_deadline: Option<Instant>,
    pub match_duration_secs: u64,
    pub host_player_id: PlayerId,
    pub next_player_id: u64,
    pub powerup_offers: Vec<PowerUpOffer>,
    pub active_powerups: Vec<ActivePowerUp>,
    pub next_offer_id: u64,
}

impl RoomState {
    pub fn reset_for_rematch(&mut self) {
        self.match_winner = None;
        self.match_deadline = None;
        self.prompt.clear();
        self.round_id = 0;
        self.powerup_offers.clear();
        self.active_powerups.clear();
        self.next_offer_id = 0;
        for player in self.players.values_mut() {
            player.size = DEFAULT_START_SIZE;
            player.progress.clear();
        }
    }

    pub fn to_snapshot(&self) -> RoomSnapshot {
        let mut players: Vec<PlayerSnapshot> = self
            .players
            .values()
            .map(PlayerState::to_snapshot)
            .collect();
        players.sort_by_key(|p| p.id);

        let match_remaining_ms = self.match_deadline.map(|deadline| {
            deadline
                .saturating_duration_since(Instant::now())
                .as_millis() as u64
        });

        let now = Instant::now();
        let active_powerups = self
            .active_powerups
            .iter()
            .filter(|pu| pu.expires_at > now)
            .map(ActivePowerUp::to_snapshot)
            .collect();

        RoomSnapshot {
            room_code: self.room_code.clone(),
            players,
            prompt: self.prompt.clone(),
            round_id: self.round_id,
            match_winner: self.match_winner,
            match_remaining_ms,
            host_player_id: self.host_player_id,
            active_powerups,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoundResolution {
    pub round_winner: PlayerId,
}

pub fn apply_round_win(
    room: &mut RoomState,
    winner_id: PlayerId,
    awarded_growth: f32,
) -> Option<RoundResolution> {
    let winner = room.players.get_mut(&winner_id)?;
    winner.size += awarded_growth;
    winner.progress.clear();
    Some(RoundResolution {
        round_winner: winner_id,
    })
}

pub fn apply_wrong_answer_penalty(
    room: &mut RoomState,
    player_id: PlayerId,
    penalty: f32,
) -> Option<f32> {
    let player = room.players.get_mut(&player_id)?;
    let old_size = player.size;
    player.size = (player.size - penalty).max(MIN_PLAYER_SIZE);
    player.progress.clear();
    Some(old_size - player.size)
}

pub fn find_top_player_ids(
    players: &HashMap<PlayerId, PlayerState>,
    exclude: PlayerId,
) -> Vec<PlayerId> {
    let max_size = players
        .iter()
        .filter(|(id, _)| **id != exclude)
        .map(|(_, p)| p.size)
        .fold(f32::NEG_INFINITY, f32::max);

    if max_size == f32::NEG_INFINITY {
        return Vec::new();
    }

    players
        .iter()
        .filter(|(id, p)| **id != exclude && p.size == max_size)
        .map(|(id, _)| *id)
        .collect()
}

pub fn deduct_from_top_players(
    room: &mut RoomState,
    total_deduction: f32,
    exclude: PlayerId,
) -> f32 {
    let top_ids = find_top_player_ids(&room.players, exclude);
    if top_ids.is_empty() {
        return 0.0;
    }
    let per_player = total_deduction / top_ids.len() as f32;
    let mut actual_total = 0.0;
    for id in &top_ids {
        if let Some(player) = room.players.get_mut(id) {
            let old = player.size;
            player.size = (player.size - per_player).max(MIN_PLAYER_SIZE);
            actual_total += old - player.size;
        }
    }
    actual_total
}

pub fn resolve_match_by_timer(room: &mut RoomState) {
    if room.match_winner.is_some() || room.players.is_empty() {
        return;
    }
    let winner = room
        .players
        .values()
        .max_by(|a, b| a.size.total_cmp(&b.size));
    if let Some(w) = winner {
        room.match_winner = Some(w.id);
    }
    room.prompt.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn player(id: PlayerId, size: f32) -> PlayerState {
        PlayerState {
            id,
            name: format!("p{id}"),
            size,
            color: "#ffffff".to_string(),
            connected: true,
            progress: String::new(),
            rejoin_token: String::new(),
        }
    }

    fn test_room() -> RoomState {
        RoomState {
            room_code: "ABCD".to_string(),
            game_key: "keyboarding".to_string(),
            game_options: serde_json::Value::Null,
            players: HashMap::from([(1, player(1, 10.0)), (2, player(2, 10.0))]),
            prompt: "abc".to_string(),
            round_id: 1,
            match_winner: None,
            match_deadline: None,
            match_duration_secs: 60,
            host_player_id: 1,
            next_player_id: 3,
            powerup_offers: Vec::new(),
            active_powerups: Vec::new(),
            next_offer_id: 0,
        }
    }

    #[test]
    fn apply_round_win_awards_growth() {
        let mut room = test_room();
        let resolution = apply_round_win(&mut room, 1, 5.0).expect("resolution");
        assert_eq!(resolution.round_winner, 1);
        assert_eq!(room.players.get(&1).unwrap().size, 15.0);
        assert_eq!(room.players.get(&2).unwrap().size, 10.0);
    }

    #[test]
    fn apply_round_win_does_not_remove_players() {
        let mut room = test_room();
        apply_round_win(&mut room, 1, 50.0).expect("resolution");
        assert_eq!(room.players.len(), 2);
        assert!(room.players.contains_key(&2));
    }

    #[test]
    fn apply_round_win_returns_none_for_missing_player() {
        let mut room = test_room();
        assert!(apply_round_win(&mut room, 99, 5.0).is_none());
    }

    #[test]
    fn resolve_match_by_timer_picks_largest() {
        let mut room = test_room();
        room.players.get_mut(&1).unwrap().size = 30.0;
        room.players.get_mut(&2).unwrap().size = 20.0;
        resolve_match_by_timer(&mut room);
        assert_eq!(room.match_winner, Some(1));
        assert!(room.prompt.is_empty());
    }

    #[test]
    fn resolve_match_by_timer_skips_if_already_won() {
        let mut room = test_room();
        room.match_winner = Some(2);
        room.players.get_mut(&1).unwrap().size = 99.0;
        resolve_match_by_timer(&mut room);
        assert_eq!(room.match_winner, Some(2));
    }

    #[test]
    fn reset_for_rematch_clears_match_state() {
        use crate::powerup::{ActivePowerUp, PowerUpKind, PowerUpOffer};

        let mut room = test_room();
        room.match_winner = Some(1);
        room.match_deadline = Some(Instant::now());
        room.prompt = "old prompt".to_string();
        room.round_id = 5;
        room.players.get_mut(&1).unwrap().size = 30.0;
        room.players.get_mut(&2).unwrap().size = 20.0;
        room.players.get_mut(&1).unwrap().progress = "partial".to_string();
        room.next_offer_id = 5;
        room.powerup_offers.push(PowerUpOffer {
            offer_id: 4,
            kind: PowerUpKind::DoublePoints,
            player_id: 2,
            expires_at: Instant::now(),
        });
        room.active_powerups.push(ActivePowerUp {
            kind: PowerUpKind::FreezeAllCompetitors,
            source_player_id: 1,
            expires_at: Instant::now(),
            duration: Duration::from_secs(15),
        });

        room.reset_for_rematch();

        assert_eq!(room.match_winner, None);
        assert!(room.match_deadline.is_none());
        assert!(room.prompt.is_empty());
        assert!(room.powerup_offers.is_empty());
        assert!(room.active_powerups.is_empty());
        assert_eq!(room.next_offer_id, 0);
        assert_eq!(room.round_id, 0);
        assert_eq!(room.players.len(), 2);
        assert_eq!(room.players.get(&1).unwrap().size, DEFAULT_START_SIZE);
        assert_eq!(room.players.get(&2).unwrap().size, DEFAULT_START_SIZE);
        assert!(room.players.get(&1).unwrap().progress.is_empty());
    }

    #[test]
    fn resolve_match_by_timer_skips_if_empty() {
        let mut room = test_room();
        room.players.clear();
        resolve_match_by_timer(&mut room);
        assert_eq!(room.match_winner, None);
    }

    #[test]
    fn wrong_answer_penalty_reduces_size() {
        let mut room = test_room();
        let shrink = apply_wrong_answer_penalty(&mut room, 1, 3.0).unwrap();
        assert_eq!(shrink, 3.0);
        assert_eq!(room.players.get(&1).unwrap().size, 7.0);
        assert_eq!(room.players.get(&2).unwrap().size, 10.0);
    }

    #[test]
    fn wrong_answer_penalty_clamps_to_min() {
        let mut room = test_room();
        let shrink = apply_wrong_answer_penalty(&mut room, 1, 100.0).unwrap();
        assert_eq!(shrink, 9.0);
        assert_eq!(room.players.get(&1).unwrap().size, MIN_PLAYER_SIZE);
    }

    #[test]
    fn wrong_answer_penalty_returns_none_for_missing_player() {
        let mut room = test_room();
        assert!(apply_wrong_answer_penalty(&mut room, 99, 3.0).is_none());
    }

    #[test]
    fn wrong_answer_penalty_clears_progress() {
        let mut room = test_room();
        room.players.get_mut(&1).unwrap().progress = "partial".to_string();
        apply_wrong_answer_penalty(&mut room, 1, 2.0);
        assert!(room.players.get(&1).unwrap().progress.is_empty());
    }

    #[test]
    fn find_top_player_ids_excludes_self() {
        let mut room = test_room();
        room.players.get_mut(&1).unwrap().size = 30.0;
        room.players.get_mut(&2).unwrap().size = 20.0;
        let top = find_top_player_ids(&room.players, 1);
        assert_eq!(top, vec![2]);
    }

    #[test]
    fn find_top_player_ids_returns_tied_players() {
        let mut room = test_room();
        room.players.insert(3, player(3, 10.0));
        room.players.get_mut(&1).unwrap().size = 25.0;
        room.players.get_mut(&2).unwrap().size = 25.0;
        room.players.get_mut(&3).unwrap().size = 10.0;
        let mut top = find_top_player_ids(&room.players, 3);
        top.sort();
        assert_eq!(top, vec![1, 2]);
    }

    #[test]
    fn find_top_player_ids_empty_when_only_excluded() {
        let mut room = test_room();
        room.players.retain(|id, _| *id == 1);
        let top = find_top_player_ids(&room.players, 1);
        assert!(top.is_empty());
    }

    #[test]
    fn deduct_from_top_players_basic() {
        let mut room = test_room();
        room.players.get_mut(&1).unwrap().size = 30.0;
        room.players.get_mut(&2).unwrap().size = 20.0;
        let deducted = deduct_from_top_players(&mut room, 5.0, 1);
        assert_eq!(deducted, 5.0);
        assert_eq!(room.players.get(&2).unwrap().size, 15.0);
        assert_eq!(room.players.get(&1).unwrap().size, 30.0);
    }

    #[test]
    fn deduct_from_top_players_splits_among_tied() {
        let mut room = test_room();
        room.players.insert(3, player(3, 20.0));
        room.players.get_mut(&1).unwrap().size = 5.0;
        room.players.get_mut(&2).unwrap().size = 20.0;
        let deducted = deduct_from_top_players(&mut room, 6.0, 1);
        assert_eq!(deducted, 6.0);
        assert_eq!(room.players.get(&2).unwrap().size, 17.0);
        assert_eq!(room.players.get(&3).unwrap().size, 17.0);
    }

    #[test]
    fn deduct_from_top_players_clamps_to_min() {
        let mut room = test_room();
        room.players.get_mut(&1).unwrap().size = 5.0;
        room.players.get_mut(&2).unwrap().size = 2.0;
        let deducted = deduct_from_top_players(&mut room, 100.0, 1);
        assert_eq!(deducted, 1.0);
        assert_eq!(room.players.get(&2).unwrap().size, MIN_PLAYER_SIZE);
    }

    #[test]
    fn deduct_from_top_players_returns_zero_when_no_targets() {
        let mut room = test_room();
        room.players.retain(|id, _| *id == 1);
        let deducted = deduct_from_top_players(&mut room, 10.0, 1);
        assert_eq!(deducted, 0.0);
    }
}
