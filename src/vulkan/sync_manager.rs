mod range_map;
pub mod resource_access;

use std::{
    collections::HashMap,
    ops::{Range, RangeInclusive},
    sync::{Arc, Mutex, MutexGuard},
};

use ash::vk;
use discrete_range_map::{DiscreteRangeMap, InclusiveInterval};

use self::resource_access::{BufferAccess, BufferAccessInfo, ImageAccess, ImageAccessInfo};

use super::command_buffer::{BufferMemoryBarrier, CmdPipelineBarrier, ImageMemoryBarrier};

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
    pub fn get_image(&self, mip_levels: u32) -> ImageResource {
        let mut inner = self.inner.lock().unwrap();

        todo!("Implement mip level tracking");

        ImageResource {
            sync_manager: self.clone(),
            key: inner.get_image(),
        }
    }

    #[must_use]
    pub fn lock(&self) -> SyncManagerLock {
        SyncManagerLock::new(self)
    }

    /// Call this after waiting for the device to be idle.
    pub fn clear_all(&self) {
        let mut inner = self.inner.lock().unwrap();
        for (_, access) in inner.buffers.iter_mut() {
            *access = ResourceRWAccess::new_only_reads(vec![]);
        }
        todo!("Implement image layout tracking");
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
    pub fn add_accesses(
        &mut self,
        buffer_accesses: Vec<BufferAccess>,
        image_accesses: Vec<ImageAccess>,
    ) -> CmdPipelineBarrier {
        // TODO: Optimise this by constructing a smol graph of dependencies and only adding barriers where necessary.
        // e.g. If we know that "A -> B", and then in a shader we read both "A" and "B", then we only need a barrier for "B".
        // TODO: Assert that the image_accesses don't overlap. (e.g. reading from the same image with different layouts. Aka writing to the same image multiple times.)

        let buffer_memory_barriers = buffer_accesses
            .into_iter()
            .flat_map(|BufferAccess { buffer, access }| {
                let wait_for = self
                    .inner
                    .add_buffer_access(buffer.resource.key, access.clone());

                wait_for
                    .into_iter()
                    // If there's no access, then there's no need for a barrier.
                    .filter(|old| old.access != vk::AccessFlags2::NONE)
                    .map(move |old| BufferMemoryBarrier {
                        src_stage_mask: old.stage,
                        src_access_mask: old.access,
                        dst_stage_mask: access.stage,
                        dst_access_mask: access.access,
                        src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        buffer: buffer.clone(),
                        offset: access.offset,
                        size: access.size,
                    })
            })
            .collect();

        let image_memory_barriers = image_accesses
            .into_iter()
            .flat_map(
                |ImageAccess {
                     image,
                     layout,
                     access,
                 }| {
                    let (old_layout, wait_for) =
                        self.inner
                            .add_image_access(image.resource.key, layout, access.clone());
                    wait_for.into_iter().map(move |old| ImageMemoryBarrier {
                        src_stage_mask: old.stage,
                        src_access_mask: old.access,
                        dst_stage_mask: access.stage,
                        dst_access_mask: access.access,
                        old_layout,
                        new_layout: layout,
                        src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        image: image.clone(),
                        subresource_range: access.subresource_range,
                    })
                },
            )
            .collect();

        CmdPipelineBarrier {
            dependency_flags: vk::DependencyFlags::empty(),
            memory_barriers: vec![],
            buffer_memory_barriers,
            image_memory_barriers,
        }
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
    // TODO: Use a https://crates.io/crates/rangemap for granular buffer/image access tracking.
    buffers: HashMap<BufferResourceKey, ResourceRWAccess<BufferAccessInfo>>,
    images: HashMap<ImageResourceKey, ResourceRWAccess<ImageAccessInfo>>,
    image_layouts: HashMap<ImageResourceKey, vk::ImageLayout>,

    buffer_key_counter: u64,
    image_key_counter: u64,
}

impl SyncManagerInternal {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
            images: HashMap::new(),
            image_layouts: HashMap::new(),
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
        self.image_layouts.remove(&key);
    }

    fn resource_write_access<T: VulkanResourceAccess>(
        resources: &mut HashMap<T::Key, ResourceRWAccess<T>>,
        key: T::Key,
        access: T,
    ) -> Vec<T> {
        let old = resources.insert(key, ResourceRWAccess::new_write(access));
        match old {
            Some(ResourceRWAccess {
                write: Some(write),
                reads,
            }) if reads.is_empty() => {
                vec![write.clone()]
            }
            Some(ResourceRWAccess { reads, .. }) => reads,

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
            Entry::Occupied(value) => {
                let ResourceRWAccess { write, reads } = value.into_mut();
                reads.push(access);
                write.clone().map(|w| vec![w]).unwrap_or_default()
            }
            Entry::Vacant(value) => {
                value.insert(ResourceRWAccess::new_only_reads(vec![access]));
                vec![]
            }
        }
    }

    fn add_buffer_access(
        &mut self,
        key: BufferResourceKey,
        access: BufferAccessInfo,
    ) -> Vec<BufferAccessInfo> {
        if access.is_write() {
            Self::resource_write_access(&mut self.buffers, key, access)
        } else {
            Self::resource_read_access(&mut self.buffers, key, access)
        }
    }

    fn add_image_access(
        &mut self,
        key: ImageResourceKey,
        layout: vk::ImageLayout,
        access: ImageAccessInfo,
    ) -> (vk::ImageLayout, Vec<ImageAccessInfo>) {
        assert!(
            access.subresource_range.base_array_layer == 0,
            "Array or 3D images are not supported"
        );
        assert!(
            access.subresource_range.layer_count == 1,
            "Array or 3D images are not supported"
        );

        let old_layout = self.image_layouts.get(&key).map(|v| *v);
        self.image_layouts.insert(key, layout);
        if access.is_write(layout, old_layout) {
            (
                layout,
                Self::resource_write_access(&mut self.images, key, access),
            )
        } else {
            (
                layout,
                Self::resource_read_access(&mut self.images, key, access),
            )
        }
    }
}

trait VulkanResourceAccess: Clone {
    type Key: std::hash::Hash + Eq + Copy;
}

impl VulkanResourceAccess for BufferAccessInfo {
    type Key = BufferResourceKey;
}

impl VulkanResourceAccess for ImageAccessInfo {
    type Key = ImageResourceKey;
}

#[derive(Hash, Eq, PartialEq, Copy, Clone)]
struct BufferResourceKey(u64);

#[derive(Hash, Eq, PartialEq, Copy, Clone)]
struct ImageResourceKey(u64);

/// Stores the last write, and all *subsequent* reads.
struct ResourceRWAccess<T> {
    /// The last write to the resource.
    write: Option<T>,
    /// All subsequent reads from the resource.
    reads: Vec<T>,
}

impl<T> ResourceRWAccess<T> {
    fn new_write(last_write: T) -> Self {
        Self {
            write: Some(last_write),
            reads: vec![],
        }
    }
    fn new_only_reads(reads: Vec<T>) -> Self {
        Self { write: None, reads }
    }
}
