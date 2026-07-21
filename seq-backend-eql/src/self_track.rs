//! Cross-packet EQL session identity: which spawn id is "us".
//!
//! Every other parser in this crate is a pure function over one packet. This
//! one is deliberately stateful, because the quirk it exists to absorb is not
//! expressible per-packet:
//!
//! eql announces the local player's `OP_ZoneEntry` TWICE per zone — a live copy
//! that moves and a static phantom a few ids higher — and it keys the player's
//! MOVEMENT to the first id but their PROFILE/BUFF/STAT data to the second.
//! Worse, the two records are not adjacent on the wire: the stat-sync packet
//! carrying the real HP/mana/endurance maxima can arrive BEFORE the phantom's
//! record does, and in some zone-ins it is the only packet that ever carries
//! them (every later one for that id is a stat-less keepalive). Matching on the
//! id alone therefore drops the player's maxima entirely, depending on nothing
//! more than wire ordering.
//!
//! So this tracker holds two things: the id pair, and the most recent wide
//! vitals for a plausible-but-not-yet-resolved twin id. When the phantom's
//! record finally lands and resolves that id as ours, the host drains the held
//! vitals via [`SelfTracker::take_pending_vitals`] and applies them.
//!
//! It lives here rather than in a host so that showeq-daemon and scry inherit
//! the behaviour instead of each re-deriving it.

use crate::StatSync;

/// The live copy and its phantom twin are issued in one batch, so their ids sit
/// within a few of each other; a stale id from a previous zone is hundreds or
/// thousands off. Matches the window the daemon's `consumeSelfSpawn` used.
pub const SAME_BATCH: u32 = 16;

/// What a self-named `OP_ZoneEntry` record means for the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpawnRouting {
    /// Not the local player — hand it to the spawn list as normal.
    NotSelf = 0,
    /// The live/moving copy: adopt as the player's id.
    AdoptSelf = 1,
    /// The phantom twin (or a re-announce of the adopted id): swallow it, but
    /// remember the id — the player's stats are keyed to it.
    SelfTwin = 2,
}

/// One stat-sync packet's verdict. `is_self` false means the packet belongs to
/// some other spawn and the caller should route its HP normally; the `has_*`
/// flags are only meaningful when `is_self` is true.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SelfStat {
    pub is_self: bool,
    pub has_hp: bool,
    pub hp_cur: i64,
    pub hp_max: i64,
    pub has_mana: bool,
    pub mana_cur: i64,
    pub mana_max: i64,
    pub has_end: bool,
    pub end_cur: i64,
    pub end_max: i64,
}

impl SelfStat {
    /// True when at least one stat is carried. A wide packet with no stat bits
    /// set is the periodic keepalive and updates nothing.
    pub fn any(&self) -> bool {
        self.has_hp || self.has_mana || self.has_end
    }

    fn from_stat_sync(s: &StatSync, is_self: bool) -> Self {
        Self {
            is_self,
            has_hp: s.has_hp,
            hp_cur: s.hp_cur,
            hp_max: s.hp_max,
            has_mana: s.has_mana,
            mana_cur: s.mana_cur,
            mana_max: s.mana_max,
            has_end: s.has_end,
            end_cur: s.end_cur,
            end_max: s.end_max,
        }
    }
}

/// Tracks the local player's id pair for one session, and holds vitals that
/// arrived before the id carrying them could be resolved.
///
/// Reset it wherever the host severs the self-id — zone change, `OP_EnterWorld`
/// re-entry, and the player's own death all issue a fresh id.
#[derive(Debug, Default, Clone)]
pub struct SelfTracker {
    self_id: u32,
    alt_id: u32,
    /// `(spawn_id, vitals)` for a wide packet whose id was a plausible twin but
    /// was not yet known to be ours. At most one — the newest wins, since these
    /// are absolute cur/max snapshots rather than deltas.
    pending: Option<(u32, SelfStat)>,
}

impl SelfTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forget the session's identity. Keeps nothing: a new zone re-announces
    /// both records, and holding a previous zone's pending vitals could let
    /// them land on a recycled id.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The adopted (movement) id, or 0 if not yet resolved.
    pub fn self_id(&self) -> u32 {
        self.self_id
    }

    /// The phantom twin id that stats/profile/buffs are keyed to, or 0.
    pub fn alt_id(&self) -> u32 {
        self.alt_id
    }

    /// Either id counts as the player.
    pub fn is_self(&self, id: u32) -> bool {
        id != 0 && (id == self.self_id || id == self.alt_id)
    }

    /// Close enough to the adopted id to be its twin, but not yet resolved.
    fn is_twin_candidate(&self, id: u32) -> bool {
        self.self_id != 0 && id != 0 && id.abs_diff(self.self_id) <= SAME_BATCH
    }

    /// Classify an `OP_ZoneEntry` record. `player_name` is the host's
    /// authoritative character name (from `OP_PlayerProfile`); an empty name on
    /// either side can never match, so records seen before the profile lands
    /// fall through to the spawn list.
    pub fn observe_spawn(
        &mut self,
        player_name: &str,
        spawn_name: &str,
        spawn_id: u32,
    ) -> SpawnRouting {
        if spawn_id == 0
            || player_name.is_empty()
            || spawn_name.is_empty()
            || spawn_name != player_name
        {
            return SpawnRouting::NotSelf;
        }

        // No id yet (fresh login / post-death sever), or an id that jumped
        // zones: this is the new live copy.
        if self.self_id == 0 || spawn_id.abs_diff(self.self_id) > SAME_BATCH {
            self.self_id = spawn_id;
            self.alt_id = 0;
            // Anything held against the previous zone's ids is stale.
            if !matches!(self.pending, Some((id, _)) if id == spawn_id) {
                self.pending = None;
            }
            return SpawnRouting::AdoptSelf;
        }

        if spawn_id != self.self_id {
            self.alt_id = spawn_id;
        }
        SpawnRouting::SelfTwin
    }

    /// Classify a decoded stat-sync packet.
    ///
    /// When the id isn't (yet) known to be ours but is a plausible twin, the
    /// vitals are held for later. The packet is still reported as `is_self:
    /// false` so the caller routes its HP to the spawn list as usual — if the
    /// id turns out to be a real neighbouring spawn rather than our twin,
    /// nothing was swallowed, and if it turns out to be ours the spawn-list
    /// update was a no-op against an id that has no spawn entry.
    pub fn observe_stat_sync(&mut self, s: &StatSync) -> SelfStat {
        if self.is_self(s.spawn_id) {
            return SelfStat::from_stat_sync(s, true);
        }

        if s.wide && self.is_twin_candidate(s.spawn_id) {
            let v = SelfStat::from_stat_sync(s, true);
            if v.any() {
                self.pending = Some((s.spawn_id, v));
            }
        }

        SelfStat::default()
    }

    /// Drain vitals held for an id that has since been resolved as ours.
    /// Returns an all-false value when there is nothing to apply.
    pub fn take_pending_vitals(&mut self) -> SelfStat {
        if let Some((id, v)) = self.pending {
            if self.is_self(id) {
                self.pending = None;
                return v;
            }
        }
        SelfStat::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: &str = "Testchar";

    fn wide(spawn_id: u32, hp: (i64, i64), mana: (i64, i64), end: (i64, i64)) -> StatSync {
        StatSync {
            spawn_id,
            wide: true,
            has_hp: true,
            hp_cur: hp.0,
            hp_max: hp.1,
            has_mana: true,
            mana_cur: mana.0,
            mana_max: mana.1,
            has_end: true,
            end_cur: end.0,
            end_max: end.1,
        }
    }

    fn keepalive(spawn_id: u32) -> StatSync {
        StatSync { spawn_id, wide: true, ..StatSync::default() }
    }

    #[test]
    fn adopts_the_first_self_record_and_twins_the_second() {
        let mut t = SelfTracker::new();
        assert_eq!(t.observe_spawn(ME, ME, 5893), SpawnRouting::AdoptSelf);
        assert_eq!(t.self_id(), 5893);
        assert_eq!(t.observe_spawn(ME, ME, 5906), SpawnRouting::SelfTwin);
        assert_eq!(t.self_id(), 5893, "movement id must not be re-homed by the twin");
        assert_eq!(t.alt_id(), 5906);
        assert!(t.is_self(5893) && t.is_self(5906));
    }

    #[test]
    fn other_spawns_are_not_self() {
        let mut t = SelfTracker::new();
        t.observe_spawn(ME, ME, 5893);
        assert_eq!(t.observe_spawn(ME, "Someoneelse", 5901), SpawnRouting::NotSelf);
        assert!(!t.is_self(5901));
    }

    #[test]
    fn a_distant_id_re_homes_rather_than_twinning() {
        let mut t = SelfTracker::new();
        t.observe_spawn(ME, ME, 5893);
        t.observe_spawn(ME, ME, 5906);
        // Next zone: ids jump far away.
        assert_eq!(t.observe_spawn(ME, ME, 12636), SpawnRouting::AdoptSelf);
        assert_eq!(t.self_id(), 12636);
        assert_eq!(t.alt_id(), 0, "the previous zone's twin must not linger");
        assert!(!t.is_self(5906));
    }

    #[test]
    fn stats_on_the_twin_id_are_ours() {
        let mut t = SelfTracker::new();
        t.observe_spawn(ME, ME, 5893);
        t.observe_spawn(ME, ME, 5906);
        let got = t.observe_stat_sync(&wide(5906, (4023, 4265), (1780, 4170), (1138, 2976)));
        assert!(got.is_self);
        assert_eq!((got.hp_max, got.mana_max, got.end_max), (4265, 4170, 2976));
    }

    /// The regression this module exists for: the only packet carrying the
    /// player's maxima arrives BEFORE the phantom record that identifies its
    /// id. Matching on the resolved ids alone loses it permanently.
    #[test]
    fn vitals_arriving_before_the_twin_record_are_replayed() {
        let mut t = SelfTracker::new();
        assert_eq!(t.observe_spawn(ME, ME, 5893), SpawnRouting::AdoptSelf);

        // Stats land first, keyed to an id nothing has claimed yet.
        let early = t.observe_stat_sync(&wide(5906, (4023, 4265), (1780, 4170), (1138, 2976)));
        assert!(!early.is_self, "cannot be attributed yet");

        // Every later packet for that id is a stat-less keepalive.
        assert!(!t.observe_stat_sync(&keepalive(5906)).any());

        // The phantom's record finally lands.
        assert_eq!(t.observe_spawn(ME, ME, 5906), SpawnRouting::SelfTwin);

        let held = t.take_pending_vitals();
        assert!(held.is_self && held.any());
        assert_eq!((held.hp_cur, held.hp_max), (4023, 4265));
        assert_eq!((held.mana_cur, held.mana_max), (1780, 4170));
        assert_eq!((held.end_cur, held.end_max), (1138, 2976));

        // Draining is one-shot.
        assert!(!t.take_pending_vitals().any());
    }

    #[test]
    fn held_vitals_are_not_applied_to_an_unrelated_id() {
        let mut t = SelfTracker::new();
        t.observe_spawn(ME, ME, 5893);
        t.observe_stat_sync(&wide(5906, (10, 20), (30, 40), (50, 60)));
        // A neighbour resolves, but it is not us and not our twin.
        assert_eq!(t.observe_spawn(ME, "Neighbour", 5901), SpawnRouting::NotSelf);
        assert!(!t.take_pending_vitals().any());
    }

    #[test]
    fn distant_ids_are_never_held() {
        let mut t = SelfTracker::new();
        t.observe_spawn(ME, ME, 5893);
        // Well outside the batch window — a real unrelated spawn.
        t.observe_stat_sync(&wide(1002, (540, 668), (381, 410), (524, 524)));
        t.observe_spawn(ME, ME, 5906);
        assert!(!t.take_pending_vitals().any());
    }

    #[test]
    fn keepalives_never_overwrite_held_vitals() {
        let mut t = SelfTracker::new();
        t.observe_spawn(ME, ME, 5893);
        t.observe_stat_sync(&wide(5906, (4023, 4265), (1780, 4170), (1138, 2976)));
        t.observe_stat_sync(&keepalive(5906));
        t.observe_spawn(ME, ME, 5906);
        assert_eq!(t.take_pending_vitals().hp_max, 4265);
    }

    #[test]
    fn reset_clears_everything() {
        let mut t = SelfTracker::new();
        t.observe_spawn(ME, ME, 5893);
        t.observe_stat_sync(&wide(5906, (1, 2), (3, 4), (5, 6)));
        t.reset();
        assert_eq!(t.self_id(), 0);
        assert_eq!(t.alt_id(), 0);
        assert!(!t.is_self(5893));
        t.observe_spawn(ME, ME, 5906);
        assert!(!t.take_pending_vitals().any(), "pre-reset vitals must not survive");
    }

    #[test]
    fn nothing_matches_before_the_profile_names_us() {
        let mut t = SelfTracker::new();
        assert_eq!(t.observe_spawn("", ME, 5893), SpawnRouting::NotSelf);
        assert_eq!(t.self_id(), 0);
        assert!(!t.observe_stat_sync(&wide(5893, (1, 2), (3, 4), (5, 6))).is_self);
    }
}
