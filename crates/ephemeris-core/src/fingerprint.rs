// The contents of this file are subject to the Common Public Attribution License Version 1.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at https://opensource.org/license/CPAL-1.0. The License is based on the Mozilla Public License Version 1.1 but Sections 14 and 15 have been added to cover use of software over a computer network and provide for limited attribution for the Original Developer. In addition, Exhibit A has been modified to be consistent with Exhibit B.
//
// Software distributed under the License is distributed on an "AS IS" basis, WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License for the specific language governing rights and limitations under the License.
//
// The Original Code is the original VirgoStars-Ephemeris source code released under CPAL-1.0 by the Initial Developer, excluding third-party programs, data, and other materials licensed separately.
//
// The Original Developer is the Initial Developer.
//
// The Initial Developer of the Original Code is Cesare-ml (VirgoStars project). All portions of the code written by Cesare-ml are Copyright (c) 2026 Cesare-ml. All Rights Reserved.
//
// Contributors retain applicable rights in their own contributions under the terms of this License.
//
// [NOTE: The text of this Exhibit A may differ slightly from the text of the notices in the Source Code files of the Original Code. You should use the text of this Exhibit A rather than the text found in the Original Code Source Code for Your Modifications.]

//! Stable SHA-256 source fingerprinting for reproducibility metadata.
//!
//! Small, original FIPS 180-4 SHA-256 implementation; used to identify
//! exact local snapshot bytes, **not** to authenticate a publisher, sign
//! data or replace independent source review.

const INITIAL: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
    0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

const ROUND: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
    0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
    0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
    0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
    0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn compress(state: &mut [u32; 8], block: &[u8]) {
    let mut schedule = [0_u32; 64];
    for (i, word) in schedule[..16].iter_mut().enumerate() {
        let start = 4 * i;
        *word = u32::from_be_bytes([
            block[start], block[start + 1], block[start + 2], block[start + 3],
        ]);
    }
    for i in 16..64 {
        let s0 = schedule[i - 15].rotate_right(7)
            ^ schedule[i - 15].rotate_right(18)
            ^ (schedule[i - 15] >> 3);
        let s1 = schedule[i - 2].rotate_right(17)
            ^ schedule[i - 2].rotate_right(19)
            ^ (schedule[i - 2] >> 10);
        schedule[i] = schedule[i - 16]
            .wrapping_add(s0)
            .wrapping_add(schedule[i - 7])
            .wrapping_add(s1);
    }

    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for (i, word) in schedule.iter().enumerate() {
        let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let choose = (e & f) ^ ((!e) & g);
        let t1 = h
            .wrapping_add(sum1)
            .wrapping_add(choose)
            .wrapping_add(ROUND[i])
            .wrapping_add(*word);
        let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let majority = (a & b) ^ (a & c) ^ (b & c);
        let t2 = sum0.wrapping_add(majority);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }

    for (state_word, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
        *state_word = state_word.wrapping_add(value);
    }
}

/// Return lowercase 64-character SHA-256 of the **exact input bytes**.
///
/// This is a reproducible *content fingerprint*: differing whitespace or
/// provenance comments cause a different digest. It does not authenticate
/// the source or verify the IANA upstream #h checksum.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let mut state = INITIAL;
    let mut complete = bytes.chunks_exact(64);
    for block in &mut complete {
        compress(&mut state, block);
    }

    let rest = complete.remainder();
    let mut padded = [0_u8; 128];
    padded[..rest.len()].copy_from_slice(rest);
    padded[rest.len()] = 0x80;

    let tail_len = if rest.len() < 56 { 64 } else { 128 };
    let bit_len = (bytes.len() as u64).wrapping_mul(8);
    padded[tail_len - 8..tail_len].copy_from_slice(&bit_len.to_be_bytes());
    for block in padded[..tail_len].chunks_exact(64) {
        compress(&mut state, block);
    }

    use std::fmt::Write;
    let mut digest = String::with_capacity(64);
    for word in state {
        write!(&mut digest, "{word:08x}").expect("writing to String cannot fail");
    }
    digest
}

#[cfg(test)]
mod tests {
    use super::sha256_hex;

    #[test]
    fn fips_180_4_empty_and_abc_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn fips_180_4_multiblock_and_million_a_vectors() {
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        assert_eq!(
            sha256_hex(&vec![b'a'; 1_000_000]),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn exact_content_bytes_and_padding_boundaries_change_hash() {
        let original = [b'x'; 55];
        let mut altered = original;
        altered[0] = b'y';
        assert_ne!(sha256_hex(&original), sha256_hex(&altered));
        for count in [0, 1, 55, 56, 63, 64, 65, 119, 120, 128] {
            let input = vec![b'z'; count];
            assert_eq!(sha256_hex(&input).len(), 64);
            assert_ne!(sha256_hex(&input), sha256_hex(&[input, vec![b'\n']].concat()));
        }
    }
}
