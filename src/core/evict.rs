use crate::core::{eval::{EvictionPolicy, RedisState}, lfu::{all_keys_lfu_evict, volatile_lfu_evict}, lru::{all_keys_lru_evict, volatile_lru_evict}, random_eviction::{all_keys_random_evict, volatile_random_evict}, ttl_evict::volatile_ttl_evict};

pub fn evict(state: &mut RedisState) -> Result<(), std::io::Error> {
    match state.eviction_policy {
        EvictionPolicy::NoEviction => {},
        EvictionPolicy::AllKeysRandom => all_keys_random_evict(state)?,
        EvictionPolicy::VolatileRandom => volatile_random_evict(state)?,
        EvictionPolicy::AllKeysLru => all_keys_lru_evict(state)?,
        EvictionPolicy::VolatileLru => volatile_lru_evict(state)?,
        EvictionPolicy::AllKeysLfu => all_keys_lfu_evict(state)?,
        EvictionPolicy::VolatileLfu => volatile_lfu_evict(state)?,
        EvictionPolicy::VolatileTtl => volatile_ttl_evict(state)?,
    };
    Ok(())
}