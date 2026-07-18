# CROSS-REF — mycelium-std-sys-host

Mycelium-internal dependencies only (steer handoff §6.1; external crates stay in Cargo
metadata). Pinned revs are the fixed (buildable) tips recorded by the Phase-B wave;
content hash = git tree hash of the pinned rev.

| Interface consumed | Repo | Pinned rev | Content hash | Notes |
|---|---|---|---|---|
| mycelium-std-rand | https://github.com/tzervas/mycelium-std-rand | `093d40ca7a90224de2c2aab0d0fb0fd3717809b5` | tree `(tree hash: fetch dep rev locally to resolve)` | Rust API of `mycelium-std-rand` (see monorepo `docs/api-index/INDEX.md#mycelium-std-rand`) |
| mycelium-std-sys | https://github.com/tzervas/mycelium-std-sys | `95957a5a91e42f003709d584e47783777e4a4618` | tree `(tree hash: fetch dep rev locally to resolve)` | Rust API of `mycelium-std-sys` (see monorepo `docs/api-index/INDEX.md#mycelium-std-sys`) |
| mycelium-std-time | https://github.com/tzervas/mycelium-std-time | `fedd18411bf33cc2cd2b0d7ba41e1259719d23c0` | tree `(tree hash: fetch dep rev locally to resolve)` | Rust API of `mycelium-std-time` (see monorepo `docs/api-index/INDEX.md#mycelium-std-time`) |

**Owning docs:** `docs/spec/stdlib/sys.md` (slice in this repo) · RFC-0016.
**Source provenance:** extracted from `tzervas/mycelium` archive `aad96b7a…`; fixed by
the course-correction Phase B (workspace root, git pins, toolchain + supply-chain
replicas, CI v2). Full program record: monorepo
`docs/planning/course-correction-2026-07-18/PROGRAM.md`.
