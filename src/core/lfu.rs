use crate::core::eval::RedisState;

pub fn all_keys_lfu_evict(state: &mut RedisState) -> Result<(), std::io::Error> {
    Ok(())
}

pub fn volatile_lfu_evict(state: &mut RedisState) -> Result<(), std::io::Error> {
    Ok(())
}
