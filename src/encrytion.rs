use std::io;
use std::io::{Read, Write};

const GOLDEN: u64 = 0x61C8_8646_80B5_83EB;
const GOLDEN32: u32 = 2_654_435_761; // 0x9E3779B1

/// 单次投递给 inner 的最大字节数。
const CHUNK: usize = 16 * 256;

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

/// 16 字节循环密钥流。
pub struct XorPad {
    pad: [u8; 16],
    phase: usize,
}

impl XorPad {
    #[inline]
    pub fn new(pad: [u8; 16]) -> Self {
        Self { pad, phase: 0 }
    }

    /// 就地异或，允许任意长度、任意切分
    pub fn apply(&mut self, buf: &mut [u8]) {
        let mut i = 0;
        while i < buf.len() {
            let take = (16 - self.phase).min(buf.len() - i);
            for (b, p) in buf[i..i + take]
                .iter_mut()
                .zip(&self.pad[self.phase..self.phase + take])
            {
                *b ^= p;
            }
            i += take;
            self.phase = (self.phase + take) & 15;
        }
    }
}

pub fn apply_key(data: &mut [u8], key: &[u8; 16]) {
    XorPad::new(*key).apply(data);
}

pub struct XorReader<R> {
    inner: R,
    pad: XorPad,
}

impl<R: Read> XorReader<R> {
    #[inline]
    pub fn new(inner: R, key: [u8; 16]) -> Self {
        Self { inner, pad: XorPad::new(key) }
    }

    #[inline]
    pub fn get_ref(&self) -> &R { &self.inner }

    #[inline]
    pub fn get_mut(&mut self) -> &mut R { &mut self.inner }

    #[inline]
    pub fn into_inner(self) -> R { self.inner }
}

impl<R: Read> Read for XorReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // 读回来多少字节，就只解开多少字节，phase 同步前进这么多
        let n = self.inner.read(buf)?;
        self.pad.apply(&mut buf[..n]);
        Ok(n)
    }
}

pub struct XorWriter<W> {
    inner: W,
    pad: XorPad,
}

impl<W: Write> XorWriter<W> {
    #[inline]
    pub fn new(inner: W, key: [u8; 16]) -> Self {
        Self { inner, pad: XorPad::new(key) }
    }

    #[inline]
    pub fn get_ref(&self) -> &W { &self.inner }

    #[inline]
    pub fn get_mut(&mut self) -> &mut W { &mut self.inner }

    #[inline]
    pub fn into_inner(self) -> W { self.inner }
}

impl<W: Write> Write for XorWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // write 拿到的是 &[u8]，没法就地异或，只能复制到栈上的定长块再写出去
        if buf.is_empty() {
            return Ok(0);
        }

        let mut tmp = [0u8; CHUNK];
        let take = buf.len().min(CHUNK);
        let tmp = &mut tmp[..take];
        tmp.copy_from_slice(&buf[..take]);

        let phase = self.pad.phase;
        self.pad.apply(tmp);

        match self.inner.write(tmp) {
            // 一个字节都没写出去：密钥流一步都不能动
            Ok(0) => {
                self.pad.phase = phase;
                Err(io::ErrorKind::WriteZero.into())
            }
            // 只有真正写出去的 n 字节才推进密钥流，
            // 这样调用方接下来用 buf[n..] 重试时，密钥流位置正好对得上
            Ok(n) => {
                self.pad.phase = (phase + n) & 15;
                Ok(n)
            }
            // 出错时调用方会整体重试 buf，所以 phase 必须退回原点
            Err(e) => {
                self.pad.phase = phase;
                Err(e)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
