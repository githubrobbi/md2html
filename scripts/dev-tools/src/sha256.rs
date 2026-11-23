// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC.

//! SHA-256 (FIPS 180-4), for the checksums beside the release archives.

/// The fractional parts of the cube roots of the first 64 primes.
const ROUND: [u32; 64] = [
    0x428A_2F98,
    0x7137_4491,
    0xB5C0_FBCF,
    0xE9B5_DBA5,
    0x3956_C25B,
    0x59F1_11F1,
    0x923F_82A4,
    0xAB1C_5ED5,
    0xD807_AA98,
    0x1283_5B01,
    0x2431_85BE,
    0x550C_7DC3,
    0x72BE_5D74,
    0x80DE_B1FE,
    0x9BDC_06A7,
    0xC19B_F174,
    0xE49B_69C1,
    0xEFBE_4786,
    0x0FC1_9DC6,
    0x240C_A1CC,
    0x2DE9_2C6F,
    0x4A74_84AA,
    0x5CB0_A9DC,
    0x76F9_88DA,
    0x983E_5152,
    0xA831_C66D,
    0xB003_27C8,
    0xBF59_7FC7,
    0xC6E0_0BF3,
    0xD5A7_9147,
    0x06CA_6351,
    0x1429_2967,
    0x27B7_0A85,
    0x2E1B_2138,
    0x4D2C_6DFC,
    0x5338_0D13,
    0x650A_7354,
    0x766A_0ABB,
    0x81C2_C92E,
    0x9272_2C85,
    0xA2BF_E8A1,
    0xA81A_664B,
    0xC24B_8B70,
    0xC76C_51A3,
    0xD192_E819,
    0xD699_0624,
    0xF40E_3585,
    0x106A_A070,
    0x19A4_C116,
    0x1E37_6C08,
    0x2748_774C,
    0x34B0_BCB5,
    0x391C_0CB3,
    0x4ED8_AA4A,
    0x5B9C_CA4F,
    0x682E_6FF3,
    0x748F_82EE,
    0x78A5_636F,
    0x84C8_7814,
    0x8CC7_0208,
    0x90BE_FFFA,
    0xA450_6CEB,
    0xBEF9_A3F7,
    0xC671_78F2,
];

/// The fractional parts of the square roots of the first eight primes.
const START: [u32; 8] = [
    0x6A09_E667,
    0xBB67_AE85,
    0x3C6E_F372,
    0xA54F_F53A,
    0x510E_527F,
    0x9B05_688C,
    0x1F83_D9AB,
    0x5BE0_CD19,
];

/// Folds one 64-byte block into the state.
///
/// The message schedule is a window of the sixteen words the next one is
/// made from: each round takes the oldest word, then the window moves on
/// by one with the new word at its end. Every position in the window and
/// in the state is a constant inside its array.
fn fold(state: &mut [u32; 8], block: &[u8; 64]) {
    let mut window = [0_u32; 16];
    for (slot, bytes) in window.iter_mut().zip(block.as_chunks::<4>().0) {
        *slot = u32::from_be_bytes(*bytes);
    }
    let mut work = *state;
    for constant in ROUND {
        let message = window[0];
        let low = window[1].rotate_right(7) ^ window[1].rotate_right(18) ^ (window[1] >> 3_u32);
        let high =
            window[14].rotate_right(17) ^ window[14].rotate_right(19) ^ (window[14] >> 10_u32);
        let next = message
            .wrapping_add(low)
            .wrapping_add(window[9])
            .wrapping_add(high);
        window.rotate_left(1);
        window[15] = next;
        let big1 = work[4].rotate_right(6) ^ work[4].rotate_right(11) ^ work[4].rotate_right(25);
        let choose = (work[4] & work[5]) ^ (!work[4] & work[6]);
        let first = work[7]
            .wrapping_add(big1)
            .wrapping_add(choose)
            .wrapping_add(constant)
            .wrapping_add(message);
        let big0 = work[0].rotate_right(2) ^ work[0].rotate_right(13) ^ work[0].rotate_right(22);
        let majority = (work[0] & work[1]) ^ (work[0] & work[2]) ^ (work[1] & work[2]);
        let second = big0.wrapping_add(majority);
        work = [
            first.wrapping_add(second),
            work[0],
            work[1],
            work[2],
            work[3].wrapping_add(first),
            work[4],
            work[5],
            work[6],
        ];
    }
    for (sum, value) in state.iter_mut().zip(work) {
        *sum = sum.wrapping_add(value);
    }
}

/// The SHA-256 of some bytes, as 64 hexadecimal digits.
///
/// ```
/// assert_eq!(
///     md2html_dev_tools::sha256_hex(b"abc"),
///     "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
/// );
/// ```
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut state = START;
    let (blocks, rest) = bytes.as_chunks::<64>();
    for block in blocks {
        fold(&mut state, block);
    }
    // The tail: the bytes left over, a single 1 bit, zeros, and the length in
    // bits as 64 bits, filling one block or two.
    let mut tail = rest.to_vec();
    tail.push(0x80);
    while tail.len() % 64 != 56 {
        tail.push(0);
    }
    let bits = u64::try_from(bytes.len())
        .unwrap_or(u64::MAX)
        .wrapping_mul(8);
    tail.extend_from_slice(&bits.to_be_bytes());
    for block in tail.as_chunks::<64>().0 {
        fold(&mut state, block);
    }
    let mut hex = String::with_capacity(64);
    for byte in state.iter().flat_map(|word| word.to_be_bytes()) {
        for nibble in [byte >> 4_u8, byte & 0x0F_u8] {
            hex.push(char::from_digit(u32::from(nibble), 16).unwrap_or('0'));
        }
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::sha256_hex;

    /// The digest of the standard test messages is the published one.
    ///
    /// # Panics
    ///
    /// When the digest of the empty message, the 448-bit message of the
    /// standard, 55, 56 or 64 `a`s (the edges of one padding block or two), or
    /// a million `a`s differs from its published value.
    #[test]
    fn the_standard_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        // One block of padding or two: 55, 56 and 64 bytes sit on the edges.
        assert_eq!(
            sha256_hex(&[b'a'; 55]),
            "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318"
        );
        assert_eq!(
            sha256_hex(&[b'a'; 56]),
            "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a"
        );
        assert_eq!(
            sha256_hex(&[b'a'; 64]),
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb"
        );
        assert_eq!(
            sha256_hex(&vec![b'a'; 1_000_000]),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }
}
