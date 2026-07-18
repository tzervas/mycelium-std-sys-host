# CROSS-REF — mycelium-std-sys-host

Mycelium-internal dependencies only (steer handoff §6.1; external crates stay in Cargo
metadata). Pinned revs are the fixed (buildable) tips recorded by the Phase-B wave;
content hash = git tree hash of the pinned rev.

| Interface consumed | Repo | Pinned rev | Content hash | Notes |
|---|---|---|---|---|
| mycelium-std-rand | https://github.com/tzervas/mycelium-std-rand | `088256c5f29031e487cb7a335cbc9ff29794b58d` | tree `(tree hash: fetch dep rev locally to resolve)` | Rust API of `mycelium-std-rand` (see monorepo `docs/api-index/INDEX.md#mycelium-std-rand`) |
| mycelium-std-sys | https://github.com/tzervas/mycelium-std-sys | `95957a5a91e42f003709d584e47783777e4a4618` | tree `853d1d2609cd4ddfee92b637d6aef5e045f3eb84` | Rust API of `mycelium-std-sys` (see monorepo `docs/api-index/INDEX.md#mycelium-std-sys`) |
| mycelium-std-time | https://github.com/tzervas/mycelium-std-time | `47ef9e7ec4143c97878083ca5c15930a21eeed83` | tree `(tree hash: fetch dep rev locally to resolve)` | Rust API of `mycelium-std-time` (see monorepo `docs/api-index/INDEX.md#mycelium-std-time`) |

**Owning docs:** `docs/spec/stdlib/sys.md` (slice in this repo) · RFC-0016.
**Source provenance:** extracted from `tzervas/mycelium` archive `aad96b7a…`; fixed by
the course-correction Phase B (workspace root, git pins, toolchain + supply-chain
replicas, CI v2). Full program record: monorepo
`docs/planning/course-correction-2026-07-18/PROGRAM.md`.
