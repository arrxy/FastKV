pub trait Evict {
    fn evict(&self, key: &str) -> Result<(), std::io::Error>;
}
