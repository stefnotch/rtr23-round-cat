use std::{collections::HashMap, sync::Arc};

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

pub struct AssetIdGenerator {
    next_id: u32,
}

impl AssetIdGenerator {
    pub fn new() -> Self {
        Self { next_id: 0 }
    }

    pub fn next(&mut self) -> AssetId {
        let id = self.next_id;
        self.next_id += 1;
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
