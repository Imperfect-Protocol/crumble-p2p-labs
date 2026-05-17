//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use serde::{Deserialize, Serialize};

use crate::{PeerId, PlayerIndex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopologyPolicy {
    Ring,
    Mesh,
    RingOneBackup,
    RingTwoBackups,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatedPeer {
    pub player_index: PlayerIndex,
    pub peer_id: PeerId,
}

pub fn expected_peers(
    topology: TopologyPolicy,
    self_index: PlayerIndex,
    seated: &[SeatedPeer],
) -> Vec<SeatedPeer> {
    if seated.len() < 2 || self_index >= seated.len() {
        return Vec::new();
    }

    match topology {
        TopologyPolicy::Mesh => seated
            .iter()
            .filter(|peer| peer.player_index != self_index)
            .cloned()
            .collect(),
        TopologyPolicy::Ring => ring_neighbors(self_index, seated, 1),
        TopologyPolicy::RingOneBackup => ring_neighbors(self_index, seated, 2),
        TopologyPolicy::RingTwoBackups => ring_neighbors(self_index, seated, 3),
    }
}

pub fn outbound_offer_targets(
    topology: TopologyPolicy,
    self_index: PlayerIndex,
    seated: &[SeatedPeer],
) -> Vec<SeatedPeer> {
    expected_peers(topology, self_index, seated)
        .into_iter()
        .filter(|peer| self_index < peer.player_index)
        .collect()
}

fn ring_neighbors(
    self_index: PlayerIndex,
    seated: &[SeatedPeer],
    backup_depth: usize,
) -> Vec<SeatedPeer> {
    let count = seated.len();
    let mut peers = Vec::new();

    for distance in 1..=backup_depth.min(count.saturating_sub(1)) {
        let previous = (self_index + count - distance) % count;
        let next = (self_index + distance) % count;

        push_unique_peer(&mut peers, seated[previous].clone());
        push_unique_peer(&mut peers, seated[next].clone());
    }

    peers
}

fn push_unique_peer(peers: &mut Vec<SeatedPeer>, peer: SeatedPeer) {
    if peers
        .iter()
        .any(|existing| existing.player_index == peer.player_index)
    {
        return;
    }
    peers.push(peer);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peers(count: usize) -> Vec<SeatedPeer> {
        (0..count)
            .map(|player_index| SeatedPeer {
                player_index,
                peer_id: format!("player-{player_index}"),
            })
            .collect()
    }

    #[test]
    fn mesh_targets_every_other_peer() {
        let targets = expected_peers(TopologyPolicy::Mesh, 1, &peers(4));
        assert_eq!(
            targets.iter().map(|p| p.player_index).collect::<Vec<_>>(),
            vec![0, 2, 3]
        );
    }

    #[test]
    fn outbound_mesh_offers_target_later_players_only() {
        let targets = outbound_offer_targets(TopologyPolicy::Mesh, 1, &peers(4));
        assert_eq!(
            targets.iter().map(|p| p.player_index).collect::<Vec<_>>(),
            vec![2, 3]
        );
    }

    #[test]
    fn backup_ring_deduplicates_small_tables() {
        let targets = expected_peers(TopologyPolicy::RingTwoBackups, 0, &peers(3));
        assert_eq!(
            targets.iter().map(|p| p.player_index).collect::<Vec<_>>(),
            vec![2, 1]
        );
    }
}
