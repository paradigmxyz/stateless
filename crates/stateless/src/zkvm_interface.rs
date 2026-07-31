//! revm [`Crypto`] and alloy [`CryptoProvider`] implementations using [`zkvm_interface`].

use alloc::{sync::Arc, vec, vec::Vec};
use core::mem::transmute;

use alloy_consensus::crypto::{CryptoProvider, RecoveryError, install_default_provider};
use alloy_primitives::Address;
use revm::precompile::{
    Crypto, PrecompileHalt,
    bls12_381::{G1Point, G1PointScalar, G2Point, G2PointScalar},
};
use zkvm_interface::{
    zkvm_blake2f_message, zkvm_blake2f_offset, zkvm_blake2f_state, zkvm_bls12_381_fp,
    zkvm_bls12_381_fp2, zkvm_bls12_381_g1_msm_pair, zkvm_bls12_381_g1_point,
    zkvm_bls12_381_g2_msm_pair, zkvm_bls12_381_g2_point, zkvm_bls12_381_pairing_pair,
    zkvm_bls12_381_scalar, zkvm_bn254_g1_point, zkvm_bn254_g2_point, zkvm_bn254_pairing_pair,
    zkvm_bn254_scalar, zkvm_keccak256_hash, zkvm_kzg_commitment, zkvm_kzg_field_element,
    zkvm_kzg_proof, zkvm_ripemd160_hash, zkvm_secp256k1_hash, zkvm_secp256k1_pubkey,
    zkvm_secp256k1_signature, zkvm_secp256r1_hash, zkvm_secp256r1_pubkey, zkvm_secp256r1_signature,
    zkvm_sha256_hash,
};

/// Installs [`ZkVMInterfaceCrypto`] as [`revm::precompile::Crypto`] backend and as
/// [`alloy_consensus::crypto::CryptoProvider`].
///
/// # Panics
///
/// Panics if a revm or Alloy crypto backend has already been installed.
pub fn install_crypto() {
    assert!(revm::install_crypto(ZkVMInterfaceCrypto));
    install_default_provider(Arc::new(ZkVMInterfaceCrypto)).unwrap();
}

/// revm and Alloy crypto provider backed by the standard zkVM interface.
#[derive(Debug, Default)]
pub struct ZkVMInterfaceCrypto;

impl Crypto for ZkVMInterfaceCrypto {
    #[inline]
    fn sha256(&self, input: &[u8]) -> [u8; 32] {
        sha256(input)
    }

    #[inline]
    fn blake2_compress(&self, rounds: u32, h: &mut [u64; 8], m: &[u64; 16], t: &[u64; 2], f: bool) {
        let mut state = zkvm_blake2f_state { data: unsafe { transmute::<[u64; 8], [u8; 64]>(*h) } };
        let m = zkvm_blake2f_message { data: unsafe { transmute::<[u64; 16], [u8; 128]>(*m) } };
        let t = zkvm_blake2f_offset { data: unsafe { transmute::<[u64; 2], [u8; 16]>(*t) } };
        let ret = unsafe { zkvm_interface::zkvm_blake2f(rounds, &mut state, &m, &t, f as u8) };
        assert_eq!(ret, 0, "blake2f failed");
        *h = unsafe { transmute::<[u8; 64], [u64; 8]>(state.data) };
    }

    #[inline]
    fn ripemd160(&self, input: &[u8]) -> [u8; 32] {
        let mut output = zkvm_ripemd160_hash { data: [0; 32] };
        let ret =
            unsafe { zkvm_interface::zkvm_ripemd160(input.as_ptr(), input.len(), &mut output) };
        assert_eq!(ret, 0, "ripemd160 failed");
        output.data
    }

    #[inline]
    fn modexp(&self, base: &[u8], exp: &[u8], modulus: &[u8]) -> Result<Vec<u8>, PrecompileHalt> {
        let mut output = vec![0u8; modulus.len()];
        let ret = unsafe {
            zkvm_interface::zkvm_modexp(
                base.as_ptr(),
                base.len(),
                exp.as_ptr(),
                exp.len(),
                modulus.as_ptr(),
                modulus.len(),
                output.as_mut_ptr(),
            )
        };
        (ret == 0).then_some(output).ok_or_else(|| PrecompileHalt::other("modexp failed"))
    }

    #[inline]
    fn secp256k1_ecrecover(
        &self,
        sig: &[u8; 64],
        recid: u8,
        msg: &[u8; 32],
    ) -> Result<[u8; 32], PrecompileHalt> {
        let msg = zkvm_secp256k1_hash { data: *msg };
        let sig = zkvm_secp256k1_signature { data: *sig };
        let mut pubkey = zkvm_secp256k1_pubkey { data: [0; 64] };
        let ret =
            unsafe { zkvm_interface::zkvm_secp256k1_ecrecover(&msg, &sig, recid, &mut pubkey) };
        if ret != 0 {
            return Err(PrecompileHalt::Secp256k1RecoverFailed);
        }
        let mut hash = keccak256(&pubkey.data);
        hash[..12].fill(0);
        Ok(hash)
    }

    #[inline]
    fn secp256r1_verify_signature(&self, msg: &[u8; 32], sig: &[u8; 64], pk: &[u8; 64]) -> bool {
        let msg = zkvm_secp256r1_hash { data: *msg };
        let sig = zkvm_secp256r1_signature { data: *sig };
        let pk = zkvm_secp256r1_pubkey { data: *pk };
        let mut verified = false;
        let ret = unsafe { zkvm_interface::zkvm_secp256r1_verify(&msg, &sig, &pk, &mut verified) };
        ret == 0 && verified
    }

    #[inline]
    fn bn254_g1_add(&self, p1: &[u8], p2: &[u8]) -> Result<[u8; 64], PrecompileHalt> {
        let p1: &[u8; 64] =
            p1.try_into().map_err(|_| PrecompileHalt::other("bn254 g1_add bad p1 len"))?;
        let p2: &[u8; 64] =
            p2.try_into().map_err(|_| PrecompileHalt::other("bn254 g1_add bad p2 len"))?;
        let p1 = zkvm_bn254_g1_point { data: *p1 };
        let p2 = zkvm_bn254_g1_point { data: *p2 };
        let mut result = zkvm_bn254_g1_point { data: [0; 64] };
        let ret = unsafe { zkvm_interface::zkvm_bn254_g1_add(&p1, &p2, &mut result) };
        (ret == 0)
            .then_some(result.data)
            .ok_or_else(|| PrecompileHalt::other("bn254_g1_add failed"))
    }

    #[inline]
    fn bn254_g1_mul(&self, point: &[u8], scalar: &[u8]) -> Result<[u8; 64], PrecompileHalt> {
        let point: &[u8; 64] =
            point.try_into().map_err(|_| PrecompileHalt::other("bn254 g1_mul bad point len"))?;
        let scalar: &[u8; 32] =
            scalar.try_into().map_err(|_| PrecompileHalt::other("bn254 g1_mul bad scalar len"))?;
        let point = zkvm_bn254_g1_point { data: *point };
        let scalar = zkvm_bn254_scalar { data: *scalar };
        let mut result = zkvm_bn254_g1_point { data: [0; 64] };
        let ret = unsafe { zkvm_interface::zkvm_bn254_g1_mul(&point, &scalar, &mut result) };
        (ret == 0)
            .then_some(result.data)
            .ok_or_else(|| PrecompileHalt::other("bn254_g1_mul failed"))
    }

    #[inline]
    fn bn254_pairing_check(&self, pairs: &[(&[u8], &[u8])]) -> Result<bool, PrecompileHalt> {
        let pairs: Vec<zkvm_bn254_pairing_pair> = pairs
            .iter()
            .map(|(g1, g2)| zkvm_bn254_pairing_pair {
                g1: zkvm_bn254_g1_point { data: (*g1).try_into().unwrap() },
                g2: zkvm_bn254_g2_point { data: (*g2).try_into().unwrap() },
            })
            .collect();
        let mut verified = false;
        let ret = unsafe {
            zkvm_interface::zkvm_bn254_pairing(pairs.as_ptr(), pairs.len(), &mut verified)
        };
        (ret == 0).then_some(verified).ok_or_else(|| PrecompileHalt::other("bn254_pairing failed"))
    }

    #[inline]
    fn bls12_381_g1_add(&self, a: G1Point, b: G1Point) -> Result<[u8; 96], PrecompileHalt> {
        let a = pack_bls12_381_g1(&a);
        let b = pack_bls12_381_g1(&b);
        let mut result = zkvm_bls12_381_g1_point { data: [0; 96] };
        let ret = unsafe { zkvm_interface::zkvm_bls12_g1_add(&a, &b, &mut result) };
        (ret == 0).then_some(result.data).ok_or(PrecompileHalt::Bls12381G1NotOnCurve)
    }

    #[inline]
    fn bls12_381_g1_msm(
        &self,
        pairs: &mut dyn Iterator<Item = Result<G1PointScalar, PrecompileHalt>>,
    ) -> Result<[u8; 96], PrecompileHalt> {
        let pairs: Vec<zkvm_bls12_381_g1_msm_pair> = pairs
            .map(|pair| {
                let (point, scalar) = pair?;
                Ok(zkvm_bls12_381_g1_msm_pair {
                    point: pack_bls12_381_g1(&point),
                    scalar: zkvm_bls12_381_scalar { data: scalar },
                })
            })
            .collect::<Result<_, PrecompileHalt>>()?;
        let mut result = zkvm_bls12_381_g1_point { data: [0; 96] };
        let ret =
            unsafe { zkvm_interface::zkvm_bls12_g1_msm(pairs.as_ptr(), pairs.len(), &mut result) };
        (ret == 0).then_some(result.data).ok_or(PrecompileHalt::Bls12381G1NotOnCurve)
    }

    #[inline]
    fn bls12_381_g2_add(&self, a: G2Point, b: G2Point) -> Result<[u8; 192], PrecompileHalt> {
        let a = pack_bls12_381_g2(&a);
        let b = pack_bls12_381_g2(&b);
        let mut result = zkvm_bls12_381_g2_point { data: [0; 192] };
        let ret = unsafe { zkvm_interface::zkvm_bls12_g2_add(&a, &b, &mut result) };
        (ret == 0).then_some(result.data).ok_or(PrecompileHalt::Bls12381G2NotOnCurve)
    }

    #[inline]
    fn bls12_381_g2_msm(
        &self,
        pairs: &mut dyn Iterator<Item = Result<G2PointScalar, PrecompileHalt>>,
    ) -> Result<[u8; 192], PrecompileHalt> {
        let pairs: Vec<zkvm_bls12_381_g2_msm_pair> = pairs
            .map(|pair| {
                let (point, scalar) = pair?;
                Ok(zkvm_bls12_381_g2_msm_pair {
                    point: pack_bls12_381_g2(&point),
                    scalar: zkvm_bls12_381_scalar { data: scalar },
                })
            })
            .collect::<Result<_, PrecompileHalt>>()?;
        let mut result = zkvm_bls12_381_g2_point { data: [0; 192] };
        let ret =
            unsafe { zkvm_interface::zkvm_bls12_g2_msm(pairs.as_ptr(), pairs.len(), &mut result) };
        (ret == 0).then_some(result.data).ok_or(PrecompileHalt::Bls12381G2NotOnCurve)
    }

    #[inline]
    fn bls12_381_pairing_check(
        &self,
        pairs: &[(G1Point, G2Point)],
    ) -> Result<bool, PrecompileHalt> {
        let pairs: Vec<zkvm_bls12_381_pairing_pair> = pairs
            .iter()
            .map(|(g1, g2)| zkvm_bls12_381_pairing_pair {
                g1: pack_bls12_381_g1(g1),
                g2: pack_bls12_381_g2(g2),
            })
            .collect();
        let mut verified = false;
        let ret = unsafe {
            zkvm_interface::zkvm_bls12_pairing(pairs.as_ptr(), pairs.len(), &mut verified)
        };
        (ret == 0)
            .then_some(verified)
            .ok_or_else(|| PrecompileHalt::other("bls12_381_pairing failed"))
    }

    #[inline]
    fn bls12_381_fp_to_g1(&self, fp: &[u8; 48]) -> Result<[u8; 96], PrecompileHalt> {
        let fp = zkvm_bls12_381_fp { data: *fp };
        let mut result = zkvm_bls12_381_g1_point { data: [0; 96] };
        let ret = unsafe { zkvm_interface::zkvm_bls12_map_fp_to_g1(&fp, &mut result) };
        (ret == 0)
            .then_some(result.data)
            .ok_or_else(|| PrecompileHalt::other("bls12_381_fp_to_g1 failed"))
    }

    #[inline]
    fn bls12_381_fp2_to_g2(&self, fp2: ([u8; 48], [u8; 48])) -> Result<[u8; 192], PrecompileHalt> {
        let fp2 = {
            let mut data = [0u8; 96];
            data[..48].copy_from_slice(&fp2.0);
            data[48..].copy_from_slice(&fp2.1);
            zkvm_bls12_381_fp2 { data }
        };
        let mut result = zkvm_bls12_381_g2_point { data: [0; 192] };
        let ret = unsafe { zkvm_interface::zkvm_bls12_map_fp2_to_g2(&fp2, &mut result) };
        (ret == 0)
            .then_some(result.data)
            .ok_or_else(|| PrecompileHalt::other("bls12_381_fp2_to_g2 failed"))
    }

    #[inline]
    fn verify_kzg_proof(
        &self,
        z: &[u8; 32],
        y: &[u8; 32],
        commitment: &[u8; 48],
        proof: &[u8; 48],
    ) -> Result<(), PrecompileHalt> {
        let commitment = zkvm_kzg_commitment { data: *commitment };
        let z = zkvm_kzg_field_element { data: *z };
        let y = zkvm_kzg_field_element { data: *y };
        let proof = zkvm_kzg_proof { data: *proof };
        let mut verified = false;
        let ret = unsafe {
            zkvm_interface::zkvm_kzg_point_eval(&commitment, &z, &y, &proof, &mut verified)
        };
        (ret == 0 && verified).then_some(()).ok_or(PrecompileHalt::BlobVerifyKzgProofFailed)
    }
}

impl CryptoProvider for ZkVMInterfaceCrypto {
    #[inline]
    fn recover_signer_unchecked(
        &self,
        sig: &[u8; 65],
        msg: &[u8; 32],
    ) -> Result<Address, RecoveryError> {
        let msg = zkvm_secp256k1_hash { data: *msg };
        let recid = sig[64];
        let sig = zkvm_secp256k1_signature { data: sig[..64].try_into().unwrap() };
        let mut pubkey = zkvm_secp256k1_pubkey { data: [0; 64] };
        let ret =
            unsafe { zkvm_interface::zkvm_secp256k1_ecrecover(&msg, &sig, recid, &mut pubkey) };
        if ret != 0 {
            return Err(RecoveryError::new());
        }
        let hash = keccak256(&pubkey.data);
        Ok(Address::from_slice(&hash[12..]))
    }

    #[inline]
    fn verify_and_compute_signer_unchecked(
        &self,
        pubkey: &[u8; 65],
        sig: &[u8; 64],
        msg: &[u8; 32],
    ) -> Result<Address, RecoveryError> {
        let pubkey = zkvm_secp256k1_pubkey { data: *uncompressed_pubkey_coordinates(pubkey)? };
        let msg = zkvm_secp256k1_hash { data: *msg };
        let sig = zkvm_secp256k1_signature { data: *sig };
        let mut verified = false;
        let ret =
            unsafe { zkvm_interface::zkvm_secp256k1_verify(&msg, &sig, &pubkey, &mut verified) };
        if ret != 0 || !verified {
            return Err(RecoveryError::new());
        }
        let hash = keccak256(&pubkey.data);
        Ok(Address::from_slice(&hash[12..]))
    }
}

#[inline]
fn uncompressed_pubkey_coordinates(pubkey: &[u8; 65]) -> Result<&[u8; 64], RecoveryError> {
    if pubkey[0] != 0x04 {
        return Err(RecoveryError::new());
    }
    Ok(pubkey[1..].try_into().expect("public key coordinates have a fixed length"))
}

#[inline]
fn sha256(data: &[u8]) -> [u8; 32] {
    let mut output = zkvm_sha256_hash { data: [0; 32] };
    let ret = unsafe { zkvm_interface::zkvm_sha256(data.as_ptr(), data.len(), &mut output) };
    assert_eq!(ret, 0, "sha256 failed");
    output.data
}

#[inline]
fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut output = zkvm_keccak256_hash { data: [0; 32] };
    let ret = unsafe { zkvm_interface::zkvm_keccak256(data.as_ptr(), data.len(), &mut output) };
    assert_eq!(ret, 0, "keccak256 failed");
    output.data
}

#[inline]
fn pack_bls12_381_g1(p: &G1Point) -> zkvm_bls12_381_g1_point {
    let mut data = [0u8; 96];
    data[..48].copy_from_slice(&p.0);
    data[48..].copy_from_slice(&p.1);
    zkvm_bls12_381_g1_point { data }
}

#[inline]
fn pack_bls12_381_g2(p: &G2Point) -> zkvm_bls12_381_g2_point {
    let mut data = [0u8; 192];
    data[..48].copy_from_slice(&p.0);
    data[48..96].copy_from_slice(&p.1);
    data[96..144].copy_from_slice(&p.2);
    data[144..].copy_from_slice(&p.3);
    zkvm_bls12_381_g2_point { data }
}

#[cfg(test)]
mod tests {
    use super::uncompressed_pubkey_coordinates;

    #[test]
    fn validates_uncompressed_public_key_prefix() {
        let mut public_key = [0u8; 65];
        assert!(uncompressed_pubkey_coordinates(&public_key).is_err());

        public_key[0] = 0x04;
        assert!(uncompressed_pubkey_coordinates(&public_key).is_ok());
    }
}
