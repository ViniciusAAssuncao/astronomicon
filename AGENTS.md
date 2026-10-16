# Astronomicon and Rocketcon work rules

- Keep Rust authoritative for physics, simulation state, and persistence. The C++/QtWidgets application in the sibling `rocketcon-desktop` repository hosts the UI through an explicit boundary and does not access domain internals or the database.
- Prefer SOLID and DRY. Keep each source file under 500 lines. Split files by responsibility when they become large; avoid mechanical splits that hide one responsibility across files.
- Comments in source code are strictly prohibited and block acceptance, including line, block, and documentation comments. Remove existing comments from source files whenever editing them. Record rationale, physics, numerical limitations, and design decisions in Markdown documentation rather than code.
- Develop in small, reviewable checkpoints. Measure the complete flight tick before redesigning its persistence or runtime.
- Preserve scientific units, invariants, and existing behavior when changing simulation code. Add meaningful tests around changed behavior.
- Do not commit, push, change remotes, or open pull requests unless the user explicitly asks. Leave changes in the working tree for review.
- Coordinate contract changes with the sibling `rocketcon-desktop` repository. Do not move Rust domain logic into that repository.
- Support the desktop as a two-dimensional aerospace simulator/game with scientific telemetry and a Windows 2.x inspired interface. Keep all physical truth, validation, events, trajectory generation, and save semantics in Rust; visual style remains in Qt.
- Treat performance as a standing product constraint. Measure complete tick latency and FFI transfer cost separately, with reproducible scenarios, before claiming a real-time rate or redesigning persistence.
- Read `docs/rocketcon-desktop-contract.md` when extending the CXX boundary for the desktop. Add small DTOs and commands only for an identified vertical slice.
