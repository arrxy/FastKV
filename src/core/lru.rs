use crate::core::eval::RedisState;

pub fn all_keys_lru_evict(state: &mut RedisState) -> Result<(), std::io::Error> {
    Ok(())
}

pub fn volatile_lru_evict(state: &mut RedisState) -> Result<(), std::io::Error> {
    Ok(())
}