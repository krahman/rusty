# rusty multiplayer starter

Authoritative multiplayer game skeleton in Rust.

## Workspace layout

- `crates/protocol`: shared wire protocol types
- `crates/shared`: deterministic game simulation logic
- `crates/server`: authoritative tick-based server
- `crates/client`: terminal client for sending movement input

## Run

Start server:

```bash
cargo run -p server
```

Start one or more clients in separate terminals:

```bash
PLAYER_NAME=alice cargo run -p client
PLAYER_NAME=bob cargo run -p client
```

In each client, type controls and press enter: `w`, `a`, `s`, `d`, `wa`, etc.

## Notes

- Server uses a fixed `20hz` simulation tick.
- Client input includes sequence numbers.
- Server snapshots include `last_processed_input` for reconciliation plumbing.
