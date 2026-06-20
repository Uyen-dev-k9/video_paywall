# video_paywall

## Project Title
video_paywall

## Project Description
video_paywall is a per-video paywall dApp built on Stellar using the Soroban
smart contract platform. Independent video creators list individual videos
with a price and a time-bound viewing duration; viewers pay to unlock
time-limited access; and creators retain the right to refund viewers while
their access window is still open. The contract is intentionally
self-contained — it records unlocks, expirations, and revenue counters
on-chain without relying on an external token transfer pipeline, so the
business rules (publish, pay, refund, expire, audit) are auditable and
enforceable from a single contract call.

## Project Vision
The long-term vision is to make short-form and premium video monetization
accessible to any creator with a Stellar wallet, removing the gatekeepers
that sit between a creator and their audience. By encoding the paywall
rules in a Soroban contract, we want a future where viewers can pay a
creator directly, refunds are transparent and on-chain, and revenue
counters are publicly verifiable — turning "pay per video" into a primitive
that any frontend, mobile app, or protocol can compose with.

## Key Features
- `publish_video` — a creator registers a new video with a price and an
  access duration; the creator's address is permanently bound to the
  listing as the refund authority.
- `unlock` — a viewer authorizes payment of the configured price and
  receives a time-stamped unlock that expires at
  `ledger_time + duration_seconds`. Revenue is recorded on-chain.
- `refund` — only the original creator can refund a viewer while the
  access window is still open, and a reason is stored in an on-chain
  audit log for transparency.
- `is_unlocked` — a read-only helper that returns whether a viewer
  currently has a valid (non-refunded, non-expired) unlock.
- `unlock_expires_at` — exposes the exact expiry timestamp of a viewer's
  unlock so frontends can show countdowns or schedule re-unlocks.
- `get_revenue` — returns the cumulative revenue for a video, net of
  refunds, so creators and analytics tools can track earnings.
- Authorization is enforced with `require_auth()` on every state-changing
  call, and storage is keyed by `Symbol` so each video has a clean,
  isolated namespace.

## Contract

- **Network:** Stellar Testnet (Public)
- **Scope:** content dApp — see `contracts/video_paywall/src/lib.rs` for the full video_paywall business logic.
- **Functions exposed:** see `Key Features` above and the `pub fn` list in `lib.rs`.
- **Contract ID:** `CDC4BH6RNJKEXAEHTQRGGM2ZSOGDHXI5BF4GQN54KC7XAP42PG6LFPYX`
- **Explorer template:** `https://stellar.expert/explorer/testnet/tx/188dc3c64bd1b825db26cabddfee857d79e93ca2da5cad5ebf43e5a261f5d521`


## Future Scope
- **Real token settlement:** wire the `unlock` and `refund` paths to a
  Stellar asset contract so payments move real on-chain value instead of
  only updating counters.
- **Tiered access:** support creator-defined tiers (720p, 1080p, 4K) with
  separate prices and durations under the same video.
- **Bundle and subscription unlocks:** let a single payment unlock a
  creator's whole catalog or a time-bounded subscription across many
  videos.
- **Revenue split:** accept multiple creator addresses per video and
  distribute the unlock price according to share weights.
- **Time-weighted refund windows:** let the creator configure a
  cooldown (e.g. refund only within 24h of unlock) and enforce it on
  chain.
- **Event / indexer integration:** emit Soroban events on `publish`,
  `unlock`, and `refund` so off-chain indexers can build creator
  dashboards and viewer history feeds in real time.
- **Frontend dApp:** a minimal HTML/JS UI using Freighter for wallet
  connect, plus a small viewer page that checks `is_unlocked` before
  playing back a hosted video URL.

## Profile

- **Name:** <!-- Fill github name -->
- **Project:** `video_paywall` (content)
- **Built with:** Soroban SDK 25, Rust, Stellar Testnet
