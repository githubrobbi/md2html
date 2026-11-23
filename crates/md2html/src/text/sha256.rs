// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! SHA-256 (FIPS 180-4), for the page's Content-Security-Policy hashes
//! and the fingerprints of translated documents.

use super::runs;

/// The words a hash starts from: the first 32 bits of the fractional
/// parts of the square roots of the first eight primes.
const INITIAL: [u32; 8] = [
    0x6A09_E667,
    0xBB67_AE85,
    0x3C6E_F372,
    0xA54F_F53A,
    0x510E_527F,
    0x9B05_688C,
    0x1F83_D9AB,
    0x5BE0_CD19,
];

/// The constant of each round: the first 32 bits of the fractional parts
/// of the cube roots of the first 64 primes.
const ROUND_CONSTANTS: [u32; 64] = [
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

/// SHA-256 of `data`.
pub(crate) fn sha256(data: &[u8]) -> [u8; 32] {
    let mut state = INITIAL;
    let mut rest = data;
    for block in runs::<64>(&mut rest) {
        compress(&mut state, block);
    }
    for block in runs::<64>(&mut final_blocks(rest, data.len()).as_slice()) {
        compress(&mut state, block);
    }
    let mut digest = [0_u8; 32];
    for (slot, byte) in digest
        .iter_mut()
        .zip(state.iter().flat_map(|word| word.to_be_bytes()))
    {
        *slot = byte;
    }
    digest
}

/// The last one or two blocks: what is left of the message after its
/// whole blocks, the end mark, zeros, and the message's length in bits.
fn final_blocks(tail: &[u8], len: usize) -> Vec<u8> {
    let mut last = Vec::with_capacity(128);
    last.extend_from_slice(tail);
    last.push(0x80);
    last.resize(if last.len() <= 56 { 56 } else { 120 }, 0);
    last.extend_from_slice(&bit_length(len).to_be_bytes());
    last
}

/// A length in bytes as the standard writes it: in bits, modulo 2^64.
/// The length's own bytes are widened (or, were `usize` wider than 64
/// bits, cut) to 64 bits, so no conversion can fail.
fn bit_length(len: usize) -> u64 {
    let mut wide = [0_u8; 8];
    for (slot, byte) in wide.iter_mut().zip(len.to_le_bytes()) {
        *slot = byte;
    }
    u64::from_le_bytes(wide).wrapping_mul(8)
}

/// Mixes one block into the state.
fn compress(state: &mut [u32; 8], block: &[u8; 64]) {
    let mut working = *state;
    for (constant, word) in ROUND_CONSTANTS.into_iter().zip(schedule(block)) {
        working = round(working, constant.wrapping_add(word));
    }
    for (slot, add) in state.iter_mut().zip(working) {
        *slot = slot.wrapping_add(add);
    }
}

/// The 64 words of a block's message schedule: its own sixteen, then
/// each next one mixed from four of the sixteen before it.
fn schedule(block: &[u8; 64]) -> impl Iterator<Item = u32> {
    // The sixteen words before the next one: `window[0]` is the oldest.
    let mut window = [0_u32; 16];
    for (slot, bytes) in window.iter_mut().zip(runs::<4>(&mut block.as_slice())) {
        *slot = u32::from_be_bytes(*bytes);
    }
    (0..64).map(move |_| {
        let word = window[0];
        let next = small_sigma1(window[14])
            .wrapping_add(window[9])
            .wrapping_add(small_sigma0(window[1]))
            .wrapping_add(window[0]);
        window.rotate_left(1);
        window[15] = next;
        word
    })
}

/// One round over the eight working words, a to h, with the round's
/// constant and schedule word already added together.
const fn round(working: [u32; 8], constant_and_word: u32) -> [u32; 8] {
    let [
        a_word,
        b_word,
        c_word,
        d_word,
        e_word,
        f_word,
        g_word,
        h_word,
    ] = working;
    let first = h_word
        .wrapping_add(big_sigma1(e_word))
        .wrapping_add(choose(e_word, f_word, g_word))
        .wrapping_add(constant_and_word);
    let second = big_sigma0(a_word).wrapping_add(majority(a_word, b_word, c_word));
    [
        first.wrapping_add(second),
        a_word,
        b_word,
        c_word,
        d_word.wrapping_add(first),
        e_word,
        f_word,
        g_word,
    ]
}

/// Each bit from `then` where `pick` has a one, from `otherwise` where
/// it has a zero: the standard's Ch, written so that every operation
/// counts.
const fn choose(pick: u32, then: u32, otherwise: u32) -> u32 {
    otherwise ^ (pick & (then ^ otherwise))
}

/// Each bit as at least two of the three have it: the standard's Maj,
/// written so that every operation counts.
const fn majority(first: u32, second: u32, third: u32) -> u32 {
    ((first ^ third) & (second ^ third)) ^ third
}

/// The standard's Σ0, on the first working word.
const fn big_sigma0(word: u32) -> u32 {
    word.rotate_right(2) ^ word.rotate_right(13) ^ word.rotate_right(22)
}

/// The standard's Σ1, on the fifth working word.
const fn big_sigma1(word: u32) -> u32 {
    word.rotate_right(6) ^ word.rotate_right(11) ^ word.rotate_right(25)
}

/// The standard's σ0, on the schedule word fifteen back.
const fn small_sigma0(word: u32) -> u32 {
    word.rotate_right(7) ^ word.rotate_right(18) ^ (word >> 3_i32)
}

/// The standard's σ1, on the schedule word two back.
const fn small_sigma1(word: u32) -> u32 {
    word.rotate_right(17) ^ word.rotate_right(19) ^ (word >> 10_i32)
}

#[cfg(test)]
mod tests {
    use super::{choose, majority, sha256};
    use crate::text::hex;

    /// The digests of the standard test messages are the published ones.
    ///
    /// # Panics
    ///
    /// Panics when the digest of the empty message, of `abc`, of the 56-byte
    /// message of FIPS 180-2, or of a thousand `a`s differs from its published
    /// value.
    #[test]
    fn known_digests() {
        let long = "a".repeat(1000);
        for (message, digest) in [
            (
                "",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                "abc",
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
                "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            ),
            (
                &long,
                "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3",
            ),
        ] {
            assert_eq!(hex(&sha256(message.as_bytes())), digest, "{message}");
        }
    }

    /// Padding is right on both sides of every block boundary.
    ///
    /// # Panics
    ///
    /// Panics when a message of 55, 56, 63, 64, 119 or 120 `a`s gets another
    /// digest: the lengths where the end mark and the length fit in the last
    /// block, just do not, or fall after a whole block, so a padding that is
    /// one block short or long fails here.
    #[test]
    fn the_padding_fits_one_block_or_spills_into_two() {
        // 55 bytes leave room for the end mark and the length in one
        // block, 56 do not; 64 and 120 fill whole blocks before it.
        for (len, digest) in [
            (
                55,
                "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318",
            ),
            (
                56,
                "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a",
            ),
            (
                63,
                "7d3e74a05d7db15bce4ad9ec0658ea98e3f06eeecf16b4c6fff2da457ddc2f34",
            ),
            (
                64,
                "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb",
            ),
            (
                119,
                "31eba51c313a5c08226adf18d4a359cfdfd8d2e816b13f4af952f7ea6584dcfb",
            ),
            (
                120,
                "2f3d335432c70b580af0e8e1b3674a7c020d683aa5f73aaaedfdc55af904c21c",
            ),
        ] {
            assert_eq!(hex(&sha256("a".repeat(len).as_bytes())), digest, "{len}");
        }
    }

    /// The `Ch` and `Maj` functions are the ones the standard defines.
    ///
    /// # Panics
    ///
    /// Panics when, for any of 27 combinations of all-zero, all-one and mixed
    /// words, `choose` differs from `(x & y) ^ (!x & z)` or `majority` from `(x
    /// & y) ^ (x & z) ^ (y & z)`.
    #[test]
    fn choose_and_majority_are_the_standards() {
        for pick in [0_u32, u32::MAX, 0x0F0F_F0F0] {
            for then in [0_u32, u32::MAX, 0x3C3C_3C3C] {
                for otherwise in [0_u32, u32::MAX, 0x5555_AAAA] {
                    assert_eq!(
                        choose(pick, then, otherwise),
                        (pick & then) ^ (!pick & otherwise)
                    );
                    assert_eq!(
                        majority(pick, then, otherwise),
                        (pick & then) ^ (pick & otherwise) ^ (then & otherwise)
                    );
                }
            }
        }
    }
}
