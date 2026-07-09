use crate::core::eval::RedisState;

pub fn volatile_ttl_evict(state: &mut RedisState) -> Result<(), std::io::Error> {
    Ok(())
}