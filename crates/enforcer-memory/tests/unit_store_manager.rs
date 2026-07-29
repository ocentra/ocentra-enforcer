use enforcer_memory::store_manager::{
    InProcessSupervisor, IndexSupervisor, StoreCache, DEFAULT_IDLE_TIMEOUT,
    INDEX_SUPERVISOR_ENV_VAR,
};
use std::time::{Duration, Instant};

fn cache_clock(value: Instant) -> enforcer_domain::memory_types::StoreCacheInstant {
    enforcer_domain::boundary::core::store_cache_instant(value)
}

#[test]
fn cache_miss_opens_and_caches_the_value() {
    let mut cache: StoreCache<String, u32> = StoreCache::new();
    assert!(cache.is_empty());
    let now = Instant::now();

    let mut open_calls = 0;
    {
        let value = cache.get_or_insert_with("proj-a".to_owned(), cache_clock(now), || {
            open_calls += 1;
            42
        });
        assert_eq!(*value, 42);
    }
    assert_eq!(open_calls, 1);
    assert!(bool::from(cache.contains(&"proj-a".to_owned())));
    assert_eq!(cache.len(), 1);
}

#[test]
fn cache_hit_does_not_reopen() {
    let mut cache: StoreCache<String, u32> = StoreCache::new();
    assert!(cache.is_empty());
    let now = Instant::now();
    let mut open_calls = 0;

    cache.get_or_insert_with("proj-a".to_owned(), cache_clock(now), || {
        open_calls += 1;
        1
    });
    cache.get_or_insert_with("proj-a".to_owned(), cache_clock(now), || {
        open_calls += 1;
        2
    });

    assert_eq!(
        open_calls, 1,
        "second call must be a cache hit, not a reopen"
    );
}

#[test]
fn evict_idle_removes_entries_past_the_timeout_no_sleep() {
    let mut cache: StoreCache<String, u32> = StoreCache::with_idle_timeout(Duration::from_secs(60));
    let t0 = Instant::now();
    cache.get_or_insert_with("proj-a".to_owned(), cache_clock(t0), || 1);

    let t_later = t0 + Duration::from_secs(61);
    let evicted = cache.evict_idle(cache_clock(t_later));

    assert_eq!(evicted, vec!["proj-a".to_owned()]);
    assert!(cache.is_empty());
}

#[test]
fn evict_idle_keeps_entries_touched_within_the_timeout() {
    let mut cache: StoreCache<String, u32> = StoreCache::with_idle_timeout(Duration::from_secs(60));
    let t0 = Instant::now();
    cache.get_or_insert_with("proj-a".to_owned(), cache_clock(t0), || 1);

    let t_soon = t0 + Duration::from_secs(30);
    let evicted = cache.evict_idle(cache_clock(t_soon));

    assert!(evicted.is_empty());
    assert!(bool::from(cache.contains(&"proj-a".to_owned())));
}

#[test]
fn touching_an_entry_resets_its_idle_clock() {
    let mut cache: StoreCache<String, u32> = StoreCache::with_idle_timeout(Duration::from_secs(60));
    let t0 = Instant::now();
    cache.get_or_insert_with("proj-a".to_owned(), cache_clock(t0), || 1);

    let t_touch = t0 + Duration::from_secs(50);
    cache.get_or_insert_with("proj-a".to_owned(), cache_clock(t_touch), || 999);

    let t_after_touch = t_touch + Duration::from_secs(55);
    let evicted = cache.evict_idle(cache_clock(t_after_touch));

    assert!(
        evicted.is_empty(),
        "a recently-touched entry must not be evicted just because its ORIGINAL insert is old"
    );
}

#[test]
fn eviction_never_drops_a_store_mid_use() {
    let mut cache: StoreCache<String, u32> = StoreCache::with_idle_timeout(Duration::from_secs(60));
    let now = Instant::now();
    cache.get_or_insert_with("proj-a".to_owned(), cache_clock(now), || 1);

    let evicted = cache.evict_idle(cache_clock(now));

    assert!(evicted.is_empty());
    assert!(bool::from(cache.contains(&"proj-a".to_owned())));
}

#[test]
fn evict_idle_handles_multiple_projects_independently() {
    let mut cache: StoreCache<String, u32> = StoreCache::with_idle_timeout(Duration::from_secs(60));
    let t0 = Instant::now();
    cache.get_or_insert_with("stale".to_owned(), cache_clock(t0), || 1);

    let t1 = t0 + Duration::from_secs(30);
    cache.get_or_insert_with("fresh".to_owned(), cache_clock(t1), || 2);

    let t_check = t0 + Duration::from_secs(61);
    let evicted = cache.evict_idle(cache_clock(t_check));

    assert_eq!(evicted, vec!["stale".to_owned()]);
    assert!(bool::from(cache.contains(&"fresh".to_owned())));
    assert_eq!(cache.len(), 1);
}

#[test]
fn remove_explicitly_drops_an_entry_regardless_of_idle_state() {
    let mut cache: StoreCache<String, u32> = StoreCache::new();
    let now = Instant::now();
    cache.get_or_insert_with("proj-a".to_owned(), cache_clock(now), || 1);

    let removed = cache.remove(&"proj-a".to_owned());

    assert_eq!(removed, Some(1));
    assert!(cache.is_empty());
}

#[test]
fn in_process_supervisor_runs_work_synchronously_and_returns_its_result() {
    let supervisor = InProcessSupervisor;
    let result = supervisor.run(|| 2 + 2);
    assert_eq!(result, 4);
}

#[test]
fn default_idle_timeout_matches_baseline_sixty_seconds() {
    assert_eq!(DEFAULT_IDLE_TIMEOUT, Duration::from_secs(60));
}

#[test]
fn index_supervisor_env_var_name_matches_baseline() {
    assert_eq!(INDEX_SUPERVISOR_ENV_VAR, "CBM_INDEX_SUPERVISOR");
}
