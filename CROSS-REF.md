# CROSS-REF — mycelium-std-sys-host

Mycelium-internal dependencies only (steer handoff §6.1; external crates stay in Cargo
metadata). Pinned revs are the fixed (buildable) tips recorded by the Phase-B wave;
content hash = git tree hash of the pinned rev.

| Interface consumed | Repo | Pinned rev | Content hash | Notes |
|---|---|---|---|---|
| mycelium-std-rand | https://github.com/tzervas/mycelium-std-rand | `088256c5f29031e487cb7a335cbc9ff29794b58d` | tree `(tree hash: fetch dep rev locally to resolve)` | Rust API of `mycelium-std-rand` (see monorepo `docs/api-index/INDEX.md#mycelium-std-rand`) |
| mycelium-std-sys | https://github.com/tzervas/mycelium-std-sys | `0de74f60cdd26c25db6704c64f17f08fca0c52af` | tree `55460538cea33ba99d4f7ccdfdc5157a2e4ff532` | process floor (WP-5 / #10) + time/rand/fs/io |
| mycelium-std-time | https://github.com/tzervas/mycelium-std-time | `47ef9e7ec4143c97878083ca5c15930a21eeed83` | tree `(tree hash: fetch dep rev locally to resolve)` | Rust API of `mycelium-std-time` (see monorepo `docs/api-index/INDEX.md#mycelium-std-time`) |
| mycelium-interp *(feature `host-registry`)* | https://github.com/tzervas/mycelium-runtime | `4af6d0051f4da4961a108869c56407e32b9a372b` | main tip post-#11 (HostCallRegistry / `register_host`) | `PrimRegistry::register_host` / `install_host_ops` |
| mycelium-core *(feature `host-registry`)* | https://github.com/tzervas/mycelium-core | `46d2515cbd86d2ae4d1365f4adcd2796737e9f0b` | same core rev as interp on that tip | Value / Binary / Bytes / Seq encoding for wild: host results |

**Owning docs:** `docs/spec/stdlib/sys.md` (slice in this repo) · `docs/INSTALL_HOST_OPS.md` · RFC-0016 · S-HOST-REGISTRY · S-STD-PROCESS.
**Source provenance:** extracted from `tzervas/mycelium` archive `aad96b7a…`; fixed by
the course-correction Phase B (workspace root, git pins, toolchain + supply-chain
replicas, CI v2). Full program record: monorepo
`docs/planning/course-correction-2026-07-18/PROGRAM.md`.
