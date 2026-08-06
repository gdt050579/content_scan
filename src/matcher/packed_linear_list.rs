use crate::ContentType;

pub(crate) trait Key: Copy + Eq {
    const WIDTH: usize;
    const ZERO: Self;
    fn pack(b: &[u8]) -> Self;
}

impl Key for u32 {
    const WIDTH: usize = 4;
    const ZERO: Self = 0;
    #[inline(always)]
    fn pack(b: &[u8]) -> Self {
        let mut buf = [0u8; 4];
        let n = b.len().min(4);
        buf[..n].copy_from_slice(&b[..n]);
        u32::from_be_bytes(buf)
    }
}

impl Key for u64 {
    const WIDTH: usize = 8;
    const ZERO: Self = 0;
    #[inline(always)]
    fn pack(b: &[u8]) -> Self {
        let mut buf = [0u8; 8];
        let n = b.len().min(8);
        buf[..n].copy_from_slice(&b[..n]);
        u64::from_be_bytes(buf)
    }
}

pub(crate) struct PackedLinearList<T: ContentType, K: Key> {
    keys: [K; 16],
    values: [T; 16],
    len: usize,        
    pat_len: usize,    
}

impl<T: ContentType, K: Key> PackedLinearList<T, K> {
    pub(crate) fn new(patterns: &[(T, &'static [u8])]) -> Option<Self> {
        if patterns.is_empty() || patterns.len() > 16 {
            return None;
        }
        let pat_len = patterns[0].1.len();
        if pat_len == 0 || pat_len > K::WIDTH {
            return None;
        }
        if patterns.iter().any(|(_, d)| d.len() != pat_len) {
            return None;
        }

        let mut keys = [K::ZERO; 16];
        let mut values: [T; 16] = [patterns[0].0; 16];
        let mut len = 0usize;

        for (i, (ct, data)) in patterns.into_iter().enumerate() {
            keys[i] = K::pack(data);
            values[i] = *ct;
            len += 1;
        }

        Some(Self { keys, values, len, pat_len })
    }

    #[inline(always)]
    pub(crate) const fn pattern_len(&self) -> usize {
        self.pat_len
    }
    #[inline(always)]
    pub(crate) fn find(&self, k: K) -> Option<T> {
        for i in 0..self.len {
            if self.keys[i] == k {
                return Some(self.values[i]);
            }
        }
        None
    }
}