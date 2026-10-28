//! Parsed public keys, cached per thread.
//!
//! Turning a 32-byte x-only key into a point costs a field square root, about
//! 4 µs, a tenth of a verification. Authors repeat a lot: on the real traffic
//! a 2048-entry cache hits for 80% of events (Concord plane keys, bots,
//! profiles; gift-wrap keys are the misses). Entries keep our lifted point
//! for batches and, once the one-at-a-time path has asked, libsecp256k1's own
//! parse of the key, so that path stays entirely libsecp256k1's.
//!
//! Only keys that parsed are cached. Anyone can choose keys that collide in a
//! set; that only costs them (and the victims in that set) cache misses.

use super::group::Affine;
use secp256k1::XOnlyPublicKey;

/// A key and whichever parses of it have been needed so far.
#[derive(Clone, Copy)]
struct Entry {
    key: [u8; 32],
    point: Option<Affine>,
    xonly: Option<XOnlyPublicKey>,
}

/// Two-way set-associative, least recently used within a set.
pub struct KeyCache {
    sets: Vec<[Option<Entry>; 2]>,
    shift: u32,
    salt: u64,
    /// Lookups that found the parse they wanted, and those that had to make it.
    pub hits: u64,
    pub misses: u64,
}

impl KeyCache {
    /// 2^bits sets of two entries.
    pub fn new(bits: u32, salt: u64) -> KeyCache {
        KeyCache { sets: vec![[None, None]; 1 << bits], shift: 64 - bits, salt, hits: 0, misses: 0 }
    }

    #[inline]
    fn set(&self, key: &[u8; 32]) -> usize {
        let v = u64::from_le_bytes(key[..8].try_into().unwrap()) ^ self.salt;
        (v.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> self.shift) as usize
    }

    /// The entry for `key`, moved to the front of its set.
    #[inline]
    fn find(&mut self, key: &[u8; 32]) -> Option<&mut Entry> {
        let i = self.set(key);
        let set = &mut self.sets[i];
        if set[0].as_ref().is_some_and(|e| e.key == *key) {
            return set[0].as_mut();
        }
        if set[1].as_ref().is_some_and(|e| e.key == *key) {
            set.swap(0, 1);
            return set[0].as_mut();
        }
        None
    }

    fn insert(&mut self, e: Entry) -> &mut Entry {
        let i = self.set(&e.key);
        let set = &mut self.sets[i];
        set[1] = set[0].take();
        set[0].insert(e)
    }

    /// The lifted point for `key`, if cached.
    pub fn point(&mut self, key: &[u8; 32]) -> Option<Affine> {
        let point = self.find(key).and_then(|e| e.point);
        if point.is_some() {
            self.hits += 1;
        } else {
            self.misses += 1;
        }
        point
    }

    /// Caches a point our `lift_x` produced for `key`.
    pub fn put_point(&mut self, key: &[u8; 32], point: Affine) {
        match self.find(key) {
            Some(e) => e.point = Some(point),
            None => {
                self.insert(Entry { key: *key, point: Some(point), xonly: None });
            }
        }
    }

    /// libsecp256k1's parse of `key` (`None` when it does not parse).
    pub fn xonly(&mut self, key: &[u8; 32]) -> Option<XOnlyPublicKey> {
        let cached = self.find(key).map(|e| e.xonly);
        if let Some(Some(xonly)) = cached {
            self.hits += 1;
            return Some(xonly);
        }
        self.misses += 1;
        let xonly = XOnlyPublicKey::from_byte_array(key).ok()?;
        match self.find(key) {
            Some(e) => e.xonly = Some(xonly),
            None => {
                self.insert(Entry { key: *key, point: None, xonly: Some(xonly) });
            }
        }
        Some(xonly)
    }
}
