pub mod scene;
pub mod shader;

use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    error::Error,
    fmt::{Display, Formatter},
};

/// A reference to an asset.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub struct AssetRef {
    name: Vec<String>,
}
impl AssetRef {
    pub fn new(name: Vec<String>) -> Self {
        Self { name }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        bincode::deserialize(bytes).unwrap()
    }
}

impl Display for AssetRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name.join("/"))
    }
}

pub type AssetTypeId = &'static str;

pub trait AssetData {
    fn id() -> AssetTypeId
    where
        Self: Sized;
    fn to_bytes(&self) -> Result<Cow<[u8]>, impl Error + 'static>
    where
        Self: Sized;
    fn from_bytes(bytes: &[u8]) -> Result<Self, impl Error + 'static>
    where
        Self: Sized;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NeverError {}

impl Display for NeverError {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        unreachable!()
    }
}
impl Error for NeverError {}
