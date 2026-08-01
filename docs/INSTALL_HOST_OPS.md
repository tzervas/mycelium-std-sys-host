# install_default_host_ops — contract (WP-4 / WP-5 / S-HOST-REGISTRY)

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

`install_process_host_ops(reg)` installs only the process trio (also called from the default install).

## v0 ops (blocking-hypha)

| wild name | Floor | Arity / result | Encoding notes |
|-----------|-------|----------------|----------------|
| `time_mono_nanos` | `std_sys::time::mono_nanos` | `() → Binary{64}` | unsigned MSB-first; total |
| `time_wall_nanos` | `std_sys::time::wall_nanos` | `() → Binary{64}` | unsigned MSB-first of `u64`; pre-epoch / `u128→u64` overflow → `EvalError::PrimType` (G2) |
| `rand_fill` | `std_sys::rand::fill_bytes` | `(Binary{W}) → Bytes` | length = unsigned Binary magnitude (checked); OS fail → `PrimType`, never silent zero-fill |
| `process_spawn` | `ProcessTable::spawn` | `(Bytes)` or `(Bytes, Seq{Bytes}) → Binary{64}` | handle id ≥ 1; empty program / OS spawn → `PrimType` |
| `process_wait` | `ProcessTable::wait` | `(Binary{64}) → Seq{Binary{32};3}` | `[success, kind, detail]`; wait **removes** handle; unknown id → `PrimType` |
| `process_kill` | `ProcessTable::kill` | `(Binary{64}) → Bytes{}` | unit empty Bytes; does not reap; unknown id → `PrimType` |
| `fs_*` | RealFs wiring | later | after floor complete |

### process_wait status triple

| index | field | meaning |
|-------|-------|---------|
| 0 | `success` | `1` / `0` |
| 1 | `kind` | `0` = exit code; `1` = signal; `2` = neither |
| 2 | `detail` | code or signal as two's-complement `i32` bits |

Signal death is never rewritten as exit code `0` (G2). See `src/host_registry.rs` rustdoc and
`mycelium-std-sys` `docs/PROCESS-HOST-HOOKS.md`.

Live children sit in a process-level `Mutex<ProcessTable>` (no `HostCtx` yet; `PrimFn` is a pure
function pointer).

## Deps

- `mycelium-interp` — pin to mycelium-runtime main tip post-#11 (`register_host` / `install_host_ops`).
- `mycelium-core` — Value / Binary / Bytes / Seq encoding (same rev as interp's core pin).
- `mycelium-std-sys` — pin to main tip with `process` floor (`spawn`/`wait`/`kill`/`ProcessTable`).

Feature-gated so pure OsEntropy/OsClock users stay light.

## Who calls

`myc` CLI run path after `PrimRegistry::with_builtins()`, before eval (L-CLI).

## Guarantee

Ambient OS results are `Declared` + zero-ε `UserDeclared` bound (VR-5). See `src/host_registry.rs` rustdoc.
