use crate::game::PlayerId;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PowerUpKind {
    FreezeAllCompetitors,
    DoublePoints,
    ScrambleFont,
    ScoreSteal,
    OngoingScoreSteal,
}

const ALL_KINDS: [PowerUpKind; 5] = [
    PowerUpKind::FreezeAllCompetitors,
    PowerUpKind::DoublePoints,
    PowerUpKind::ScrambleFont,
    PowerUpKind::ScoreSteal,
    PowerUpKind::OngoingScoreSteal,
];

const BASE_PLAYER_COUNT: f64 = 5.0;
const BASE_OFFER_DURATION_SECS: f64 = 30.0;
const BASE_DISTRIBUTION_INTERVAL_SECS: f64 = 10.0;

#[derive(Debug, Clone)]
pub struct PowerUpOffer {
    pub offer_id: u64,
    pub kind: PowerUpKind,
    pub player_id: PlayerId,
    pub expires_at: Instant,
}

#[derive(Debug, Clone)]
pub struct ActivePowerUp {
    pub kind: PowerUpKind,
    pub source_player_id: PlayerId,
    pub expires_at: Instant,
    pub duration: Duration,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivePowerUpSnapshot {
    pub kind: PowerUpKind,
    pub source_player_id: PlayerId,
    pub remaining_ms: u64,
    pub duration_ms: u64,
}

impl ActivePowerUp {
    pub fn to_snapshot(&self) -> ActivePowerUpSnapshot {
        let remaining = self
            .expires_at
            .saturating_duration_since(Instant::now())
            .as_millis() as u64;
        ActivePowerUpSnapshot {
            kind: self.kind,
            source_player_id: self.source_player_id,
            remaining_ms: remaining,
            duration_ms: self.duration.as_millis() as u64,
        }
    }
}

fn player_scale(player_count: usize) -> f64 {
    (player_count as f64 / BASE_PLAYER_COUNT).clamp(0.4, 1.0)
}

pub fn effect_duration(kind: PowerUpKind, player_count: usize) -> Duration {
    let base_secs: f64 = match kind {
        PowerUpKind::FreezeAllCompetitors => 15.0,
        PowerUpKind::DoublePoints => 30.0,
        PowerUpKind::ScrambleFont => 20.0,
        PowerUpKind::ScoreSteal => 5.0,
        PowerUpKind::OngoingScoreSteal => 30.0,
    };
    let scaled = base_secs * player_scale(player_count);
    Duration::from_secs_f64(scaled)
}

pub fn offer_duration(player_count: usize) -> Duration {
    let scaled = BASE_OFFER_DURATION_SECS * player_scale(player_count);
    Duration::from_secs_f64(scaled)
}

pub fn distribution_interval(player_count: usize) -> Duration {
    let scale = player_scale(player_count);
    let scaled = BASE_DISTRIBUTION_INTERVAL_SECS / scale;
    Duration::from_secs_f64(scaled.clamp(BASE_DISTRIBUTION_INTERVAL_SECS, 25.0))
}

/// Select a power-up recipient weighted toward players furthest behind.
/// Excludes the player(s) with the highest score.
pub fn pick_powerup_recipient(players: &[(PlayerId, f32)], rng: &mut impl Rng) -> Option<PlayerId> {
    if players.len() < 2 {
        return None;
    }

    let max_size = players
        .iter()
        .map(|(_, size)| *size)
        .fold(f32::NEG_INFINITY, f32::max);

    let eligible: Vec<(PlayerId, f32)> = players
        .iter()
        .filter(|(_, size)| *size < max_size)
        .map(|(id, size)| (*id, max_size - size + 1.0))
        .collect();

    if eligible.is_empty() {
        return None;
    }

    let total_weight: f32 = eligible.iter().map(|(_, w)| w).sum();
    let mut roll = rng.random_range(0.0..total_weight);
    for (id, weight) in &eligible {
        roll -= weight;
        if roll <= 0.0 {
            return Some(*id);
        }
    }

    Some(eligible.last().unwrap().0)
}

pub fn pick_powerup_kind(rng: &mut impl Rng) -> PowerUpKind {
    ALL_KINDS[rng.random_range(0..ALL_KINDS.len())]
}

pub fn is_player_frozen(active_powerups: &[ActivePowerUp], player_id: PlayerId) -> bool {
    let now = Instant::now();
    active_powerups.iter().any(|pu| {
        pu.kind == PowerUpKind::FreezeAllCompetitors
            && pu.source_player_id != player_id
            && pu.expires_at > now
    })
}

pub fn has_double_points(active_powerups: &[ActivePowerUp], player_id: PlayerId) -> bool {
    let now = Instant::now();
    active_powerups.iter().any(|pu| {
        pu.kind == PowerUpKind::DoublePoints
            && pu.source_player_id == player_id
            && pu.expires_at > now
    })
}

pub fn has_ongoing_score_steal(active_powerups: &[ActivePowerUp], player_id: PlayerId) -> bool {
    let now = Instant::now();
    active_powerups.iter().any(|pu| {
        pu.kind == PowerUpKind::OngoingScoreSteal
            && pu.source_player_id == player_id
            && pu.expires_at > now
    })
}

pub struct ExpiredItems {
    pub expired_offers: Vec<PowerUpOffer>,
    pub expired_effects: Vec<ActivePowerUp>,
}

pub fn cleanup_expired(
    offers: &mut Vec<PowerUpOffer>,
    actives: &mut Vec<ActivePowerUp>,
    now: Instant,
) -> ExpiredItems {
    let mut expired_offers = Vec::new();
    let mut expired_effects = Vec::new();

    let mut i = 0;
    while i < offers.len() {
        if offers[i].expires_at <= now {
            expired_offers.push(offers.swap_remove(i));
        } else {
            i += 1;
        }
    }

    let mut i = 0;
    while i < actives.len() {
        if actives[i].expires_at <= now {
            expired_effects.push(actives.swap_remove(i));
        } else {
            i += 1;
        }
    }

    ExpiredItems {
        expired_offers,
        expired_effects,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_recipient_excludes_leader() {
        let players = vec![(1, 20.0), (2, 10.0), (3, 5.0)];
        let mut rng = rand::rng();
        for _ in 0..100 {
            let picked = pick_powerup_recipient(&players, &mut rng).unwrap();
            assert_ne!(picked, 1, "leader should never be picked");
        }
    }

    #[test]
    fn pick_recipient_returns_none_for_single_player() {
        let players = vec![(1, 10.0)];
        assert!(pick_powerup_recipient(&players, &mut rand::rng()).is_none());
    }

    #[test]
    fn pick_recipient_returns_none_when_all_tied() {
        let players = vec![(1, 10.0), (2, 10.0), (3, 10.0)];
        assert!(pick_powerup_recipient(&players, &mut rand::rng()).is_none());
    }

    #[test]
    fn pick_recipient_favors_trailing_players() {
        let players = vec![(1, 30.0), (2, 20.0), (3, 5.0)];
        let mut counts = std::collections::HashMap::new();
        let mut rng = rand::rng();
        for _ in 0..10_000 {
            let picked = pick_powerup_recipient(&players, &mut rng).unwrap();
            *counts.entry(picked).or_insert(0) += 1;
        }
        assert!(
            counts.get(&3).copied().unwrap_or(0) > counts.get(&2).copied().unwrap_or(0),
            "player 3 (furthest behind) should be picked more often than player 2"
        );
    }

    #[test]
    fn pick_kind_returns_valid_variant() {
        let mut rng = rand::rng();
        for _ in 0..100 {
            let kind = pick_powerup_kind(&mut rng);
            assert!(matches!(
                kind,
                PowerUpKind::FreezeAllCompetitors
                    | PowerUpKind::DoublePoints
                    | PowerUpKind::ScrambleFont
                    | PowerUpKind::ScoreSteal
                    | PowerUpKind::OngoingScoreSteal
            ));
        }
    }

    #[test]
    fn frozen_check_respects_source_and_expiry() {
        let now = Instant::now();
        let actives = vec![ActivePowerUp {
            kind: PowerUpKind::FreezeAllCompetitors,
            source_player_id: 1,
            expires_at: now + Duration::from_secs(10),
            duration: Duration::from_secs(10),
        }];
        assert!(is_player_frozen(&actives, 2));
        assert!(!is_player_frozen(&actives, 1));
    }

    #[test]
    fn frozen_check_ignores_expired() {
        let now = Instant::now();
        let actives = vec![ActivePowerUp {
            kind: PowerUpKind::FreezeAllCompetitors,
            source_player_id: 1,
            expires_at: now - Duration::from_secs(1),
            duration: Duration::from_secs(10),
        }];
        assert!(!is_player_frozen(&actives, 2));
    }

    #[test]
    fn double_points_check() {
        let now = Instant::now();
        let actives = vec![ActivePowerUp {
            kind: PowerUpKind::DoublePoints,
            source_player_id: 1,
            expires_at: now + Duration::from_secs(10),
            duration: Duration::from_secs(10),
        }];
        assert!(has_double_points(&actives, 1));
        assert!(!has_double_points(&actives, 2));
    }

    #[test]
    fn ongoing_score_steal_check() {
        let now = Instant::now();
        let actives = vec![ActivePowerUp {
            kind: PowerUpKind::OngoingScoreSteal,
            source_player_id: 1,
            expires_at: now + Duration::from_secs(10),
            duration: Duration::from_secs(10),
        }];
        assert!(has_ongoing_score_steal(&actives, 1));
        assert!(!has_ongoing_score_steal(&actives, 2));
    }

    #[test]
    fn cleanup_removes_expired_entries() {
        let now = Instant::now();
        let mut offers = vec![
            PowerUpOffer {
                offer_id: 0,
                kind: PowerUpKind::DoublePoints,
                player_id: 1,
                expires_at: now - Duration::from_secs(1),
            },
            PowerUpOffer {
                offer_id: 1,
                kind: PowerUpKind::FreezeAllCompetitors,
                player_id: 2,
                expires_at: now + Duration::from_secs(10),
            },
        ];
        let mut actives = vec![
            ActivePowerUp {
                kind: PowerUpKind::FreezeAllCompetitors,
                source_player_id: 3,
                expires_at: now - Duration::from_secs(1),
                duration: Duration::from_secs(10),
            },
            ActivePowerUp {
                kind: PowerUpKind::DoublePoints,
                source_player_id: 4,
                expires_at: now + Duration::from_secs(10),
                duration: Duration::from_secs(10),
            },
        ];

        let expired = cleanup_expired(&mut offers, &mut actives, now);
        assert_eq!(offers.len(), 1);
        assert_eq!(offers[0].player_id, 2);
        assert_eq!(actives.len(), 1);
        assert_eq!(actives[0].source_player_id, 4);
        assert_eq!(expired.expired_offers.len(), 1);
        assert_eq!(expired.expired_effects.len(), 1);
    }

    #[test]
    fn scaling_at_baseline_returns_base_values() {
        let dur = effect_duration(PowerUpKind::FreezeAllCompetitors, 5);
        assert_eq!(dur, Duration::from_secs(15));
        assert_eq!(offer_duration(5), Duration::from_secs(30));
        assert_eq!(distribution_interval(5), Duration::from_secs(10));
    }

    #[test]
    fn scaling_at_two_players_reduces_durations() {
        let freeze = effect_duration(PowerUpKind::FreezeAllCompetitors, 2);
        assert_eq!(freeze, Duration::from_secs(6));
        let offer = offer_duration(2);
        assert_eq!(offer, Duration::from_secs(12));
        let interval = distribution_interval(2);
        assert_eq!(interval, Duration::from_secs(25));
    }
}
