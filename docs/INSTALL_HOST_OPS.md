# install_default_host_ops — contract (WP-4 / S-HOST-REGISTRY)

**Spike S1 (2026-08-01):** this crate owns the *default* host op table.

## Signature (`feature = "host-registry"`)

```rust
use mycelium_interp::PrimRegistry;
use mycelium_std_sys_host::install_default_host_ops;

// after PrimRegistry::with_builtins():
let mut reg = PrimRegistry::with_builtins();
install_default_host_ops(&mut reg);
// Interpreter::new(reg, swap) — CLI bind (L-CLI)
```

## v0 ops (blocking-hypha)

| wild name | Floor | Arity / result | Encoding notes |
|-----------|-------|----------------|----------------|
| `time_mono_nanos` | `std_sys::time::mono_nanos` | `() → Binary{64}` | unsigned MSB-first; total |
| `time_wall_nanos` | `std_sys::time::wall_nanos` | `() → Binary{64}` | unsigned MSB-first of `u64`; pre-epoch / `u128→u64` overflow → `EvalError::PrimType` (G2) |
| `rand_fill` | `std_sys::rand::fill_bytes` | `(Binary{W}) → Bytes` | length = unsigned Binary magnitude (checked); OS fail → `PrimType`, never silent zero-fill |
| `fs_*` | RealFs wiring | later | after floor complete |

## Deps

- `mycelium-interp` — pin to post-#11 rev (`register_host` / `install_host_ops`). Current train pin: runtime branch `train/gap-closure-host-call-registry` head until merge.
- `mycelium-core` — Value / Binary / Bytes encoding (same rev as interp's core pin).

Feature-gated so pure OsEntropy/OsClock users stay light.

## Who calls

`myc` CLI run path after `PrimRegistry::with_builtins()`, before eval (L-CLI).

## Guarantee

Ambient OS results are `Declared` + zero-ε `UserDeclared` bound (VR-5). See `src/host_registry.rs` rustdoc.
