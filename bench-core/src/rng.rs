//! Deterministic ChaCha8 random stream.
//!
//! Every encounter uses one `ChaCha8` stream seeded once. The cursor tracks
//! how many 32-bit draws have been consumed so the stream can be rewound for
//! undo/replay. A batch fight derives its seed deterministically from the
//! master seed and fight index.

use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

/// Algorithm name stored in encounter state.
pub const RNG_ALGORITHM: &str = "chacha8";

/// RNG state snapshot stored in encounter state files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RngState {
    pub algorithm: String,
    pub initial_seed: u64,
    pub cursor: u64,
}

impl RngState {
    pub fn new(seed: u64) -> Self {
        Self {
            algorithm: RNG_ALGORITHM.into(),
            initial_seed: seed,
            cursor: 0,
        }
    }
}

/// Wrapper around `ChaCha8Rng` that records a rewindable cursor.
#[derive(Debug, Clone)]
pub struct Stream {
    rng: ChaCha8Rng,
    initial_seed: u64,
    cursor: u64,
}

impl Stream {
    /// Build a stream from a seed.
    pub fn from_seed(seed: u64) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            initial_seed: seed,
            cursor: 0,
        }
    }

    /// Build a stream from saved state.
    pub fn from_state(state: RngState) -> Self {
        let mut stream = Self::from_seed(state.initial_seed);
        stream.advance_to(state.cursor);
        stream
    }

    /// Snapshot current RNG state.
    pub fn state(&self) -> RngState {
        RngState {
            algorithm: RNG_ALGORITHM.into(),
            initial_seed: self.initial_seed,
            cursor: self.cursor,
        }
    }

    /// Draw a single `u32`, advancing the cursor.
    pub fn draw_u32(&mut self) -> u32 {
        self.cursor += 1;
        self.rng.next_u32()
    }

    /// Draw a single `u64`, advancing the cursor by two draws.
    pub fn draw_u64(&mut self) -> u64 {
        self.cursor += 2;
        self.rng.next_u64()
    }

    /// Advance the stream to `target` draws, no-op if already ahead.
    pub fn advance_to(&mut self, target: u64) {
        while self.cursor < target {
            self.rng.next_u32();
            self.cursor += 1;
        }
    }

    /// Rewind to a previous cursor. Panics if `target` is greater than current.
    pub fn rewind_to(&mut self, target: u64) {
        assert!(
            target <= self.cursor,
            "rewind target {} is beyond cursor {}",
            target,
            self.cursor
        );
        let mut fresh = Self::from_seed(self.initial_seed);
        fresh.advance_to(target);
        *self = fresh;
    }

    /// Roll a `dN` (1-indexed), drawing once.
    pub fn roll_die(&mut self, sides: u32) -> u32 {
        if sides == 0 {
            return 0;
        }
        (self.draw_u32() % sides) + 1
    }
}

/// Derive a per-fight seed from a master seed and fight index.
///
/// Uses SHA-256 over the big-endian encoding of both numbers and returns the
/// first 64 bits as a `u64`. This mixing is stable across platforms and Rust
/// versions, preserving the bit-for-bit replay guarantee in ADR 0007.
pub fn derive_fight_seed(master_seed: u64, fight_index: u64) -> u64 {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(master_seed.to_be_bytes());
    hasher.update(fight_index.to_be_bytes());
    let digest = hasher.finalize();
    u64::from_be_bytes([
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Stream::from_seed(42);
        let mut b = Stream::from_seed(42);
        assert_eq!(a.draw_u32(), b.draw_u32());
        assert_eq!(a.draw_u64(), b.draw_u64());
        assert_eq!(a.state(), b.state());
    }

    #[test]
    fn rewind_reproduces_state() {
        let mut stream = Stream::from_seed(7);
        let checkpoint = stream.state();
        stream.draw_u32();
        stream.draw_u32();
        stream.rewind_to(checkpoint.cursor);
        assert_eq!(stream.state(), checkpoint);
        assert_eq!(stream.draw_u32(), Stream::from_seed(7).draw_u32());
    }

    #[test]
    fn fight_seed_deterministic() {
        let s1 = derive_fight_seed(123, 0);
        let s2 = derive_fight_seed(123, 0);
        let s3 = derive_fight_seed(123, 1);
        assert_eq!(s1, s2);
        assert_ne!(s1, s3);
    }

    #[test]
    fn fight_seed_cross_platform_spec() {
        // The SHA-256 based mixing should produce this exact value on any
        // platform for the inputs (42, 7). If this changes, the replay contract
        // in ADR 0007 breaks.
        let seed = derive_fight_seed(42, 7);
        assert_eq!(seed, 0x72_12_eb_fe_b3_ca_a3_cf);
    }
}
