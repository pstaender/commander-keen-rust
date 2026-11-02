//! Id Software's Huffman expansion (CAL_HuffExpand). The dictionary is 255
//! nodes of two 16-bit words; values < 256 are leaves, otherwise node+256.

#[derive(Clone)]
pub struct Huffman {
    nodes: Vec<(u16, u16)>,
}

impl Huffman {
    pub fn new(dict: &[u8]) -> Self {
        let nodes = (0..255)
            .map(|i| {
                let a = u16::from_le_bytes([dict[i * 4], dict[i * 4 + 1]]);
                let b = u16::from_le_bytes([dict[i * 4 + 2], dict[i * 4 + 3]]);
                (a, b)
            })
            .collect();
        Huffman { nodes }
    }

    /// Expand `src` until `len` bytes have been produced.
    pub fn expand(&self, src: &[u8], len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(len);
        let mut node = 254usize;
        'outer: for &b in src {
            for bit in 0..8 {
                let (n0, n1) = self.nodes[node];
                let v = if (b >> bit) & 1 == 0 { n0 } else { n1 };
                if v < 256 {
                    out.push(v as u8);
                    node = 254;
                    if out.len() >= len {
                        break 'outer;
                    }
                } else {
                    node = (v - 256) as usize;
                    if node > 254 {
                        node = 254;
                    }
                }
            }
        }
        out.resize(len, 0);
        out
    }
}
