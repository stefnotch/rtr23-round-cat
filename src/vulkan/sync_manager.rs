pub mod resource_access;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

use ash::vk;

use self::resource_access::{BufferAccess, ImageAccess};

/// Does not directly correspond to a Vulkan object.
#[derive(Clone)]
pub struct SyncManager {
    inner: Arc<Mutex<SyncManagerInternal>>,
}

impl SyncManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SyncManagerInternal::new())),
        }
    }

    #[must_use]
    pub fn get_buffer(&self) -> BufferResource {
        let mut inner = self.inner.lock().unwrap();
        BufferResource {
            sync_manager: self.clone(),
            key: inner.get_buffer(),
        }
    }

    #[must_use]
    pub fn get_image(&self) -> ImageResource {
        let mut inner = self.inner.lock().unwrap();
        ImageResource {
            sync_manager: self.clone(),
            key: inner.get_image(),
        }
    }

    #[must_use]
    pub fn lock(&self) -> SyncManagerLock {
        SyncManagerLock::new(self)
    }
}

pub struct SyncManagerLock<'a> {
    inner: MutexGuard<'a, SyncManagerInternal>,
}

impl<'a> SyncManagerLock<'a> {
    pub fn new(sync_manager: &'a SyncManager) -> Self {
        Self {
            inner: sync_manager.inner.lock().unwrap(),
        }
    }

    #[must_use]
    pub fn add_buffer_access(
        &mut self,
        resource: &BufferResource,
        access: BufferAccess,
    ) -> Vec<BufferAccess> {
        self.inner.add_buffer_access(resource.key, access)
    }

    #[must_use]
    pub fn add_image_access(
        &mut self,
        resource: &ImageResource,
        access: ImageAccess,
    ) -> Vec<ImageAccess> {
        self.inner.add_image_access(resource.key, access)
    }
}

pub struct BufferResource {
    sync_manager: SyncManager,
    key: BufferResourceKey,
}

pub struct ImageResource {
    sync_manager: SyncManager,
    key: ImageResourceKey,
}

impl Drop for BufferResource {
    fn drop(&mut self) {
        let mut inner = self.sync_manager.inner.lock().unwrap();
        inner.remove_buffer(self.key);
    }
}

impl Drop for ImageResource {
    fn drop(&mut self) {
        let mut inner = self.sync_manager.inner.lock().unwrap();
        inner.remove_image(self.key);
    }
}

// Internals //

struct SyncManagerInternal {
    buffers: HashMap<BufferResourceKey, ResourceRWAccess<BufferAccess>>,
    images: HashMap<ImageResourceKey, ResourceRWAccess<ImageAccess>>,

    buffer_key_counter: u64,
    image_key_counter: u64,
}

impl SyncManagerInternal {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
            images: HashMap::new(),
            buffer_key_counter: 0,
            image_key_counter: 0,
        }
    }

    fn get_buffer(&mut self) -> BufferResourceKey {
        let key = BufferResourceKey(self.buffer_key_counter);
        self.buffer_key_counter += 1;
        key
    }

    fn get_image(&mut self) -> ImageResourceKey {
        let key = ImageResourceKey(self.image_key_counter);
        self.image_key_counter += 1;
        key
    }

    fn remove_buffer(&mut self, key: BufferResourceKey) {
        self.buffers.remove(&key);
    }

    fn remove_image(&mut self, key: ImageResourceKey) {
        self.images.remove(&key);
    }

    fn resource_write_access<T: VulkanResourceAccess>(
        resources: &mut HashMap<T::Key, ResourceRWAccess<T>>,
        key: T::Key,
        access: T,
    ) -> Vec<T> {
        let old = resources.insert(key, ResourceRWAccess::new_write(access));
        match old {
            Some(ResourceRWAccess::WriteThenRead(last_write, reads)) if reads.is_empty() => {
                vec![last_write]
            }
            Some(ResourceRWAccess::WriteThenRead(.., reads)) => reads,
            Some(ResourceRWAccess::OnlyReads(reads)) => reads,
            None => vec![],
        }
    }
    fn resource_read_access<T: VulkanResourceAccess>(
        resources: &mut HashMap<T::Key, ResourceRWAccess<T>>,
        key: T::Key,
        access: T,
    ) -> Vec<T> {
        use std::collections::hash_map::Entry;
        match resources.entry(key) {
            Entry::Occupied(value) => match value.into_mut() {
                ResourceRWAccess::WriteThenRead(last_write, reads) => {
                    reads.push(access);
                    vec![last_write.clone()]
                }
                ResourceRWAccess::OnlyReads(reads) => {
                    reads.push(access);
                    vec![]
                }
            },
            Entry::Vacant(value) => {
                value.insert(ResourceRWAccess::OnlyReads(vec![access]));
                vec![]
            }
        }
    }

    fn add_buffer_access(
        &mut self,
        key: BufferResourceKey,
        access: BufferAccess,
    ) -> Vec<BufferAccess> {
        if access.is_write() {
            Self::resource_write_access(&mut self.buffers, key, access)
        } else {
            Self::resource_read_access(&mut self.buffers, key, access)
        }
    }

    fn add_image_access(&mut self, key: ImageResourceKey, access: ImageAccess) -> Vec<ImageAccess> {
        let old_layout = self.images.get(&key).and_then(|access| match access {
            ResourceRWAccess::WriteThenRead(last_write, _) => Some(last_write.layout),
            ResourceRWAccess::OnlyReads(_) => None,
        });
        if access.is_write(old_layout) {
            Self::resource_write_access(&mut self.images, key, access)
        } else {
            Self::resource_read_access(&mut self.images, key, access)
        }
    }
}

trait VulkanResourceAccess: Clone {
    type Key: std::hash::Hash + Eq + Copy;
}

impl VulkanResourceAccess for BufferAccess {
    type Key = BufferResourceKey;
}

impl VulkanResourceAccess for ImageAccess {
    type Key = ImageResourceKey;
}

#[derive(Hash, Eq, PartialEq, Copy, Clone)]
struct BufferResourceKey(u64);

#[derive(Hash, Eq, PartialEq, Copy, Clone)]
struct ImageResourceKey(u64);

/// Stores the last write, and all *subsequent* reads.
enum ResourceRWAccess<T> {
    OnlyReads(Vec<T>),
    WriteThenRead(T, Vec<T>),
}

impl<T> ResourceRWAccess<T> {
    fn new_write(last_write: T) -> Self {
        Self::WriteThenRead(last_write, Vec::new())
    }
}
