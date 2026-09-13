use std::io;
use std::io::{Read, Write};

use anyhow::Result;

const GOLDEN: u64 = 0x61C8_8646_80B5_83EB;
const GOLDEN32: u32 = 2_654_435_761; // 0x9E3779B1

#[inline]
fn mix(v: u64) -> u64 {
    let t = 0xBF58_476D_1CE4_E5B9u64.wrapping_mul(v ^ (v >> 30));
    let u = t ^ (t >> 27);
    let w = 0x94D0_49BB_1331_11EBu64.wrapping_mul(u);
    w ^ (w >> 31)
}

pub fn key_gen(key: u32) -> [u8; 16] {
    let seed = ((key as u64) << 32) ^ (GOLDEN32 as u64 * key as u64 + 1);
    let seed = seed.wrapping_sub(GOLDEN);

    let mut ks = [0u8; 16];
    ks[..8].copy_from_slice(&mix(seed).to_le_bytes());
    ks[8..].copy_from_slice(&mix(seed.wrapping_sub(GOLDEN)).to_le_bytes());
    ks
}

pub fn apply_key(data: &mut [u8], key: &[u8; 16]) {
    for (i, byte) in data.iter_mut().enumerate() {
        *byte ^= key[i % 16];
    }
}

pub struct XorReader<R> {
    inner: R,
    pad: [u8; 16],
    phase: usize,
}

impl<R: Read> XorReader<R> {
    #[inline]
    pub fn new(inner: R, pad: [u8; 16]) -> Self {
        Self { inner, pad, phase: 0 }
    }
}

impl<R: Read> Read for XorReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;

        let mut i = 0;
        while i < n {
            let take = (16 - self.phase).min(n - i);
            for (b, p) in buf[i..i + take]
                .iter_mut()
                .zip(&self.pad[self.phase..self.phase + take])
            {
                *b ^= p;
            }
            i += take;
            self.phase = (self.phase + take) & 15;
        }
        Ok(n)
    }
}