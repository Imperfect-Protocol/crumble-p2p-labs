//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use alloy_primitives::Keccak256;
use bls12_381::G1Projective;

pub struct Keccak256Hash(Keccak256);

impl digest::BlockInput for Keccak256Hash {
    type BlockSize = digest::generic_array::typenum::U64;
}

impl digest::Digest for Keccak256Hash {
    type OutputSize = digest::generic_array::typenum::U64;

    fn new() -> Self {
        Self(Keccak256::default())
    }

    fn output_size() -> usize {
        32
    }

    fn chain(mut self, data: impl AsRef<[u8]>) -> Self {
        self.0.update(data);
        self
    }

    fn update(&mut self, data: impl AsRef<[u8]>) {
        self.0.update(data);
    }

    fn finalize(self) -> digest::Output<Self> {
        let res = self.0.finalize();
        #[allow(deprecated)]
        let mut arr = digest::generic_array::GenericArray::default();
        arr[..32].copy_from_slice(&res.0);
        arr
    }

    fn reset(&mut self) {
        self.0 = Keccak256::default();
    }

    #[allow(deprecated)]
    fn digest(_data: &[u8]) -> digest::Output<Self> {
        unimplemented!()
    }

    fn finalize_reset(&mut self) -> digest::Output<Self> {
        unimplemented!()
    }
}

pub fn hash_to_curve(message: &[u8]) -> G1Projective {
    use bls12_381::hash_to_curve::{ExpandMsgXmd, HashToCurve};
    let cs = b"BLS_SIG_BLS12381G2_XMD:KECCAK-256_SSWU_RO_";
    <G1Projective as HashToCurve<ExpandMsgXmd<Keccak256Hash>>>::hash_to_curve(message, cs)
}
