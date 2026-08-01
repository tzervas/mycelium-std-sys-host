# install_default_host_ops — contract (WP-4)

**Spike S1 (2026-08-01):** this crate owns the *default* host op table.

## Signature (lands when `mycelium-runtime` #11 merges)

```rust
// feature = "host-registry"
use mycelium_interp::{install_host_ops, PrimRegistry};

pub fn install_default_host_ops(reg: &mut PrimRegistry) {
    // Map audited std-sys floor → wild: names
    // install_host_ops(reg, &[
    //   ("time_mono_nanos", host_time_mono),
    //   ("rand_fill", host_rand_fill),
    //   // fs / process follow
    // ]);
}
```

## v0 first ops (blocking-hypha)

| wild name | Floor | Notes |
|-----------|-------|-------|
| `time_mono_nanos` | `std_sys::time::mono_nanos` | total |
| `time_wall_nanos` | `std_sys::time::wall_nanos` | never-silent err |
| `rand_fill` | `std_sys::rand::fill_bytes` | never-silent err |
| `fs_*` | RealFs wiring | after M-541 floor complete |

## Who calls

`myc` CLI run path after `PrimRegistry::with_builtins()`, before eval.

## Deps

`mycelium-interp` with `register_host` — pin to post-#11 rev. Feature-gated so pure OsEntropy/OsClock users stay light.
