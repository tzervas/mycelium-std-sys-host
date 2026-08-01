//! Default `wild:` host ops installed into a [`PrimRegistry`] (S-HOST-REGISTRY / WP-4–WP-5).
//!
//! # Feature
//!
//! Gated behind **`host-registry`**. Pure `OsEntropy` / `OsClock` users do not pull
//! `mycelium-interp`.
//!
//! # I/O model — blocking-hypha
//!
//! Every host fn here **may block** the calling OS thread (floor `fill_bytes` opens
//! `/dev/urandom`; clock reads are short; `process_wait` blocks until the child exits).
//! No reactor in v0 (spike S1).
//!
//! # Value encoding (v0 pragmatic — document widths; zipper bump if changed)
//!
//! | wild name | Args | Result | Encoding |
//! |-----------|------|--------|----------|
//! | `time_mono_nanos` | `()` | `Binary{64}` | unsigned MSB-first (`mycelium_core::binary::uint_to_bits`) of `std_sys::time::mono_nanos` |
//! | `time_wall_nanos` | `()` | `Binary{64}` | unsigned MSB-first of `u64` from `wall_nanos()`; `u128→u64` overflow or pre-epoch floor err → explicit [`EvalError::PrimType`] |
//! | `rand_fill` | `(Binary{W})` | `Bytes` | length = unsigned magnitude of the Binary operand (checked, never wrap); payload `Payload::Bytes` |
//! | `process_spawn` | `(Bytes)` or `(Bytes, Seq{Bytes})` | `Binary{64}` | handle id from host [`ProcessTable`] (ids start at 1; 0 reserved) |
//! | `process_wait` | `(Binary{64})` | `Seq{Binary{32};3}` | exit-status triple (see below); successful wait **removes** the handle |
//! | `process_kill` | `(Binary{64})` | `Bytes` (empty) | unit; does **not** remove the handle (still needs wait to reap) |
//!
//! ## `process_wait` status encoding (`Seq` of three `Binary{32}`, MSB-first)
//!
//! Index layout (homogeneous `Seq` — mixed widths are not representable):
//!
//! | index | field | meaning |
//! |-------|-------|---------|
//! | 0 | `success` | `1` if OS reports successful exit, else `0` |
//! | 1 | `kind` | `0` = exited with code; `1` = terminated by signal; `2` = neither (explicit, never fabricated) |
//! | 2 | `detail` | when `kind=0`: exit code as two's-complement `i32` bits; when `kind=1`: signal number as two's-complement `i32`; when `kind=2`: `0` |
//!
//! **G2:** signal death is never rewritten as exit code `0` — `kind=1` + signal in `detail`.
//!
//! Errors map to [`EvalError::PrimType`] with `prim: "wild:process_*"` and
//! [`ProcessError`](mycelium_std_sys::process::ProcessError) in `why` (same posture as
//! `time_wall_nanos` / `rand_fill`).
//!
//! # Stateful host table
//!
//! [`PrimFn`] is a pure function pointer (no `HostCtx` yet). Live children live in a process-level
//! [`Mutex`]`<`[`ProcessTable`]`>` (see `docs/PROCESS-HOST-HOOKS.md` in `mycelium-std-sys`).
//!
//! # Guarantee (VR-5 / G2)
//!
//! Ambient OS reads / process lifecycle are tagged **`Declared`** with a zero-magnitude
//! `UserDeclared` error bound. OS failures never silent-zero entropy, never wrap clocks, and
//! never no-op unknown process handles.
//!
//! # Catalog names
//!
//! Registered as bare names (`time_mono_nanos`, `process_spawn`, …);
//! [`PrimRegistry::register_host`] stores them under `wild:{name}`.

use std::ffi::OsString;
use std::sync::{Mutex, OnceLock};

use mycelium_core::{
    binary, Bound, BoundBasis, BoundKind, GuaranteeStrength, Meta, NormKind, Payload, Provenance,
    Repr, Value,
};
use mycelium_interp::{install_host_ops, prims::PrimFn, EvalError, PrimRegistry};
use mycelium_std_sys::process::{ExitStatus, ProcessError, ProcessTable, SpawnOpts};

/// Install the v0 floor-backed default host ops into `reg`.
///
/// Registers at least:
/// - `wild:time_mono_nanos`
/// - `wild:time_wall_nanos`
/// - `wild:rand_fill`
/// - `wild:process_spawn`
/// - `wild:process_wait`
/// - `wild:process_kill`
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
    install_process_host_ops(reg);
}

/// Install only the WP-5 process host ops (`wild:process_spawn` / `wait` / `kill`).
///
/// Called from [`install_default_host_ops`]; also usable alone for embedders that want process
/// without re-installing time/rand.
pub fn install_process_host_ops(reg: &mut PrimRegistry) {
    install_host_ops(
        reg,
        &[
            ("process_spawn", host_process_spawn as PrimFn),
            ("process_wait", host_process_wait as PrimFn),
            ("process_kill", host_process_kill as PrimFn),
        ],
    );
}

// --- process table (process-level; PrimFn has no HostCtx yet) -----------------------------------

fn process_table() -> &'static Mutex<ProcessTable> {
    static TABLE: OnceLock<Mutex<ProcessTable>> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(ProcessTable::new()))
}

fn with_process_table<R>(
    prim: &str,
    f: impl FnOnce(&mut ProcessTable) -> Result<R, EvalError>,
) -> Result<R, EvalError> {
    let mut guard = process_table().lock().map_err(|_| EvalError::PrimType {
        prim: prim.to_owned(),
        why: "process table mutex poisoned (prior host panic)".to_owned(),
    })?;
    f(&mut guard)
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

/// `i32` → `Binary{32}` two's-complement MSB-first.
fn i32_as_binary32(prim: &str, v: i32) -> Result<Value, EvalError> {
    let bits = binary::int_to_bits(i64::from(v), 32).ok_or_else(|| EvalError::PrimType {
        prim: prim.to_owned(),
        why: format!("internal: i32 {v} did not fit Binary{{32}} (unreachable)"),
    })?;
    Value::new(
        Repr::Binary { width: 32 },
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

fn expect_arity_range(prim: &str, args: &[&Value], lo: usize, hi: usize) -> Result<(), EvalError> {
    if args.len() >= lo && args.len() <= hi {
        Ok(())
    } else {
        Err(EvalError::PrimType {
            prim: prim.to_owned(),
            why: format!("expected {lo}..={hi} argument(s), got {}", args.len()),
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

/// Unsigned magnitude of a `Binary{W}` as `u64` (handle ids), checked — never wrap (G2).
fn binary_as_u64(prim: &str, v: &Value) -> Result<u64, EvalError> {
    let bits = match (v.repr(), v.payload()) {
        (Repr::Binary { .. }, Payload::Bits(b)) => b.as_slice(),
        _ => {
            return Err(EvalError::PrimType {
                prim: prim.to_owned(),
                why: "expected a Binary handle id".to_owned(),
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
                why: "Binary handle magnitude overflowed u128".to_owned(),
            })?;
    }
    u64::try_from(n).map_err(|_| EvalError::PrimType {
        prim: prim.to_owned(),
        why: format!("Binary handle {n} does not fit u64"),
    })
}

fn value_as_bytes<'a>(prim: &str, v: &'a Value) -> Result<&'a [u8], EvalError> {
    v.bytes().ok_or_else(|| EvalError::PrimType {
        prim: prim.to_owned(),
        why: "expected Bytes operand".to_owned(),
    })
}

/// Bytes → OS program/arg path. UTF-8 required (explicit refuse on invalid UTF-8; G2).
fn bytes_as_os_string(prim: &str, bytes: &[u8]) -> Result<OsString, EvalError> {
    let s = std::str::from_utf8(bytes).map_err(|_| EvalError::PrimType {
        prim: prim.to_owned(),
        why: "program/arg Bytes are not valid UTF-8 (refused, never lossy-spawned)".to_owned(),
    })?;
    Ok(OsString::from(s))
}

fn process_err(prim: &str, e: ProcessError) -> EvalError {
    EvalError::PrimType {
        prim: prim.to_owned(),
        why: e.to_string(),
    }
}

/// Encode [`ExitStatus`] as `Seq{Binary{32}; 3}` per module rustdoc (never maps signal → code 0).
fn exit_status_as_value(prim: &str, st: ExitStatus) -> Result<Value, EvalError> {
    let success: i32 = if st.success { 1 } else { 0 };
    let (kind, detail): (i32, i32) = match (st.code, st.signal) {
        (Some(code), _) => (0, code),
        (None, Some(sig)) => (1, sig),
        (None, None) => (2, 0),
    };
    let elems = vec![
        i32_as_binary32(prim, success)?,
        i32_as_binary32(prim, kind)?,
        i32_as_binary32(prim, detail)?,
    ];
    Value::new(
        Repr::Seq {
            elem: Box::new(Repr::Binary { width: 32 }),
            len: 3,
        },
        Payload::Seq(elems),
        host_declared_meta()?,
    )
    .map_err(EvalError::Wf)
}

fn unit_bytes() -> Result<Value, EvalError> {
    Value::new(
        Repr::Bytes,
        Payload::Bytes(Vec::new()),
        host_declared_meta()?,
    )
    .map_err(EvalError::Wf)
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

/// `wild:process_spawn : (Bytes [, Seq{{Bytes}}]) → Binary{64}` — spawn via host `ProcessTable`.
///
/// Arity 1: program only (empty argv). Arity 2: program + args as homogeneous `Seq` of `Bytes`.
/// Stdio/env inherit parent (floor default). Empty program → explicit floor error (G2).
fn host_process_spawn(prim: &str, args: &[&Value]) -> Result<Value, EvalError> {
    expect_arity_range(prim, args, 1, 2)?;
    let program = bytes_as_os_string(prim, value_as_bytes(prim, args[0])?)?;
    let argv: Vec<OsString> = if args.len() == 2 {
        let elems = args[1].seq_elems().ok_or_else(|| EvalError::PrimType {
            prim: prim.to_owned(),
            why: "expected Seq of Bytes for argv".to_owned(),
        })?;
        let mut out = Vec::with_capacity(elems.len());
        for (i, el) in elems.iter().enumerate() {
            let b = el.bytes().ok_or_else(|| EvalError::PrimType {
                prim: prim.to_owned(),
                why: format!("argv[{i}] is not Bytes"),
            })?;
            out.push(bytes_as_os_string(prim, b)?);
        }
        out
    } else {
        Vec::new()
    };

    // Floor takes homogeneous AsRef<OsStr> slices — use OsString for both program and args.
    let arg_refs: Vec<&OsString> = argv.iter().collect();
    let id = with_process_table(prim, |table| {
        table
            .spawn(&program, &arg_refs, &SpawnOpts::inherit())
            .map_err(|e| process_err(prim, e))
    })?;
    u64_as_binary64(id)
}

/// `wild:process_wait : (Binary{{64}}) → Seq{{Binary{{32}};3}}` — block until exit; remove handle.
fn host_process_wait(prim: &str, args: &[&Value]) -> Result<Value, EvalError> {
    expect_arity(prim, args, 1)?;
    let id = binary_as_u64(prim, args[0])?;
    let st = with_process_table(prim, |table| {
        table.wait(id).map_err(|e| process_err(prim, e))
    })?;
    exit_status_as_value(prim, st)
}

/// `wild:process_kill : (Binary{{64}}) → Bytes{{}}` — SIGKILL on Unix; does not reap (still wait).
fn host_process_kill(prim: &str, args: &[&Value]) -> Result<Value, EvalError> {
    expect_arity(prim, args, 1)?;
    let id = binary_as_u64(prim, args[0])?;
    with_process_table(prim, |table| {
        table.kill(id).map_err(|e| process_err(prim, e))
    })?;
    unit_bytes()
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

    fn bits_to_i32(v: &Value) -> i32 {
        match v.payload() {
            Payload::Bits(b) => {
                let n = binary::bits_to_int(b);
                i32::try_from(n).expect("status field fits i32")
            }
            _ => panic!("expected Bits"),
        }
    }

    fn bytes_val(b: &[u8]) -> Value {
        Value::new(
            Repr::Bytes,
            Payload::Bytes(b.to_vec()),
            Meta::exact(Provenance::Root),
        )
        .expect("wf")
    }

    fn args_seq(args: &[&[u8]]) -> Value {
        let elems: Vec<Value> = args.iter().map(|a| bytes_val(a)).collect();
        let len = u32::try_from(elems.len()).expect("args len fits u32");
        Value::new(
            Repr::Seq {
                elem: Box::new(Repr::Bytes),
                len,
            },
            Payload::Seq(elems),
            Meta::exact(Provenance::Root),
        )
        .expect("wf")
    }

    fn decode_status(v: &Value) -> (i32, i32, i32) {
        assert_eq!(
            *v.repr(),
            Repr::Seq {
                elem: Box::new(Repr::Binary { width: 32 }),
                len: 3
            }
        );
        let elems = v.seq_elems().expect("seq");
        assert_eq!(elems.len(), 3);
        (
            bits_to_i32(&elems[0]),
            bits_to_i32(&elems[1]),
            bits_to_i32(&elems[2]),
        )
    }

    #[test]
    fn install_registers_catalog_names() {
        let mut reg = PrimRegistry::with_builtins();
        assert!(!reg.has_host("time_mono_nanos"));
        assert!(!reg.has_host("time_wall_nanos"));
        assert!(!reg.has_host("rand_fill"));
        assert!(!reg.has_host("process_spawn"));
        assert!(!reg.has_host("wild:time_mono_nanos"));

        install_default_host_ops(&mut reg);

        assert!(reg.has_host("time_mono_nanos"));
        assert!(reg.has_host("wild:time_mono_nanos"));
        assert!(reg.has_host("time_wall_nanos"));
        assert!(reg.has_host("wild:time_wall_nanos"));
        assert!(reg.has_host("rand_fill"));
        assert!(reg.has_host("wild:rand_fill"));
        assert!(reg.has_host("process_spawn"));
        assert!(reg.has_host("wild:process_spawn"));
        assert!(reg.has_host("process_wait"));
        assert!(reg.has_host("wild:process_wait"));
        assert!(reg.has_host("process_kill"));
        assert!(reg.has_host("wild:process_kill"));

        // Catalog keys appear under wild: in the name list.
        let names = reg.names();
        assert!(names.contains(&"wild:time_mono_nanos"));
        assert!(names.contains(&"wild:time_wall_nanos"));
        assert!(names.contains(&"wild:rand_fill"));
        assert!(names.contains(&"wild:process_spawn"));
        assert!(names.contains(&"wild:process_wait"));
        assert!(names.contains(&"wild:process_kill"));
    }

    #[test]
    fn install_process_host_ops_alone() {
        let mut reg = PrimRegistry::empty();
        install_process_host_ops(&mut reg);
        assert!(reg.has_host("wild:process_spawn"));
        assert!(reg.has_host("wild:process_wait"));
        assert!(reg.has_host("wild:process_kill"));
        assert!(!reg.has_host("wild:time_mono_nanos"));
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

        let spawn = reg.get("wild:process_spawn").unwrap();
        let err = spawn("wild:process_spawn", &[]).unwrap_err();
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

    // --- process host ops (Linux/Unix smoke; share global ProcessTable safely) ----------------

    #[test]
    fn process_spawn_wait_true_succeeds() {
        let mut reg = PrimRegistry::empty();
        install_default_host_ops(&mut reg);
        let spawn = reg.get("wild:process_spawn").expect("registered");
        let wait = reg.get("wild:process_wait").expect("registered");

        let cmd = bytes_val(b"/bin/true");
        let handle = spawn("wild:process_spawn", &[&cmd]).expect("spawn /bin/true");
        assert_eq!(*handle.repr(), Repr::Binary { width: 64 });
        let id = bits_to_u64(&handle);
        assert!(id >= 1, "handle ids start at 1, got {id}");

        let status = wait("wild:process_wait", &[&handle]).expect("wait");
        let (success, kind, detail) = decode_status(&status);
        assert_eq!(success, 1, " /bin/true must succeed");
        assert_eq!(kind, 0, "exited with code");
        assert_eq!(detail, 0, "exit code 0");

        // Second wait on same handle is UnknownHandle — never silent (G2).
        let err = wait("wild:process_wait", &[&handle]).expect_err("second wait");
        match err {
            EvalError::PrimType { prim, why } => {
                assert_eq!(prim, "wild:process_wait");
                assert!(
                    why.contains("unknown process handle") || why.contains("not live"),
                    "must name unknown handle: {why}"
                );
            }
            other => panic!("expected PrimType, got {other:?}"),
        }
    }

    #[test]
    fn process_spawn_with_args_false_nonzero() {
        let mut reg = PrimRegistry::empty();
        install_default_host_ops(&mut reg);
        let spawn = reg.get("wild:process_spawn").expect("registered");
        let wait = reg.get("wild:process_wait").expect("registered");

        // /bin/false ignores args; empty argv Seq exercises the arity-2 path.
        let cmd = bytes_val(b"/bin/false");
        let argv = args_seq(&[]);
        let handle = spawn("wild:process_spawn", &[&cmd, &argv]).expect("spawn /bin/false");
        let status = wait("wild:process_wait", &[&handle]).expect("wait");
        let (success, kind, detail) = decode_status(&status);
        assert_eq!(success, 0);
        assert_eq!(kind, 0);
        assert_eq!(detail, 1);
    }

    #[test]
    fn process_spawn_empty_program_is_explicit() {
        let mut reg = PrimRegistry::empty();
        install_default_host_ops(&mut reg);
        let spawn = reg.get("wild:process_spawn").expect("registered");
        let cmd = bytes_val(b"");
        let err = spawn("wild:process_spawn", &[&cmd]).expect_err("empty program");
        match err {
            EvalError::PrimType { prim, why } => {
                assert_eq!(prim, "wild:process_spawn");
                assert!(
                    why.contains("empty program"),
                    "must name empty program: {why}"
                );
            }
            other => panic!("expected PrimType, got {other:?}"),
        }
    }

    #[test]
    fn process_kill_then_wait_not_success() {
        let mut reg = PrimRegistry::empty();
        install_default_host_ops(&mut reg);
        let spawn = reg.get("wild:process_spawn").expect("registered");
        let wait = reg.get("wild:process_wait").expect("registered");
        let kill = reg.get("wild:process_kill").expect("registered");

        let cmd = bytes_val(b"/bin/sleep");
        let argv = args_seq(&[b"30"]);
        let handle = spawn("wild:process_spawn", &[&cmd, &argv]).expect("spawn sleep");

        let unit = kill("wild:process_kill", &[&handle]).expect("kill");
        assert_eq!(*unit.repr(), Repr::Bytes);
        assert_eq!(unit.bytes().expect("bytes").len(), 0);

        let status = wait("wild:process_wait", &[&handle]).expect("wait after kill");
        let (success, kind, detail) = decode_status(&status);
        assert_eq!(success, 0, "killed process must not report success (G2)");
        // Unix: SIGKILL → kind=1, detail=9. Non-Unix may report code without signal.
        #[cfg(unix)]
        {
            assert_eq!(kind, 1, "signaled, not rewritten as exit code 0");
            assert_eq!(detail, 9, "SIGKILL");
        }
        #[cfg(not(unix))]
        {
            let _ = (kind, detail);
        }
    }

    #[test]
    fn process_unknown_handle_is_explicit() {
        let mut reg = PrimRegistry::empty();
        install_default_host_ops(&mut reg);
        let wait = reg.get("wild:process_wait").expect("registered");
        let kill = reg.get("wild:process_kill").expect("registered");
        // 0 is reserved / never issued; also never live.
        let handle = bin_len(0, 64);
        for (name, f) in [("wild:process_wait", wait), ("wild:process_kill", kill)] {
            let err = f(name, &[&handle]).expect_err("unknown handle");
            match err {
                EvalError::PrimType { prim, why } => {
                    assert_eq!(prim, name);
                    assert!(
                        why.contains("unknown process handle") || why.contains("not live"),
                        "{name}: {why}"
                    );
                }
                other => panic!("{name}: expected PrimType, got {other:?}"),
            }
        }
    }

    #[test]
    fn process_spawn_rejects_non_bytes_cmd() {
        let mut reg = PrimRegistry::empty();
        install_default_host_ops(&mut reg);
        let spawn = reg.get("wild:process_spawn").unwrap();
        let not_bytes = bin_len(1, 8);
        let err = spawn("wild:process_spawn", &[&not_bytes]).unwrap_err();
        match err {
            EvalError::PrimType { why, .. } => {
                assert!(why.contains("Bytes"), "{why}");
            }
            other => panic!("expected PrimType, got {other:?}"),
        }
    }
}
