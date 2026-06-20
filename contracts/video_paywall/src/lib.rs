#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Map, Symbol};

/// On-chain representation of a published video entry.
#[contracttype]
#[derive(Clone)]
pub struct Video {
    /// Address of the creator who owns this video listing.
    pub creator: Address,
    /// Price a viewer must pay to unlock the video, in the smallest token unit.
    pub price: u32,
    /// Number of seconds of access granted by a single unlock.
    pub duration_seconds: u64,
    /// Ledger timestamp at which the video was first published.
    pub published_at: u64,
    /// Cumulative revenue accumulated for this video, net of refunds.
    pub total_revenue: u32,
}

/// On-chain record of a single viewer's access to a single video.
#[contracttype]
#[derive(Clone)]
pub struct UnlockRecord {
    /// Unix timestamp at which this unlock expires.
    pub expires_at: u64,
    /// Whether the creator has refunded this unlock.
    pub refunded: bool,
}

/// VideoPaywall is a per-video paywall smart contract. A creator publishes a
/// video with a price and an access duration; viewers pay the price to receive
/// a time-bound unlock token; creators can refund viewers during the active
/// access window.
#[contract]
pub struct VideoPaywall;

#[contractimpl]
impl VideoPaywall {
    /// Publishes a new video on the paywall under `video_id`. The `creator`
    /// sets the unlock `price` and the `duration_seconds` that an unlock
    /// grants access to the content. Requires authorization from `creator`.
    /// Panics if the video is already published or if `duration_seconds`
    /// is zero.
    pub fn publish_video(
        env: Env,
        creator: Address,
        video_id: Symbol,
        price: u32,
        duration_seconds: u64,
    ) {
        creator.require_auth();

        if duration_seconds == 0 {
            panic!("duration_seconds must be greater than zero");
        }

        let videos_key = Symbol::new(&env, "videos");
        let mut videos: Map<Symbol, Video> = env
            .storage()
            .instance()
            .get(&videos_key)
            .unwrap_or_else(|| Map::new(&env));

        if videos.contains_key(video_id.clone()) {
            panic!("video already published");
        }

        let video = Video {
            creator: creator.clone(),
            price,
            duration_seconds,
            published_at: env.ledger().timestamp(),
            total_revenue: 0,
        };

        videos.set(video_id.clone(), video);
        env.storage().instance().set(&videos_key, &videos);

        // Initialize the per-video unlock map so subsequent unlocks can be
        // stored without first touching this slot.
        let unlocks_key = Symbol::new(&env, "unlocks");
        let mut unlocks: Map<Symbol, Map<Address, UnlockRecord>> = env
            .storage()
            .instance()
            .get(&unlocks_key)
            .unwrap_or_else(|| Map::new(&env));

        let mut video_unlocks: Map<Address, UnlockRecord> = Map::new(&env);
        video_unlocks.set(creator, UnlockRecord {
            expires_at: env.ledger().timestamp() + duration_seconds,
            refunded: false,
        });
        unlocks.set(video_id, video_unlocks);
        env.storage().instance().set(&unlocks_key, &unlocks);
    }

    /// Viewer pays the video's price to unlock access for the configured
    /// duration. The viewer's authorization is required. The unlock expiry
    /// timestamp is returned. Panics if the video is unknown or if the
    /// viewer already holds a non-refunded, non-expired unlock for the
    /// same video. Note: this contract records the unlock and increments
    /// revenue counters; it does not perform any real token transfer.
    pub fn unlock(env: Env, viewer: Address, video_id: Symbol) -> u64 {
        viewer.require_auth();

        let videos_key = Symbol::new(&env, "videos");
        let mut videos: Map<Symbol, Video> = env
            .storage()
            .instance()
            .get(&videos_key)
            .unwrap_or_else(|| Map::new(&env));

        let mut video = match videos.get(video_id.clone()) {
            Some(v) => v,
            None => panic!("video not found"),
        };

        let unlocks_key = Symbol::new(&env, "unlocks");
        let mut unlocks: Map<Symbol, Map<Address, UnlockRecord>> = env
            .storage()
            .instance()
            .get(&unlocks_key)
            .unwrap_or_else(|| Map::new(&env));

        let mut video_unlocks: Map<Address, UnlockRecord> = match unlocks.get(video_id.clone()) {
            Some(m) => m,
            None => panic!("unlocks map missing for video"),
        };

        if let Some(existing) = video_unlocks.get(viewer.clone()) {
            if !existing.refunded && existing.expires_at > env.ledger().timestamp() {
                panic!("viewer already has an active unlock");
            }
        }

        let expires_at = env
            .ledger()
            .timestamp()
            .saturating_add(video.duration_seconds);

        video_unlocks.set(
            viewer.clone(),
            UnlockRecord {
                expires_at,
                refunded: false,
            },
        );
        unlocks.set(video_id.clone(), video_unlocks);
        env.storage().instance().set(&unlocks_key, &unlocks);

        // Record revenue. This contract does not move any real asset; it
        // only updates an on-chain counter.
        video.total_revenue = video.total_revenue.saturating_add(video.price);
        videos.set(video_id, video);
        env.storage().instance().set(&videos_key, &videos);

        expires_at
    }

    /// Creator refunds a viewer's unlock for a given video and records the
    /// provided `reason`. Only the original creator of the video may issue
    /// a refund, and only while the access window is still active and the
    /// unlock has not already been refunded. Revenue for the video is
    /// decremented by the unlock price.
    pub fn refund(
        env: Env,
        creator: Address,
        viewer: Address,
        video_id: Symbol,
        reason: Symbol,
    ) {
        creator.require_auth();

        let videos_key = Symbol::new(&env, "videos");
        let mut videos: Map<Symbol, Video> = env
            .storage()
            .instance()
            .get(&videos_key)
            .unwrap_or_else(|| Map::new(&env));

        let mut video = match videos.get(video_id.clone()) {
            Some(v) => v,
            None => panic!("video not found"),
        };

        if video.creator != creator {
            panic!("only the video creator can refund");
        }

        let unlocks_key = Symbol::new(&env, "unlocks");
        let mut unlocks: Map<Symbol, Map<Address, UnlockRecord>> = env
            .storage()
            .instance()
            .get(&unlocks_key)
            .unwrap_or_else(|| Map::new(&env));

        let mut video_unlocks: Map<Address, UnlockRecord> = match unlocks.get(video_id.clone()) {
            Some(m) => m,
            None => panic!("unlocks map missing for video"),
        };

        let mut record = match video_unlocks.get(viewer.clone()) {
            Some(r) => r,
            None => panic!("no unlock found for viewer"),
        };

        if record.refunded {
            panic!("unlock already refunded");
        }

        if record.expires_at <= env.ledger().timestamp() {
            panic!("unlock window has expired, refund no longer possible");
        }

        record.refunded = true;
        video_unlocks.set(viewer.clone(), record);
        unlocks.set(video_id.clone(), video_unlocks);
        env.storage().instance().set(&unlocks_key, &unlocks);

        // Decrement revenue (saturating so we never underflow).
        video.total_revenue = video.total_revenue.saturating_sub(video.price);
        videos.set(video_id.clone(), video);
        env.storage().instance().set(&videos_key, &videos);

        // Persist the refund reason in a small audit log.
        let refunds_key = Symbol::new(&env, "refunds");
        let mut refunds: Map<Symbol, Map<Address, Symbol>> = env
            .storage()
            .instance()
            .get(&refunds_key)
            .unwrap_or_else(|| Map::new(&env));

        let mut video_refunds: Map<Address, Symbol> = match refunds.get(video_id.clone()) {
            Some(m) => m,
            None => Map::new(&env),
        };
        video_refunds.set(viewer, reason);
        refunds.set(video_id, video_refunds);
        env.storage().instance().set(&refunds_key, &refunds);
    }

    /// Returns true if the viewer currently holds an active (non-refunded
    /// and non-expired) unlock for the given video. Returns false otherwise.
    pub fn is_unlocked(env: Env, viewer: Address, video_id: Symbol) -> bool {
        let unlocks_key = Symbol::new(&env, "unlocks");
        let unlocks: Map<Symbol, Map<Address, UnlockRecord>> = env
            .storage()
            .instance()
            .get(&unlocks_key)
            .unwrap_or_else(|| Map::new(&env));

        let video_unlocks = match unlocks.get(video_id) {
            Some(m) => m,
            None => return false,
        };

        match video_unlocks.get(viewer) {
            Some(r) => !r.refunded && r.expires_at > env.ledger().timestamp(),
            None => false,
        }
    }

    /// Returns the Unix timestamp at which the viewer's unlock expires for
    /// the given video. Returns 0 if the viewer has never unlocked the
    /// video. Does not check whether the unlock has already been refunded
    /// or expired; callers can compare the result against the current
    /// ledger time.
    pub fn unlock_expires_at(env: Env, viewer: Address, video_id: Symbol) -> u64 {
        let unlocks_key = Symbol::new(&env, "unlocks");
        let unlocks: Map<Symbol, Map<Address, UnlockRecord>> = env
            .storage()
            .instance()
            .get(&unlocks_key)
            .unwrap_or_else(|| Map::new(&env));

        let video_unlocks = match unlocks.get(video_id) {
            Some(m) => m,
            None => return 0,
        };

        match video_unlocks.get(viewer) {
            Some(r) => r.expires_at,
            None => 0,
        }
    }

    /// Returns the cumulative revenue recorded for a video. Revenue is
    /// incremented on every successful `unlock` call and decremented on
    /// every successful `refund` call. Returns 0 if the video has not been
    /// published.
    pub fn get_revenue(env: Env, video_id: Symbol) -> u32 {
        let videos_key = Symbol::new(&env, "videos");
        let videos: Map<Symbol, Video> = env
            .storage()
            .instance()
            .get(&videos_key)
            .unwrap_or_else(|| Map::new(&env));

        match videos.get(video_id) {
            Some(v) => v.total_revenue,
            None => 0,
        }
    }
}
