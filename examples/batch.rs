//! Signature verification per signature: libsecp256k1 one at a time (parsing each key, as a
//! client does per event), the same with this crate's key cache, and batches of 16 to 1024
//! with every signature by its own key, by 8 keys, or by one.
//!
//!   cargo run --release --example batch [rounds]

use nostr_verify::{SigRef, Verifier};
use secp256k1::{Keypair, Secp256k1};
use std::hint::black_box;
use std::time::Instant;

struct Owned {
    pk: [u8; 32],
    msg: [u8; 32],
    sig: [u8; 64],
}

fn signed(n: usize, keys: usize) -> Vec<Owned> {
    let secp = Secp256k1::new();
    let pairs: Vec<Keypair> = (0..keys).map(|_| Keypair::new(&secp, &mut rand::thread_rng())).collect();
    (0..n)
        .map(|i| {
            let kp = &pairs[i % keys];
            let msg: [u8; 32] = rand::random();
            let sig = secp.sign_schnorr_no_aux_rand(&msg, kp);
            Owned { pk: kp.x_only_public_key().0.serialize(), msg, sig: sig.to_byte_array() }
        })
        .collect()
}

fn refs(v: &[Owned]) -> Vec<SigRef<'_>> {
    v.iter().map(|o| SigRef { pubkey: &o.pk, msg: &o.msg, sig: &o.sig }).collect()
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn main() {
    let rounds: usize = std::env::args().nth(1).map_or(15, |s| s.parse().expect("rounds"));
    let mut ok = Vec::new();

    // One at a time over 512 signatures by 512 keys: every key parsed, then 8 keys: cached.
    let (many, few) = (signed(512, 512), signed(512, 8));
    let (many, few) = (refs(&many), refs(&few));
    let mut v = Verifier::new();
    let plain = median(
        (0..rounds)
            .map(|_| {
                let t = Instant::now();
                for c in &many {
                    black_box(nostr_verify::verify_one(c));
                }
                t.elapsed().as_nanos() as f64 / many.len() as f64
            })
            .collect(),
    );
    let cached = median(
        (0..rounds)
            .map(|_| {
                let t = Instant::now();
                for c in &few {
                    black_box(v.verify_one(c));
                }
                t.elapsed().as_nanos() as f64 / few.len() as f64
            })
            .collect(),
    );
    println!("µs per signature, median of {rounds} rounds");
    println!("  {:<44} {:>7.2}", "libsecp256k1, one at a time", plain / 1000.0);
    println!("  {:<44} {:>7.2}  {:.2}x", "  with the key cache (8 keys, warm)", cached / 1000.0, plain / cached);

    println!("  {:<44} {:>7} {:>7} {:>7}", "batch of n, signed by:", "n keys", "8 keys", "1 key");
    for n in [16, 64, 256, 1024] {
        let mut row = Vec::new();
        for keys in [n, 8, 1] {
            // New keys every round: only repeats inside a batch help.
            let sets: Vec<Vec<Owned>> = (0..rounds).map(|_| signed(n, keys)).collect();
            let mut v = Verifier::new();
            let per = median(
                sets.iter()
                    .map(|set| {
                        let sigs = refs(set);
                        let t = Instant::now();
                        assert!(v.verify_batch(&sigs, &mut ok));
                        t.elapsed().as_nanos() as f64 / n as f64
                    })
                    .collect(),
            );
            row.push(per);
        }
        println!(
            "  {:<44} {:>7.2} {:>7.2} {:>7.2}   ({:.2}x / {:.2}x / {:.2}x)",
            format!("  {n}"),
            row[0] / 1000.0,
            row[1] / 1000.0,
            row[2] / 1000.0,
            plain / row[0],
            plain / row[1],
            plain / row[2]
        );
    }
}
