//! Default `wild:` host ops installed into a [`PrimRegistry`] (S-HOST-REGISTRY / WP-4 L-HOST).
//!
//! # Feature
//!
//! Gated behind **`host-registry`**. Pure `OsEntropy` / `OsClock` users do not pull
//! `mycelium-interp`.
//!
//! # I/O model — blocking-hypha
//!
//! Every host fn here **may block** the calling OS thread (floor `fill_bytes` opens
//! `/dev/urandom`; clock reads are short). No reactor in v0 (spike S1).
//!
//! # Value encoding (v0 pragmatic — document widths; zipper bump if changed)
//!
//! | wild name | Args | Result | Encoding |
//! |-----------|------|--------|----------|
//! | `time_mono_nanos` | `()` | `Binary{64}` | unsigned MSB-first (`mycelium_core::binary::uint_to_bits`) of `std_sys::time::mono_nanos` |
//! | `time_wall_nanos` | `()` | `Binary{64}` | unsigned MSB-first of `u64` from `wall_nanos()`; `u128→u64` overflow or pre-epoch floor err → explicit [`EvalError::PrimType`] |
//! | `rand_fill` | `(Binary{W})` | `Bytes` | length = unsigned magnitude of the Binary operand (checked, never wrap); payload `Payload::Bytes` |
//!
//! # Guarantee (VR-5 / G2)
//!
//! Ambient OS reads are tagged **`Declared`** with a zero-magnitude `UserDeclared` error bound
//! (same posture as host ambient sources elsewhere). OS failures never silent-zero entropy and
//! never wrap clocks.
//!
//! # Catalog names
//!
//! Registered as bare names (`time_mono_nanos`, …); [`PrimRegistry::register_host`] stores them
//! under `wild:{name}`.

use mycelium_core::{
    binary, Bound, BoundBasis, BoundKind, GuaranteeStrength, Meta, NormKind, Payload, Provenance,
    Repr, Value,
};
use mycelium_interp::{install_host_ops, prims::PrimFn, EvalError, PrimRegistry};

/// Install the v0 floor-backed default host ops into `reg`.
///
/// Registers at least:
/// - `wild:time_mono_nanos`
/// - `wild:time_wall_nanos`
/// - `wild:rand_fill`
///
/// Last registration for a name wins (same rule as [`install_host_ops`]). Safe to call after
/// [`PrimRegistry::with_builtins`] — builtins grant zero `wild:` ops.
pub fn install_default_host_ops(reg: &mut PrimRegistry) {
    install_host_ops(
        reg,
        &[
            ("time_mono_nanos", host_time_mono_nanos as PrimFn),
            ("time_wall_nanos", host_time_wall_nanos as PrimFn),
            ("rand_fill", host_rand_fill as PrimFn),
        ],
    );
}

// --- encoding helpers ---------------------------------------------------------------------------

/// Ambient-host result meta: `Declared` + zero-ε `UserDeclared` bound (M-I4), provenance `Root`
/// (no Core-IR inputs — the value is OS-sourced).
fn host_declared_meta() -> Result<Meta, EvalError> {
    let bound = Bound {
        kind: BoundKind::Error {
            eps: 0.0,
            norm: NormKind::Linf,
        },
        basis: BoundBasis::UserDeclared,
    };
    Meta::new(
        Provenance::Root,
        GuaranteeStrength::Declared,
        Some(bound),
        None,
        None,
        None,
    )
    .map_err(EvalError::Wf)
}

/// `u64` → `Binary{64}` MSB-first (unsigned). `u64` always fits 64 bits.
fn u64_as_binary64(ns: u64) -> Result<Value, EvalError> {
    let bits = binary::uint_to_bits(ns, 64).ok_or_else(|| EvalError::PrimType {
        prim: "wild:time_encode".to_owned(),
        why: "internal: u64 did not fit Binary{64} (unreachable)".to_owned(),
    })?;
    Value::new(
        Repr::Binary { width: 64 },
        Payload::Bits(bits),
        host_declared_meta()?,
    )
    .map_err(EvalError::Wf)
}

fn expect_arity(prim: &str, args: &[&Value], n: usize) -> Result<(), EvalError> {
    if args.len() == n {
        Ok(())
    } else {
        Err(EvalError::PrimType {
            prim: prim.to_owned(),
            why: format!("expected {n} argument(s), got {}", args.len()),
        })
    }
}

/// Unsigned magnitude of a `Binary{W}` length/index (MSB-first), checked — never wrap (G2).
fn binary_as_usize(prim: &str, v: &Value) -> Result<usize, EvalError> {
    let bits = match (v.repr(), v.payload()) {
        (Repr::Binary { .. }, Payload::Bits(b)) => b.as_slice(),
        _ => {
            return Err(EvalError::PrimType {
                prim: prim.to_owned(),
                why: "expected a Binary length operand".to_owned(),
            });
        }
    };
    let mut n: u128 = 0;
    for &b in bits {
        n = n
            .checked_shl(1)
            .and_then(|x| x.checked_add(u128::from(b)))
            .ok_or_else(|| EvalError::PrimType {
                prim: prim.to_owned(),
                why: "Binary length magnitude overflowed u128".to_owned(),
            })?;
    }
    usize::try_from(n).map_err(|_| EvalError::PrimType {
        prim: prim.to_owned(),
        why: format!("Binary length {n} does not fit usize on this host"),
    })
}

// --- host ops -----------------------------------------------------------------------------------

/// `wild:time_mono_nanos : () → Binary{64}` — total monotonic floor read.
fn host_time_mono_nanos(prim: &str, args: &[&Value]) -> Result<Value, EvalError> {
    expect_arity(prim, args, 0)?;
    let ns = mycelium_std_sys::time::mono_nanos();
    u64_as_binary64(ns)
}

/// `wild:time_wall_nanos : () → Binary{64}` — wall clock; OS/pre-epoch/overflow → explicit err.
fn host_time_wall_nanos(prim: &str, args: &[&Value]) -> Result<Value, EvalError> {
    expect_arity(prim, args, 0)?;
    let ns_u128 = mycelium_std_sys::time::wall_nanos().map_err(|e| EvalError::PrimType {
        prim: prim.to_owned(),
        why: format!("wall clock unavailable: {e}"),
    })?;
    let ns_u64 = u64::try_from(ns_u128).map_err(|_| EvalError::PrimType {
        prim: prim.to_owned(),
        why: format!(
            "wall_nanos {ns_u128} exceeds Binary{{64}} (u64::MAX); refused, never truncated (G2)"
        ),
    })?;
    u64_as_binary64(ns_u64)
}

/// `wild:rand_fill : (Binary{{W}}) → Bytes` — fill `len` bytes from OS entropy; never silent zero.
fn host_rand_fill(prim: &str, args: &[&Value]) -> Result<Value, EvalError> {
    expect_arity(prim, args, 1)?;
    let len = binary_as_usize(prim, args[0])?;
    let mut buf = vec![0u8; len];
    mycelium_std_sys::rand::fill_bytes(&mut buf).map_err(|e| EvalError::PrimType {
        prim: prim.to_owned(),
        why: format!("entropy fill failed: {e}"),
    })?;
    Value::new(Repr::Bytes, Payload::Bytes(buf), host_declared_meta()?).map_err(EvalError::Wf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelium_core::{Meta, Payload, Provenance, Repr, Value};
    use mycelium_interp::PrimRegistry;

    fn bin_len(n: u64, width: u32) -> Value {
        let bits = binary::uint_to_bits(n, width).expect("fits");
        Value::new(
            Repr::Binary { width },
            Payload::Bits(bits),
            Meta::exact(Provenance::Root),
        )
        .expect("wf")
    }

    fn bits_to_u64(v: &Value) -> u64 {
        match v.payload() {
            Payload::Bits(b) => binary::bits_to_uint(b),
            _ => panic!("expected Bits"),
        }
    }

    #[test]
    fn install_registers_catalog_names() {
        let mut reg = PrimRegistry::with_builtins();
        assert!(!reg.has_host("time_mono_nanos"));
        assert!(!reg.has_host("time_wall_nanos"));
        assert!(!reg.has_host("rand_fill"));
        assert!(!reg.has_host("wild:time_mono_nanos"));

        install_default_host_ops(&mut reg);

        assert!(reg.has_host("time_mono_nanos"));
        assert!(reg.has_host("wild:time_mono_nanos"));
        assert!(reg.has_host("time_wall_nanos"));
        assert!(reg.has_host("wild:time_wall_nanos"));
        assert!(reg.has_host("rand_fill"));
        assert!(reg.has_host("wild:rand_fill"));

        // Catalog keys appear under wild: in the name list.
        let names = reg.names();
        assert!(names.contains(&"wild:time_mono_nanos"));
        assert!(names.contains(&"wild:time_wall_nanos"));
        assert!(names.contains(&"wild:rand_fill"));
    }

    #[test]
    fn time_mono_nanos_is_total_binary64() {
        let mut reg = PrimRegistry::empty();
        install_default_host_ops(&mut reg);
        let f = reg.get("wild:time_mono_nanos").expect("registered");
        let v = f("wild:time_mono_nanos", &[]).expect("mono is total");
        assert_eq!(*v.repr(), Repr::Binary { width: 64 });
        assert_eq!(v.meta().guarantee(), GuaranteeStrength::Declared);
        // A second read is non-decreasing (floor Instant).
        let v2 = f("wild:time_mono_nanos", &[]).expect("mono is total");
        assert!(bits_to_u64(&v2) >= bits_to_u64(&v));
    }

    #[test]
    fn time_wall_nanos_binary64_or_explicit_err() {
        let mut reg = PrimRegistry::empty();
        install_default_host_ops(&mut reg);
        let f = reg.get("wild:time_wall_nanos").expect("registered");
        match f("wild:time_wall_nanos", &[]) {
            Ok(v) => {
                assert_eq!(*v.repr(), Repr::Binary { width: 64 });
                assert!(bits_to_u64(&v) > 0, "post-epoch wall");
            }
            Err(EvalError::PrimType { prim, why }) => {
                assert_eq!(prim, "wild:time_wall_nanos");
                assert!(
                    why.contains("wall") || why.contains("Binary"),
                    "explicit OS/encoding failure: {why}"
                );
            }
            Err(other) => panic!("unexpected error shape: {other:?}"),
        }
    }

    #[test]
    fn rand_fill_bytes_or_explicit_unavailable() {
        let mut reg = PrimRegistry::empty();
        install_default_host_ops(&mut reg);
        let f = reg.get("wild:rand_fill").expect("registered");
        let len = bin_len(32, 32);
        match f("wild:rand_fill", &[&len]) {
            Ok(v) => {
                assert_eq!(*v.repr(), Repr::Bytes);
                let bytes = v.bytes().expect("Bytes payload");
                assert_eq!(bytes.len(), 32);
                // Smoke: CSPRNG all-zero is astronomically unlikely; still not a hard contract.
                assert!(
                    bytes.iter().any(|&b| b != 0),
                    "entropy fill returned all-zero (smoke)"
                );
            }
            Err(EvalError::PrimType { prim, why }) => {
                assert_eq!(prim, "wild:rand_fill");
                assert!(
                    why.contains("entropy"),
                    "must name entropy failure, never silent zero: {why}"
                );
            }
            Err(other) => panic!("unexpected error shape: {other:?}"),
        }
    }

    #[test]
    fn rand_fill_zero_len_is_empty_bytes() {
        let mut reg = PrimRegistry::empty();
        install_default_host_ops(&mut reg);
        let f = reg.get("wild:rand_fill").expect("registered");
        let len = bin_len(0, 8);
        match f("wild:rand_fill", &[&len]) {
            Ok(v) => {
                assert_eq!(v.bytes().expect("Bytes").len(), 0);
            }
            Err(EvalError::PrimType { why, .. }) if why.contains("entropy") => {
                // Platform without /dev/urandom — empty fill still hits the floor; empty is Ok
                // on the floor, but if a future floor changes, stay never-silent.
            }
            Err(other) => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn arity_errors_are_explicit() {
        let mut reg = PrimRegistry::empty();
        install_default_host_ops(&mut reg);
        let mono = reg.get("wild:time_mono_nanos").unwrap();
        let junk = bin_len(1, 8);
        let err = mono("wild:time_mono_nanos", &[&junk]).unwrap_err();
        assert!(
            matches!(err, EvalError::PrimType { .. }),
            "arity mismatch must be PrimType, got {err:?}"
        );

        let fill = reg.get("wild:rand_fill").unwrap();
        let err = fill("wild:rand_fill", &[]).unwrap_err();
        assert!(matches!(err, EvalError::PrimType { .. }));
    }

    #[test]
    fn rand_fill_rejects_non_binary_len() {
        let mut reg = PrimRegistry::empty();
        install_default_host_ops(&mut reg);
        let f = reg.get("wild:rand_fill").unwrap();
        let not_bin = Value::new(
            Repr::Bytes,
            Payload::Bytes(vec![1, 2, 3]),
            Meta::exact(Provenance::Root),
        )
        .unwrap();
        let err = f("wild:rand_fill", &[&not_bin]).unwrap_err();
        match err {
            EvalError::PrimType { why, .. } => {
                assert!(why.contains("Binary"), "{why}");
            }
            other => panic!("expected PrimType, got {other:?}"),
        }
    }
}
