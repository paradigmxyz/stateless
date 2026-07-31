//! OpenVM Crypto Implementation for REVM
//!
//! This module provides OpenVM-optimized implementations of cryptographic operations
//! for both transaction validation (via Alloy crypto provider) and precompile execution.

extern crate alloc;

use alloc::{sync::Arc, vec::Vec};

use alloy_consensus::crypto::{
    backend::{install_default_provider, CryptoProvider},
    RecoveryError,
};
use alloy_primitives::Address;
use openvm_curve_utils::SubgroupCheck;
use openvm_ecc_guest::{
    algebra::IntMod,
    weierstrass::{IntrinsicCurve, WeierstrassPoint},
    AffinePoint, Group,
};
use openvm_k256::ecdsa::{signature::hazmat::PrehashVerifier, RecoveryId, Signature, VerifyingKey};
use openvm_keccak256::keccak256;
use openvm_kzg::{Bytes32, Bytes48, KzgProof};
use openvm_pairing::{
    bls12_381::{self as bls, Bls12_381},
    bn254::{self as bn, Bn254},
    PairingCheck,
};
use openvm_sha2::{Digest, Sha256};
use revm::{
    install_crypto,
    precompile::{
        bls12_381::{
            G1Point as BlsG1Point, G1PointScalar as BlsG1PointScalar, G2Point as BlsG2Point,
            G2PointScalar as BlsG2PointScalar,
        },
        bls12_381_const::{
            FP_LENGTH as BLS_FP_LEN, G1_LENGTH as BLS_G1_LEN, G2_LENGTH as BLS_G2_LEN,
            SCALAR_LENGTH as BLS_SCALAR_LEN,
        },
        Crypto, PrecompileHalt,
    },
};

// BN254 constants
const BN_FQ_LEN: usize = 32;
const BN_G1_LEN: usize = 64;
const BN_G2_LEN: usize = 128;
/// BN_SCALAR_LEN specifies the number of bytes needed to represent an Fr element.
/// This is an element in the scalar field of BN254.
const BN_SCALAR_LEN: usize = 32;

/// OpenVM k256 backend for Alloy crypto operations (transaction validation)
#[derive(Debug, Default)]
struct OpenVmK256Provider;

impl CryptoProvider for OpenVmK256Provider {
    fn recover_signer_unchecked(
        &self,
        sig: &[u8; 65],
        msg: &[u8; 32],
    ) -> Result<Address, RecoveryError> {
        // Extract components: sig[0..32]=r, sig[32..64]=s, sig[64]=recovery_id
        // Parse signature using OpenVM k256
        let mut signature = Signature::from_slice(&sig[..64]).map_err(|_| RecoveryError::new())?;

        // Normalize signature if needed
        let mut recid = sig[64];
        if let Some(sig_normalized) = signature.normalize_s() {
            signature = sig_normalized;
            recid ^= 1;
        }

        // Create recovery ID
        let recovery_id = RecoveryId::from_byte(recid).ok_or(RecoveryError::new())?;

        // Recover public key using OpenVM
        let recovered_key =
            VerifyingKey::recover_from_prehash_noverify(msg, &signature.to_bytes(), recovery_id)
                .map_err(|_| RecoveryError::new())?;

        // Hash the uncompressed SEC1 key without the 0x04 prefix.
        let public_key = recovered_key.to_encoded_point(false);
        let encoded_pubkey = &public_key.as_bytes()[1..65];

        // Hash to get Ethereum address
        let pubkey_hash = keccak256(encoded_pubkey);
        let address_bytes = &pubkey_hash[12..32]; // Last 20 bytes

        Ok(Address::from_slice(address_bytes))
    }

    fn verify_and_compute_signer_unchecked(
        &self,
        pubkey: &[u8; 65],
        sig: &[u8; 64],
        msg: &[u8; 32],
    ) -> Result<Address, RecoveryError> {
        let vk = VerifyingKey::from_sec1_bytes(pubkey).map_err(|_| RecoveryError::new())?;

        let mut signature = Signature::from_slice(sig).map_err(|_| RecoveryError::new())?;
        if let Some(sig_normalized) = signature.normalize_s() {
            signature = sig_normalized;
        }

        vk.verify_prehash(msg.as_ref(), &signature)
            .map_err(|_| RecoveryError::new())?;

        // Compute address directly from the provided pubkey bytes (skip 0x04 prefix)
        let pubkey_hash = keccak256(&pubkey[1..65]);
        Ok(Address::from_slice(&pubkey_hash[12..32]))
    }
}

/// OpenVM custom crypto implementation for faster precompiles
#[derive(Debug, Default)]
struct OpenVmCrypto;

impl Crypto for OpenVmCrypto {
    /// Custom SHA-256 implementation with openvm optimization
    fn sha256(&self, input: &[u8]) -> [u8; 32] {
        Sha256::digest(input).into()
    }

    /// Custom BN254 G1 addition with openvm optimization
    fn bn254_g1_add(&self, p1_bytes: &[u8], p2_bytes: &[u8]) -> Result<[u8; 64], PrecompileHalt> {
        let p1 = read_bn_g1_point(p1_bytes)?;
        let p2 = read_bn_g1_point(p2_bytes)?;
        let result = p1 + p2;
        Ok(encode_bn_g1_point(result))
    }

    /// Custom BN254 G1 scalar multiplication with openvm optimization
    fn bn254_g1_mul(
        &self,
        point_bytes: &[u8],
        scalar_bytes: &[u8],
    ) -> Result<[u8; 64], PrecompileHalt> {
        let p = read_bn_g1_point(point_bytes)?;
        let s = read_bn_scalar(scalar_bytes);
        let result = Bn254::msm(&[s], &[p]);
        Ok(encode_bn_g1_point(result))
    }

    /// Custom BN254 pairing check with openvm optimization
    fn bn254_pairing_check(&self, pairs: &[(&[u8], &[u8])]) -> Result<bool, PrecompileHalt> {
        if pairs.is_empty() {
            return Ok(true);
        }
        let mut g1_points = Vec::with_capacity(pairs.len());
        let mut g2_points = Vec::with_capacity(pairs.len());

        for (g1_bytes, g2_bytes) in pairs {
            let g1 = read_bn_g1_point(g1_bytes)?;
            let g2 = read_bn_g2_point(g2_bytes)?;

            let (g1_x, g1_y) = g1.into_coords();
            let g1 = AffinePoint::new(g1_x, g1_y);

            let (g2_x, g2_y) = g2.into_coords();
            let g2 = AffinePoint::new(g2_x, g2_y);

            g1_points.push(g1);
            g2_points.push(g2);
        }

        let pairing_result = Bn254::pairing_check(&g1_points, &g2_points).is_ok();
        Ok(pairing_result)
    }

    /// Custom BLS12-381 G1 addition with openvm optimization
    fn bls12_381_g1_add(
        &self,
        a: BlsG1Point,
        b: BlsG1Point,
    ) -> Result<[u8; BLS_G1_LEN], PrecompileHalt> {
        // EIP-2537 G1ADD validates on-curve only, not subgroup membership.
        let p1 = read_bls_g1_point_no_subgroup_check(&a)?;
        let p2 = read_bls_g1_point_no_subgroup_check(&b)?;
        let sum = p1 + p2;
        Ok(encode_bls_g1_point(&sum))
    }

    /// Custom BLS12-381 G1 MSM with openvm optimization
    fn bls12_381_g1_msm(
        &self,
        pairs: &mut dyn Iterator<Item = Result<BlsG1PointScalar, PrecompileHalt>>,
    ) -> Result<[u8; BLS_G1_LEN], PrecompileHalt> {
        let mut scalars = Vec::new();
        let mut points = Vec::new();

        for pair in pairs {
            let (point_bytes, scalar_bytes) = pair?;
            points.push(read_bls_g1_point(&point_bytes)?);
            scalars.push(read_bls_scalar(&scalar_bytes));
        }

        if points.is_empty() {
            return Ok([0u8; BLS_G1_LEN]);
        }

        let result = Bls12_381::msm(&scalars, &points);
        Ok(encode_bls_g1_point(&result))
    }

    /// Custom BLS12-381 G2 addition with openvm optimization
    fn bls12_381_g2_add(
        &self,
        a: BlsG2Point,
        b: BlsG2Point,
    ) -> Result<[u8; BLS_G2_LEN], PrecompileHalt> {
        // EIP-2537 G2ADD validates on-curve only, not subgroup membership.
        let p1 = read_bls_g2_point_no_subgroup_check(&a)?;
        let p2 = read_bls_g2_point_no_subgroup_check(&b)?;
        let sum = p1 + p2;
        Ok(encode_bls_g2_point(&sum))
    }

    /// Custom BLS12-381 G2 MSM with openvm optimization
    fn bls12_381_g2_msm(
        &self,
        pairs: &mut dyn Iterator<Item = Result<BlsG2PointScalar, PrecompileHalt>>,
    ) -> Result<[u8; BLS_G2_LEN], PrecompileHalt> {
        let mut scalars = Vec::new();
        let mut points = Vec::new();

        for pair in pairs {
            let (point_bytes, scalar_bytes) = pair?;
            points.push(read_bls_g2_point(&point_bytes)?);
            scalars.push(read_bls_scalar(&scalar_bytes));
        }

        if points.is_empty() {
            return Ok([0u8; BLS_G2_LEN]);
        }

        // directly using openvm_ecc_guest::msm here
        let result = openvm_ecc_guest::msm(&scalars, &points);
        Ok(encode_bls_g2_point(&result))
    }

    /// Custom BLS12-381 pairing check with openvm optimization
    fn bls12_381_pairing_check(
        &self,
        pairs: &[(BlsG1Point, BlsG2Point)],
    ) -> Result<bool, PrecompileHalt> {
        if pairs.is_empty() {
            return Ok(true);
        }

        let mut g1_points = Vec::with_capacity(pairs.len());
        let mut g2_points = Vec::with_capacity(pairs.len());

        for (g1_bytes, g2_bytes) in pairs {
            let g1 = read_bls_g1_point(g1_bytes)?;
            let g2 = read_bls_g2_point(g2_bytes)?;

            let (g1_x, g1_y) = g1.into_coords();
            let (g2_x, g2_y) = g2.into_coords();

            g1_points.push(AffinePoint::new(g1_x, g1_y));
            g2_points.push(AffinePoint::new(g2_x, g2_y));
        }

        let pairing_result = Bls12_381::pairing_check(&g1_points, &g2_points).is_ok();
        Ok(pairing_result)
    }

    /// Custom secp256k1 ECDSA signature recovery with openvm optimization
    fn secp256k1_ecrecover(
        &self,
        sig_bytes: &[u8; 64],
        mut recid: u8,
        msg_hash: &[u8; 32],
    ) -> Result<[u8; 32], PrecompileHalt> {
        let mut sig = Signature::from_slice(sig_bytes)
            .map_err(|_| PrecompileHalt::other("Invalid signature format"))?;

        if let Some(sig_normalized) = sig.normalize_s() {
            sig = sig_normalized;
            recid ^= 1;
        }

        let recovery_id = RecoveryId::from_byte(recid)
            .ok_or_else(|| PrecompileHalt::other("Invalid recovery ID"))?;

        let recovered_key =
            VerifyingKey::recover_from_prehash_noverify(msg_hash, &sig.to_bytes(), recovery_id)
                .map_err(|_| PrecompileHalt::other("Key recovery failed"))?;

        let public_key = recovered_key.to_encoded_point(false);
        let encoded_pubkey = &public_key.as_bytes()[1..65];

        let pubkey_hash = keccak256(encoded_pubkey);
        let mut address = [0u8; 32];
        address[12..].copy_from_slice(&pubkey_hash[12..]);

        Ok(address)
    }

    /// Custom secp256r1 signature verification with openvm optimization
    fn secp256r1_verify_signature(&self, msg: &[u8; 32], sig: &[u8; 64], pk: &[u8; 64]) -> bool {
        use openvm_p256::{
            ecdsa::{signature::hazmat::PrehashVerifier, Signature, VerifyingKey},
            EncodedPoint,
        };

        // Can fail only if the input is not exact length.
        let Ok(signature) = Signature::from_slice(sig) else {
            return false;
        };
        // Decode the public key bytes (x,y coordinates) using EncodedPoint
        let encoded_point = EncodedPoint::from_untagged_bytes(&(*pk).into());
        // Create VerifyingKey from the encoded point
        let Ok(public_key) = VerifyingKey::from_encoded_point(&encoded_point) else {
            return false;
        };

        public_key.verify_prehash(msg, &signature).is_ok()
    }

    /// Custom KZG point evaluation with configurable backends
    fn verify_kzg_proof(
        &self,
        z: &[u8; 32],
        y: &[u8; 32],
        commitment: &[u8; 48],
        proof: &[u8; 48],
    ) -> Result<(), PrecompileHalt> {
        let env = openvm_kzg::EnvKzgSettings::default();
        let kzg_settings = env.get();

        let commitment_bytes = Bytes48::from_slice(commitment)
            .map_err(|_| PrecompileHalt::other("invalid commitment bytes"))?;
        let z_bytes =
            Bytes32::from_slice(z).map_err(|_| PrecompileHalt::other("invalid z bytes"))?;
        let y_bytes =
            Bytes32::from_slice(y).map_err(|_| PrecompileHalt::other("invalid y bytes"))?;
        let proof_bytes =
            Bytes48::from_slice(proof).map_err(|_| PrecompileHalt::other("invalid proof bytes"))?;

        let valid = KzgProof::verify_kzg_proof(
            &commitment_bytes,
            &z_bytes,
            &y_bytes,
            &proof_bytes,
            kzg_settings,
        )
        .map_err(|_| PrecompileHalt::other("openvm kzg proof verification failed"))?;
        if valid {
            Ok(())
        } else {
            Err(PrecompileHalt::BlobVerifyKzgProofFailed)
        }
    }

    /// Custom modular exponentiation with BN254 Fr acceleration
    fn modexp(&self, base: &[u8], exp: &[u8], modulus: &[u8]) -> Result<Vec<u8>, PrecompileHalt> {
        if is_bn254_fr(modulus) {
            return Ok(accelerated_modexp_bn254_fr(base, exp));
        }
        Ok(aurora_engine_modexp::modexp(base, exp, modulus))
    }
}

/// Returns true if the modulus (big-endian, possibly with leading zeros) equals BN254 Fr.
fn is_bn254_fr(modulus: &[u8]) -> bool {
    // Strip leading zeros
    let stripped = match modulus.iter().position(|&b| b != 0) {
        Some(i) => &modulus[i..],
        None => return false, // all zeros
    };
    // bn::Scalar::MODULUS is little-endian; compare against reversed input
    stripped.len() == BN_SCALAR_LEN
        && stripped
            .iter()
            .rev()
            .eq(bn::Scalar::MODULUS.as_ref().iter())
}

/// Accelerated modexp for BN254 Fr using field arithmetic intrinsics.
fn accelerated_modexp_bn254_fr(base: &[u8], exp: &[u8]) -> Vec<u8> {
    use openvm_ecc_guest::algebra::{ExpBytes, Reduce};

    // OpenVM's field reduction requires inputs to be aligned to the field byte size.
    let padded_len = base
        .len()
        .next_multiple_of(BN_SCALAR_LEN)
        .max(BN_SCALAR_LEN);
    let mut padded = vec![0u8; padded_len];
    padded[padded_len - base.len()..].copy_from_slice(base);
    let base_fr = bn::Scalar::reduce_be_bytes(&padded);

    base_fr.exp_bytes(true, exp).to_be_bytes().as_ref().to_vec()
}

/// Install OpenVM crypto implementations globally
pub fn install_openvm_crypto() -> Result<bool, Box<dyn std::error::Error>> {
    // Install OpenVM k256 provider for Alloy (transaction validation)
    install_default_provider(Arc::new(OpenVmK256Provider))?;

    // Install OpenVM crypto for REVM precompiles
    let installed = install_crypto(OpenVmCrypto);

    Ok(installed)
}

// Helper functions for BN254 operations

#[inline]
fn read_bn_fq(input: &[u8]) -> Result<bn::Fp, PrecompileHalt> {
    if input.len() < BN_FQ_LEN {
        Err(PrecompileHalt::Bn254FieldPointNotAMember)
    } else {
        bn::Fp::from_be_bytes(&input[..BN_FQ_LEN]).ok_or(PrecompileHalt::Bn254FieldPointNotAMember)
    }
}

#[inline]
fn read_bn_fq2(input: &[u8]) -> Result<bn::Fp2, PrecompileHalt> {
    let y = read_bn_fq(&input[..BN_FQ_LEN])?;
    let x = read_bn_fq(&input[BN_FQ_LEN..BN_FQ_LEN * 2])?;
    Ok(bn::Fp2::new(x, y))
}

#[inline]
fn read_bn_g1_point(input: &[u8]) -> Result<bn::G1Affine, PrecompileHalt> {
    if input.len() != BN_G1_LEN {
        return Err(PrecompileHalt::Bn254PairLength);
    }
    let px = read_bn_fq(&input[0..BN_FQ_LEN])?;
    let py = read_bn_fq(&input[BN_FQ_LEN..BN_G1_LEN])?;
    // SAFETY: `read_bn_fq` produces canonical Fp elements; `from_xy` itself checks the curve
    // equation and returns `None` if `(px, py)` is not on the curve.
    let point = unsafe { bn::G1Affine::from_xy(px, py) }
        .ok_or(PrecompileHalt::Bn254AffineGFailedToCreate)?;
    if point.is_in_correct_subgroup() {
        Ok(point)
    } else {
        Err(PrecompileHalt::Bn254AffineGFailedToCreate)
    }
}

#[inline]
fn read_bn_g2_point(input: &[u8]) -> Result<bn::G2Affine, PrecompileHalt> {
    if input.len() != BN_G2_LEN {
        return Err(PrecompileHalt::Bn254PairLength);
    }
    let c0 = read_bn_fq2(&input[0..BN_G1_LEN])?;
    let c1 = read_bn_fq2(&input[BN_G1_LEN..BN_G2_LEN])?;
    // SAFETY: `read_bn_fq2` produces canonical Fp2 elements; `from_xy` itself checks the curve
    // equation and returns `None` if `(c0, c1)` is not on the twist.
    let point = unsafe { bn::G2Affine::from_xy(c0, c1) }
        .ok_or(PrecompileHalt::Bn254AffineGFailedToCreate)?;
    if point.is_in_correct_subgroup() {
        Ok(point)
    } else {
        Err(PrecompileHalt::Bn254AffineGFailedToCreate)
    }
}

#[inline]
fn encode_bn_g1_point(point: bn::G1Affine) -> [u8; BN_G1_LEN] {
    let mut output = [0u8; BN_G1_LEN];

    let x_bytes: &[u8] = point.x().as_le_bytes();
    let y_bytes: &[u8] = point.y().as_le_bytes();
    for i in 0..BN_FQ_LEN {
        output[i] = x_bytes[BN_FQ_LEN - 1 - i];
        output[i + BN_FQ_LEN] = y_bytes[BN_FQ_LEN - 1 - i];
    }
    output
}

/// Reads a scalar from the input slice
///
/// Note: The scalar does not need to be canonical.
///
/// # Panics
///
/// If `input.len()` is not equal to [`BN_SCALAR_LEN`].
#[inline]
fn read_bn_scalar(input: &[u8]) -> bn::Scalar {
    assert_eq!(
        input.len(),
        BN_SCALAR_LEN,
        "unexpected scalar length. got {}, expected {BN_SCALAR_LEN}",
        input.len()
    );
    bn::Scalar::from_be_bytes_unchecked(input)
}

// Helper functions for BLS12-381 operations

#[inline]
fn read_bls_fp(input: &[u8]) -> Result<bls::Fp, PrecompileHalt> {
    if input.len() != BLS_FP_LEN {
        return Err(PrecompileHalt::other("invalid BLS12-381 fp length"));
    }
    bls::Fp::from_be_bytes(input)
        .ok_or_else(|| PrecompileHalt::other("element not in BLS12-381 base field"))
}

#[inline]
fn read_bls_fp2(c0: &[u8], c1: &[u8]) -> Result<bls::Fp2, PrecompileHalt> {
    let real = read_bls_fp(c0)?;
    let imag = read_bls_fp(c1)?;
    Ok(bls::Fp2::new(real, imag))
}

#[inline]
fn read_bls_g1_point_no_subgroup_check(
    point: &BlsG1Point,
) -> Result<bls::G1Affine, PrecompileHalt> {
    let px = read_bls_fp(&point.0)?;
    let py = read_bls_fp(&point.1)?;
    // SAFETY: `read_bls_fp` produces canonical Fp elements; `from_xy` itself checks the curve
    // equation and returns `None` if `(px, py)` is not on the curve.
    unsafe { bls::G1Affine::from_xy(px, py) }.ok_or(PrecompileHalt::Bls12381G1NotOnCurve)
}

#[inline]
fn read_bls_g1_point(point: &BlsG1Point) -> Result<bls::G1Affine, PrecompileHalt> {
    let point = read_bls_g1_point_no_subgroup_check(point)?;
    if point.is_in_correct_subgroup() {
        Ok(point)
    } else {
        Err(PrecompileHalt::Bls12381G1NotInSubgroup)
    }
}

#[inline]
fn read_bls_g2_point_no_subgroup_check(
    point: &BlsG2Point,
) -> Result<bls::G2Affine, PrecompileHalt> {
    let x = read_bls_fp2(&point.0, &point.1)?;
    let y = read_bls_fp2(&point.2, &point.3)?;
    // SAFETY: `read_bls_fp2` produces canonical Fp2 elements; `from_xy` itself checks the curve
    // equation and returns `None` if `(x, y)` is not on the twist.
    unsafe { bls::G2Affine::from_xy(x, y) }.ok_or(PrecompileHalt::Bls12381G2NotOnCurve)
}

#[inline]
fn read_bls_g2_point(point: &BlsG2Point) -> Result<bls::G2Affine, PrecompileHalt> {
    let point = read_bls_g2_point_no_subgroup_check(point)?;
    if point.is_in_correct_subgroup() {
        Ok(point)
    } else {
        Err(PrecompileHalt::Bls12381G2NotInSubgroup)
    }
}

#[inline]
fn read_bls_scalar(input: &[u8]) -> bls::Scalar {
    assert_eq!(
        input.len(),
        BLS_SCALAR_LEN,
        "unexpected scalar length. got {}, expected {BLS_SCALAR_LEN}",
        input.len()
    );
    bls::Scalar::from_be_bytes_unchecked(input)
}

#[inline]
fn encode_bls_g1_point(point: &bls::G1Affine) -> [u8; BLS_G1_LEN] {
    if point.is_identity() {
        return [0u8; BLS_G1_LEN];
    }

    let mut output = [0u8; BLS_G1_LEN];
    let x_bytes: &[u8] = point.x().as_le_bytes();
    let y_bytes: &[u8] = point.y().as_le_bytes();
    for i in 0..BLS_FP_LEN {
        output[i] = x_bytes[BLS_FP_LEN - 1 - i];
        output[i + BLS_FP_LEN] = y_bytes[BLS_FP_LEN - 1 - i];
    }
    output
}

#[inline]
fn encode_bls_g2_point(point: &bls::G2Affine) -> [u8; BLS_G2_LEN] {
    if point.is_identity() {
        return [0u8; BLS_G2_LEN];
    }

    let mut output = [0u8; BLS_G2_LEN];
    let x = point.x();
    let y = point.y();
    let x_c0 = x.c0.as_le_bytes();
    let x_c1 = x.c1.as_le_bytes();
    let y_c0 = y.c0.as_le_bytes();
    let y_c1 = y.c1.as_le_bytes();
    for i in 0..BLS_FP_LEN {
        output[i] = x_c0[BLS_FP_LEN - 1 - i];
        output[i + BLS_FP_LEN] = x_c1[BLS_FP_LEN - 1 - i];
        output[i + (2 * BLS_FP_LEN)] = y_c0[BLS_FP_LEN - 1 - i];
        output[i + (3 * BLS_FP_LEN)] = y_c1[BLS_FP_LEN - 1 - i];
    }
    output
}
