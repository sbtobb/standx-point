# Issues - StandX Auth CLI Optimization

## Session: 2026-02-03T14:24:00Z


- Plan mentioned using `jsonwebtoken`, but it is not present in Cargo.toml/Cargo.lock; JWT parsing was implemented via base64url decode instead.
- Minor: `tests/common::generate_test_keypair()` is now unused, causing a dead_code warning in `cargo test`.
