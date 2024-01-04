use std::{
    collections::HashMap,
    sync::{atomic::AtomicU32, Arc},
};

pub trait Asset {
    fn id(&self) -> AssetId;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AssetId(u32);
impl AssetId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

#[derive(Clone, Debug)]
pub struct AssetIdGenerator {
    next_id: Arc<AtomicU32>,
}

impl AssetIdGenerator {
    pub fn new() -> Self {
        Self {
            next_id: Default::default(),
        }
    }

    pub fn next(&self) -> AssetId {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        AssetId::new(id)
    }
}

pub struct Assets<T: Asset> {
    pub assets: HashMap<AssetId, Arc<T>>,
}

impl<T: Asset> Assets<T> {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }
}

impl<T: Asset> Default for Assets<T> {
    fn default() -> Self {
        Self::new()
    }
}
