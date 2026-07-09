use crate::core::evict::Evict;

pub struct LRUEvict;

impl Evict for LRUEvict {
    fn evict(&self, key: &str) -> Result<(), std::io::Error> {
        Ok(())
    }
}
