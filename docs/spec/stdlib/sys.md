# Spec — `std.sys` (audited OS/syscall floor — the `wild`-boundary phylum)

| Field | Value |
|---|---|
| **Status** | **Accepted** (2026-06-21 — maintainer-ratified). Code landed as `mycelium-std-sys` (M-541, Batch P5-B; 18/18 tests, `cargo clippy -D warnings` clean); the DN-16 honesty re-audit (2026-06-21) found the crate honest (all 23 ops `Declared`; fallibility explicit) — its only gap was this missing spec, now written + ratified, completing **25/25** stdlib spec ratification. §7 FLAGs (Q1 the `wild`/FFI floor lift = M-541-future; Q2 math `NaN` vs `Result` = M-525; Q3 `fs::exists` error conflation; Q4 effect-row wiring; Q5 `wall_nanos` String error) are documented follow-ups, not ratification blockers. Was *pending ratification* / *Draft* prior. |
| **Module / Ring** | `std.sys` · Ring 0 (RFC-0016 §4.2 — kernel-adjacent; the `wild`/FFI boundary below Ring 1/2) · Tier A (infrastructure phylum, not a Tier-B common module) |
| **Tracks** | `M-541` — Phase-5 task: "the minimal audited wild/FFI floor inventory (RFC-0016 §8-Q6, LR-9)" |
| **Scope** | The **single, audited phylum boundary** for all OS-syscall / `wild`/FFI contact in the Mycelium standard library tree. Four sub-floors: **`math`** (transcendental `f64` wrappers over Rust libm), **`rand`** (platform entropy via `/dev/urandom`), **`fs`** (thin `std::fs` syscall wrappers), **`time`** (OS wall clock, monotonic clock, thread sleep via `std::time`). Every function returns an explicit `Result` or documented infallible value — never panics, never silently swallows failures (G2). |
| **Boundary** | Out of scope: the higher-level ergonomic modules that *consume* this floor — `std.math` (M-525), `std.rand` (M-531), `std.fs` (M-528), `std.time` (M-529). Those crates use Rust's own `f64`/`std::*` wrappers as placeholders until the call-site wiring lands (deferred, M-541 DONE note); `std-sys` establishes the interface. The **real** `unsafe` FFI (C FFI, `getrandom` syscall, platform-specific assembly) that would lift `#![forbid(unsafe_code)]` is deferred to a future wave — currently the floor is pure safe-Rust `std::*` wrappers (FLAGGED §7-Q1 / FLAG-GETRANDOM). |
| **Depends on** | RFC-0016 §8-Q6 (RESOLVED — the `std-sys` phylum split); RFC-0016 §4.1 (the C1–C6 contract; every sys op meets the never-silent clause); RFC-0016 §9 (the `wild`-free certification badge — the floor being in one phylum is what lets pure `std` earn it); ADR-014 (confines `wild` to an audited block, LR-9); RFC-0013 (diagnostic record shape for errors); RFC-0001 (guarantee lattice). |
| **Grounds on** | Rust `std::f64` (libm transcendentals), Rust `std::fs::File` (entropy/FS), Rust `std::time::{SystemTime, Instant}` (clocks), Rust `std::thread::sleep` (sleep). KC-3: **no new trusted code** above the Rust stdlib — this phylum is the honesty *boundary*, not a new kernel. |

---

## 1. Summary

`std.sys` is the **single, audited `wild`/syscall boundary phylum** for the Mycelium standard library (RFC-0016 §8-Q6, DN-07 RESOLVED, LR-9). Its honesty crux: **every OS-facing call is isolated in this one phylum**, so all pure `std` crates (`std.math`, `std.rand`, `std.fs`, `std.time`) can earn a `wild`-free badge (RFC-0016 §9) as soon as they route through here rather than calling Rust's `std::*` directly. All functions carry the `Declared` guarantee tag — the platform libm precision, OS entropy quality, FS semantics, and clock resolution are **asserted, not audited**; promotion to `Empirical` requires documented trials with measured bounds (VR-5). Every fallible op returns an explicit `Result`; never panics, never silent zero-fill, never silent partial-reads (C1/G2). Currently `#![forbid(unsafe_code)]` holds — the entire floor is safe-Rust `std::*` wrappers; the real `unsafe` FFI floor is deferred (M-541 / FLAGGED §7-Q1). Ring 0 (kernel-adjacent phylum boundary); it adds no trusted bounds above the Rust stdlib (KC-3).

## 2. Scope & module boundary

- **In scope:**
  - **`math` module** (`crates/mycelium-std-sys/src/math.rs`): 14 `[Declared]` transcendental function wrappers (`sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `exp`, `exp2`, `ln`, `log2`, `log10`, `sqrt`, `cbrt`) — thin delegates to Rust `f64::*` (platform libm). Total (no failure path from the Rust wrappers themselves); precision is `Declared`, not audited.
  - **`rand` module** (`crates/mycelium-std-sys/src/rand.rs`): `fill_bytes(buf: &mut [u8]) -> Result<(), EntropyError>` — fills the caller's buffer from `/dev/urandom` via `std::fs::File` + `read_exact`. `EntropyError::Unavailable(String)` on any open/read failure or on platforms without `/dev/urandom`. Never panics; never zero-fills silently (G2).
  - **`fs` module** (`crates/mycelium-std-sys/src/fs.rs`): five thin `std::fs` wrappers — `read`, `write`, `exists`, `create_dir_all`, `remove_file` — each returning `Result<_, io::Error>` on OS failure (except `exists` which follows `std::path::Path::exists` semantics, returning `false` on any error).
  - **`time` module** (`crates/mycelium-std-sys/src/time.rs`): three OS clock/sleep wrappers — `wall_nanos() -> Result<u128, String>` (wall clock nanoseconds since Unix epoch; `Err` if system time precedes epoch), `mono_nanos() -> u64` (process-local monotonic nanoseconds; infallible; saturates at `u64::MAX` on overflow), `sleep_nanos(nanos: u64)` (thread sleep; imprecision is documented, not silent).

- **Out of scope (and who owns it):**
  - **Ergonomic/higher-level surfaces over these floors:** `std.math` (M-525) owns math-function ergonomics; `std.rand` (M-531) owns the seeded/entropy generator abstraction; `std.fs` (M-528) owns affine file-handle semantics; `std.time` (M-529) owns typed duration/instant/clock-distinction values. `std-sys` is consumed by those modules, not a user-facing surface itself.
  - **Cross-platform OS entropy beyond `/dev/urandom`** (Windows, WASI, bare-metal) — `getrandom`-based cross-platform entropy is FLAGGED (FLAG-GETRANDOM); currently `fill_bytes` returns `Err(Unavailable)` on non-Unix platforms.
  - **The Mycelium-lang `wild`/FFI form** — the `#![forbid(unsafe_code)]` boundary is currently intact because the floor uses safe-Rust `std::*`. The real `unsafe` FFI floor (C FFI, platform `syscall`, assembly) that would be needed for `no_std` targets or tighter auditing is deferred to a future wave (FLAGGED §7-Q1).
  - **Call-site wiring** from pure `std` crates through `std-sys` — those crates currently use Rust's own `f64`/`std::*` as placeholders. Wiring is the next step after M-541 establishes the interface floor (M-541 DONE note).

- **Ring & layering:** Ring 0 (kernel-adjacent phylum boundary — RFC-0016 §4.2). `std-sys` is **below** Ring 1 capability surfaces and Ring 2 general library modules; it is the single OS-contact surface all of them will wire through. It is a consumer of Rust `std`, not a producer of new kernel primitives (KC-3: no new trusted code above what Rust's stdlib already provides).

## 3. Exported-op surface (actual Rust surface — not a design sketch)

The surface is **already implemented** in Rust (`crates/mycelium-std-sys/src/`). These are the actual public signatures, not a sketch.

```rust
// crates/mycelium-std-sys/src/math.rs — [Declared] transcendental floor
// All: fn(f64) -> f64 (total; precision Declared); two-arg forms noted.
pub fn sin(x: f64) -> f64      // delegates to f64::sin (platform libm)
pub fn cos(x: f64) -> f64      // delegates to f64::cos
pub fn tan(x: f64) -> f64      // delegates to f64::tan
pub fn asin(x: f64) -> f64     // delegates to f64::asin
pub fn acos(x: f64) -> f64     // delegates to f64::acos
pub fn atan(x: f64) -> f64     // delegates to f64::atan
pub fn atan2(y: f64, x: f64) -> f64  // delegates to f64::atan2
pub fn exp(x: f64) -> f64      // delegates to f64::exp
pub fn exp2(x: f64) -> f64     // delegates to f64::exp2
pub fn ln(x: f64) -> f64       // delegates to f64::ln
pub fn log2(x: f64) -> f64     // delegates to f64::log2
pub fn log10(x: f64) -> f64    // delegates to f64::log10
pub fn sqrt(x: f64) -> f64     // delegates to f64::sqrt
pub fn cbrt(x: f64) -> f64     // delegates to f64::cbrt

// crates/mycelium-std-sys/src/rand.rs — [Declared] entropy floor
pub enum EntropyError { Unavailable(String) }  // explicit error type (G2)
pub fn fill_bytes(buf: &mut [u8]) -> Result<(), EntropyError>
    // reads /dev/urandom via std::fs::File + read_exact;
    // Err(Unavailable) on open/read failure or non-Unix platform;
    // empty buf -> Ok(()) without a read (no platform contact needed)

// crates/mycelium-std-sys/src/fs.rs — [Declared] filesystem floor
pub fn read(path: &Path) -> Result<Vec<u8>, io::Error>
pub fn write(path: &Path, contents: &[u8]) -> Result<(), io::Error>
pub fn exists(path: &Path) -> bool      // false on any OS error (follows std::path::Path::exists)
pub fn create_dir_all(path: &Path) -> Result<(), io::Error>
pub fn remove_file(path: &Path) -> Result<(), io::Error>

// crates/mycelium-std-sys/src/time.rs — [Declared] clock/sleep floor
pub fn wall_nanos() -> Result<u128, String>
    // nanoseconds since Unix epoch; Err(String) if system time < epoch
pub fn mono_nanos() -> u64
    // process-local monotonic nanoseconds (non-decreasing within a run);
    // saturates at u64::MAX on overflow (~584 years); infallible
pub fn sleep_nanos(nanos: u64)
    // sleeps the current thread; actual duration may exceed nanos (documented, not silent)
```

> **Crate attribute (`lib.rs:40`):** `#![forbid(unsafe_code)]` — the entire phylum is safe-Rust. The real unsafe FFI floor is deferred (FLAGGED §7-Q1).

## 4. Guarantee matrix (the load-bearing deliverable — RFC-0016 §4.5)

Rows = all exported ops across the four modules. Tags are from the code's own `[Declared]` annotations (`crates/mycelium-std-sys/src/*.rs`) — not upgraded here (VR-5).

| Op | Guarantee tag | Fallibility (explicit error set) | Declared effects | EXPLAIN-able? |
|---|---|---|---|---|
| `math::sin` | `Declared` | total (no failure path from `f64::sin`) | none | n/a |
| `math::cos` | `Declared` | total | none | n/a |
| `math::tan` | `Declared` | total | none | n/a |
| `math::asin` | `Declared` | total (NaN propagated; domain errors via IEEE754 NaN, not `Result`) | none | n/a |
| `math::acos` | `Declared` | total (NaN propagated) | none | n/a |
| `math::atan` | `Declared` | total | none | n/a |
| `math::atan2` | `Declared` | total | none | n/a |
| `math::exp` | `Declared` | total | none | n/a |
| `math::exp2` | `Declared` | total | none | n/a |
| `math::ln` | `Declared` | total (NaN / ±Inf propagated; ≤0 domain via IEEE754, not `Result`) | none | n/a |
| `math::log2` | `Declared` | total (NaN / ±Inf propagated) | none | n/a |
| `math::log10` | `Declared` | total (NaN / ±Inf propagated) | none | n/a |
| `math::sqrt` | `Declared` | total (NaN propagated for negative inputs) | none | n/a |
| `math::cbrt` | `Declared` | total | none | n/a |
| `rand::fill_bytes` | `Declared` | `Err(EntropyError::Unavailable(String))` — open failure, short read, EOF, or non-Unix platform; never panics, never zero-fills silently | **`entropy`** (an OS read) | yes (the error string names the cause) |
| `fs::read` | `Declared` | `Err(io::Error)` on any OS error | **`io`** | yes (OS error kind + message) |
| `fs::write` | `Declared` | `Err(io::Error)` on any OS error | **`io`** | yes |
| `fs::exists` | `Declared` | total (`false` on OS error — follows `std::path::Path::exists`; FLAG §7-Q3) | **`io`** | n/a |
| `fs::create_dir_all` | `Declared` | `Err(io::Error)` on any OS error | **`io`** | yes |
| `fs::remove_file` | `Declared` | `Err(io::Error)` on any OS error | **`io`** | yes |
| `time::wall_nanos` | `Declared` | `Err(String)` if system time is before the Unix epoch | **`time`** | yes (error string) |
| `time::mono_nanos` | `Declared` | total (saturates at `u64::MAX` on `u128→u64` overflow; infallible) | **`time`** | n/a |
| `time::sleep_nanos` | `Declared` | total (actual sleep duration may exceed requested; documented, not silent) | **`time`** | n/a |

**Tag justification (VR-5 — downgrade rather than overclaim):**

- **All rows tag `Declared`.** This is the honest tag for this phylum. The code's own `[Declared]` annotations (reproduced verbatim from the doc-comments in each module) state the basis explicitly: "no audited theorem backs the libm precision, OS entropy quality, FS semantics, or clock resolution" (`lib.rs:17–24`). Promotion requires:
  - `Empirical`: documented test coverage with measured error bounds (e.g. Diehard/TestU01 for entropy, a measured ULP-error table for transcendentals).
  - `Proven`: a verified theorem whose side-conditions are checked.
  Neither is established in this v0 of the crate. Tags are held at `Declared` (VR-5).

- **`math` rows: `Declared` but total.** Rust's `f64::*` methods are thin libm delegates; they never panic and produce `f64` results for all inputs (including NaN/±Inf via IEEE 754 semantics). The `Declared` tag covers *precision*, not fallibility. Domain errors (e.g. `ln(-1.0)`) produce `NaN` via IEEE 754 propagation — not an explicit `Result`. This is the Rust stdlib's own behaviour; the sys floor inherits it. FLAGGED §7-Q2: whether `std.math` (M-525) should wrap these into `Result`-returning ops with explicit domain errors is a higher-level module decision, not decided here.

- **`rand::fill_bytes`: `Declared`, fallible, effectful.** The entropy source (`/dev/urandom`) is a genuine kernel CSPRNG on Linux/macOS, but without a documented statistical-quality audit it cannot be upgraded to `Empirical`. The fallibility is rigorous: `read_exact` ensures either the full buffer is filled or an `Err` is returned — no silent partial fills (`rand.rs:81`). Never zero-fills silently (`rand.rs:77`).

- **`fs::exists`: total but ambiguous.** Returns `bool` — `false` on any OS error (including permission denied), following `std::path::Path::exists` semantics. This is a honesty note, not a fix: callers who need to distinguish "does not exist" from "permission denied" must use `std::fs::metadata`. FLAGGED §7-Q3.

- **`time::mono_nanos`: total, saturating.** The `u128→u64` narrowing cast saturates at `u64::MAX` (~584 years elapsed) rather than wrapping (`time.rs:45`). This is documented, not silent.

- **`time::sleep_nanos`: declared imprecision.** The module doc (`time.rs:12–14`) explicitly documents that actual sleep duration may exceed `nanos` and directs callers to use `mono_nanos()` before/after if precision matters. This is the declared, never-silent behaviour (G2).

## 5. §4.1 contract conformance (C1–C6)

- **C1 — never-silent (G2).** Every fallible op returns an explicit `Result` or documented `bool` (see `exists` FLAGGED §7-Q3). `fill_bytes` returns `Err(EntropyError::Unavailable(...))` on any failure; never panics, never zero-fills (`rand.rs:70–83`). `read_exact` ensures no silent partial-read. `wall_nanos` returns `Err(String)` if the system time precedes the epoch. `mono_nanos`' `u128→u64` saturating cast is documented. `sleep_nanos`' imprecision is documented. The math functions propagate IEEE 754 NaN — domain errors are not swallowed, though they produce `NaN` rather than `Result::Err` (FLAGGED §7-Q2).
- **C2 — honest per-op tag (VR-5).** All ops carry `[Declared]` in their doc-comments (`lib.rs:17–24`; per-function doc in each module). No op claims `Empirical` or `Proven` without a documented basis. Tags match the code exactly and are not upgraded here.
- **C3 — no black boxes / EXPLAIN (SC-3/G11).** The phylum is the *EXPLAIN surface* for the OS boundary: every op's doc-comment names what it delegates to (`f64::sin`, `/dev/urandom`, `std::fs::write`, `SystemTime::now`), what can fail, and what the failure string contains. `fill_bytes`' error string includes the underlying OS error message (`rand.rs:77,82`). The OS wall-clock error string forwards `SystemTimeError::to_string()` (`time.rs:30`). No opaque failure.
- **C4 — content-addressed, value-semantic (ADR-003).** The math, FS read, and clock ops return plain `f64` / `Vec<u8>` / `u64` / `u128` values — immutable, value-semantic. `fill_bytes` fills a caller-provided buffer (no heap allocation at the sys layer); FS read returns an owned `Vec<u8>`. No ambient mutable state except the `OnceLock<Instant>` origin in `mono_nanos` (process-local; not a shared mutable value in the value-model sense).
- **C5 — above the small kernel (KC-3).** `std-sys` adds **no** new trusted code: every function is a thin wrapper over already-trusted Rust stdlib primitives. The `#![forbid(unsafe_code)]` attribute (`lib.rs:40`) confirms this — no `unsafe` block exists in the crate. The `wild`/FFI floor that M-541 inventories is the Rust stdlib's *own* OS contact, not new code written here.
- **C6 — declared, bounded effects (RFC-0014).** Effects are named in the guarantee matrix (entropy / io / time). No op has undeclared effects. `sleep_nanos`' thread-blocking is documented. The entropy effect (`fill_bytes`) is the honest label for an OS read that introduces real nondeterminism. FLAGGED §7-Q4: the formal RFC-0014 effect-row notation is not yet wired to `std-sys` ops (the RFC-0014 surface is the consuming `std.*` modules' concern, not the raw floor).

## 6. Grounding

- **The `std-sys` phylum split rationale + LR-9:** RFC-0016 §8-Q6 (RESOLVED — "split the audited `wild` floor into a separate `std-sys` phylum so pure `std` stays leak-free *by construction* (LR-9) and can publish a `wild`-free certification badge (§9)") and DN-07 (the ratification record, 2026-06-17). The three call sites are fs syscalls (M-528), rand platform entropy (M-531), and math libm transcendentals (M-525) — the `issues.yaml` M-541 body confirms this inventory.
- **The `wild`-free badge criterion:** RFC-0016 §9 ("a `phylum` with no `wild` blocks is leak-free *by construction* (LR-9); the stdlib can publish which modules clear it").
- **The audited `wild` boundary rule:** ADR-014 (confines `wild`/FFI to an audited block; LR-9/S6); the mandate that OS contact appears in exactly one place in the std tree (`lib.rs:27–31` — "placing syscall contact here means the `wild` audit surface is bounded, inspectable, and EXPLAIN-able (G2/G11/SC-3)").
- **The `Declared` guarantee tag and VR-5:** RFC-0001 §4.3 (the `Exact ⊐ Proven ⊐ Empirical ⊐ Declared` lattice); VR-5 (downgrade to stay honest, never upgrade without a checked basis); `lib.rs:16–24` (the crate-level honesty declaration).
- **Never-silent fallibility:** G2 (never silent); RFC-0013 (the diagnostic record shape — error strings in `fill_bytes` and `wall_nanos` carry cause text); I1 (failures propagate, never swallowed).
- **KC-3 / no new trusted code:** KC-3 (small auditable kernel); `#![forbid(unsafe_code)]` (`lib.rs:40`).
- **The Rust `core`/`std` isolation precedent grounding the phylum split:** `research/08` T8.1 (the stdlib prior-art record that grounds §8-Q6 resolution).

## 7. Open questions (FLAGGED — resolve before ratification)

- **(Q1) The real `unsafe` FFI floor is deferred (M-541 / FLAG-GETRANDOM).** Currently `#![forbid(unsafe_code)]` is intact because the entire floor uses safe-Rust `std::*` wrappers. The real `wild` audit — C FFI, `getrandom` syscall, `no_std` platform targets, assembly — is deferred to a future wave. When that floor lands it will **lift** `#![forbid(unsafe_code)]` and require a genuine LR-9 `wild`-block inventory. — *Disposition: FLAGGED; FLAG-GETRANDOM (cross-platform OS entropy via `getrandom`) noted in `rand.rs:22–23`; deferred. Ties RFC-0016 §8-Q6 / ADR-014. The call-site wiring from pure `std` crates through `std-sys` is also deferred (M-541 DONE note: "wiring is deferred to a future wave; this module establishes the interface").*

- **(Q2) Math domain errors: `NaN` propagation vs explicit `Result`.** The `math` module inherits Rust's IEEE 754 semantics: `ln(-1.0)` → `NaN`, `asin(2.0)` → `NaN`, not `Err(DomainError)`. Whether `std.math` (M-525) should wrap these ops with `Result`-returning domain-error variants is a higher-level module decision — `std-sys` is the raw floor and cannot decide this unilaterally. The NaN propagation is not silent (it is IEEE 754 standard behaviour), but it is also not an explicit `Result::Err`. — *Disposition: FLAGGED; decision owned by M-525 / `std.math`; `std-sys` provides the total-or-NaN floor regardless of M-525's choice.*

- **(Q3) `fs::exists` error conflation.** `exists(path)` returns `false` on any OS error including permission denied, following `std::path::Path::exists`. This conflates "does not exist" with "cannot determine". The doc-comment (`fs.rs:34–36`) flags this and directs callers to `std::fs::metadata` when the distinction matters. Whether `std-sys` should expose a separate `try_exists` returning `Result<bool, io::Error>` (as Rust 1.63 added) is open. — *Disposition: FLAGGED; `fs::exists` currently matches `std::path::Path::exists` behaviour verbatim. A `try_exists` variant can be added without breaking the current op (additive).*

- **(Q4) RFC-0014 effect-row wiring to `std-sys` ops.** The guarantee matrix names effects (entropy / io / time) but the formal RFC-0014 effect-row notation is not yet wired to this phylum's signatures. That wiring is the concern of the *consuming* `std.*` module layers (M-528/M-529/M-531/M-525), not of the raw floor. — *Disposition: FLAGGED; ties RFC-0014 and the per-module specs for the consuming std layers. `std-sys` ops are by definition effectful; the declaration happens at the consuming layer.*

- **(Q5) `wall_nanos` error type (`String` vs a typed error).** `wall_nanos()` returns `Result<u128, String>` — the `String` holds the `SystemTimeError::to_string()` message. A typed `TimeError` (analogous to `EntropyError` in `rand`) would be more honest and EXPLAIN-able. — *Disposition: FLAGGED; a future wave can promote this to a typed error without breaking the never-silent contract (the `Err` already names the cause). Low priority: the consuming `std.time` layer will wrap this into its own typed `TimeErr` anyway.*

## Meta — changelog

- **2026-06-21 — Implemented (Rust-first), pending ratification.** Stands up the `std.sys` (`mycelium-std-sys`, M-541) per-crate spec under RFC-0016 (Accepted). `std-sys` is the **single, audited OS/syscall boundary phylum** ratified by RFC-0016 §8-Q6 (RESOLVED, DN-07 2026-06-17): the `wild` floor splits into this separate phylum so all pure `std` crates can earn a `wild`-free badge (RFC-0016 §9). Code landed 2026-06-19 (M-541 DONE): four modules — `math` (14 `[Declared]` transcendental wrappers over Rust `f64`/libm), `rand` (`fill_bytes` via `/dev/urandom`, `EntropyError`, `[Declared]`), `fs` (`read`/`write`/`exists`/`create_dir_all`/`remove_file`, `[Declared]`), `time` (`wall_nanos`/`mono_nanos`/`sleep_nanos`, `[Declared]`) — 18/18 tests pass, `cargo clippy -D warnings` clean. This spec documents the real Rust surface (§3 is the actual API, not a sketch) and the **guarantee matrix** (§4): all 23 exported ops carry `Declared` tags matched exactly to the code's own `[Declared]` doc-comment annotations — never upgraded (VR-5). Honesty crux: `#![forbid(unsafe_code)]` is currently intact because the floor uses safe-Rust `std::*` wrappers; the real `unsafe` FFI floor is deferred (FLAGGED §7-Q1, FLAG-GETRANDOM). Every fallible op is never-silent: `fill_bytes` returns `Err(EntropyError::Unavailable(...))`, never panics, never zero-fills; `wall_nanos` returns `Err(String)` if system time precedes epoch; FS ops propagate `io::Error`; math ops propagate IEEE 754 NaN (FLAGGED §7-Q2). §4.1 conformance (C1–C6) stated concretely. Five questions FLAGGED (the real `unsafe`/`getrandom` floor; math domain `NaN` vs `Result`; `fs::exists` error conflation; RFC-0014 effect-row wiring; `wall_nanos` typed error). Grounding: RFC-0016 §8-Q6/§9, DN-07, ADR-014, LR-9, RFC-0001 (lattice/VR-5/G2/KC-3), RFC-0013, research/08 T8.1. Status set to *Implemented (Rust-first), pending ratification* — not `Accepted` (the maintainer ratification is the orchestrator's append-only decision, recorded separately). Append-only.
