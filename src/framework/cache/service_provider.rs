use std::sync::Arc;
use crate::framework::foundation::{Container, ServiceProvider};
use super::memory_store::MemoryStore;

pub struct CacheServiceProvider;

impl ServiceProvider for CacheServiceProvider {
    fn register(&self, container: &Container) {
        container.singleton::<MemoryStore, _>(|_| Arc::new(MemoryStore::new()));
    }
}
