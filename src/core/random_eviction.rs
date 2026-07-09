use crate::core::eval::RedisState;

pub fn all_keys_random_evict(state: &mut RedisState) -> Result<(), std::io::Error> {
    Ok(())
}

pub fn volatile_random_evict(state: &mut RedisState) -> Result<(), std::io::Error> {
    Ok(())
}