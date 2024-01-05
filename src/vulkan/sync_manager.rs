mod range_map;
pub mod resource_access;

use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, Mutex, MutexGuard},
};

use ash::vk;
use nodit::{interval, InclusiveInterval, Interval};

use self::{
    range_map::{OptRangeMap, RangeMap, RangeMapLike, SmallArrayRangeMap},
    resource_access::{BufferAccess, BufferAccessInfo, ImageAccess, ImageAccessInfo, MipLevel},
};

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

    /// Call this after waiting for the device to be idle.
    pub fn clear_all(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.clear_all();
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
                    .add_buffer_access(buffer.resource.key, buffer.size, access.clone())
                    .into_iter()
                    // If there's no access, then there's no need for a barrier.
                    .filter(|old| old.access() != vk::AccessFlags2::NONE)
                    // Combine all the old accesses into one barrier.
                    .fold(
                        ResourceAccessInfo::empty(),
                        ResourceAccessInfo::into_combined,
                    );

                if wait_for.access() == vk::AccessFlags2::NONE {
                    None
                } else {
                    Some(BufferMemoryBarrier {
                        src_stage_mask: wait_for.stage(),
                        src_access_mask: wait_for.access(),
                        dst_stage_mask: access.stage,
                        dst_access_mask: access.access,
                        src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        buffer: buffer.clone(),
                        offset: access.offset,
                        size: access.size,
                    })
                }
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
                    let wait_for = self.inner.add_image_access(
                        image.resource.key,
                        image.mip_levels as MipLevel,
                        layout,
                        access.clone(),
                    );
                    wait_for.into_iter().map(move |(range, old_layout, old)| {
                        let combined_accesses = old.into_iter().fold(
                            ResourceAccessInfo::empty(),
                            ResourceAccessInfo::into_combined,
                        );
                        ImageMemoryBarrier {
                            src_stage_mask: combined_accesses.stage(),
                            src_access_mask: combined_accesses.access(),
                            dst_stage_mask: access.stage,
                            dst_access_mask: access.access,
                            old_layout,
                            new_layout: layout,
                            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                            image: image.clone(),
                            subresource_range: vk::ImageSubresourceRange {
                                aspect_mask: access.subresource_range.aspect_mask,
                                base_mip_level: range.start() as _,
                                level_count: (range.end() + 1 - range.start()) as _,
                                base_array_layer: access.subresource_range.base_array_layer,
                                layer_count: access.subresource_range.layer_count,
                            },
                        }
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

impl fmt::Debug for BufferResource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BufferResource")
            .field("key", &self.key)
            .finish()
    }
}

impl fmt::Debug for ImageResource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImageResource")
            .field("key", &self.key)
            .finish()
    }
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

#[derive(Clone)]
enum ResourceAccessInfo {
    Read {
        stage: vk::PipelineStageFlags2,
        access: vk::AccessFlags2,
    },
    Write {
        stage: vk::PipelineStageFlags2,
        access: vk::AccessFlags2,
    },
}

impl ResourceAccessInfo {
    fn stage(&self) -> vk::PipelineStageFlags2 {
        match self {
            Self::Read { stage, .. } | Self::Write { stage, .. } => *stage,
        }
    }

    fn access(&self) -> vk::AccessFlags2 {
        match self {
            Self::Read { access, .. } | Self::Write { access, .. } => *access,
        }
    }

    fn combine(&self, other: &Self) -> Self {
        let combined_stage = self.stage() | other.stage();
        let combined_access = self.access() | other.access();
        match (self, other) {
            (Self::Read { .. }, Self::Read { .. }) => Self::Read {
                stage: combined_stage,
                access: combined_access,
            },
            _ => Self::Write {
                stage: combined_stage,
                access: combined_access,
            },
        }
    }

    fn into_combined(self, other: Self) -> Self {
        self.combine(&other)
    }

    fn empty() -> Self {
        ResourceAccessInfo::Read {
            stage: vk::PipelineStageFlags2::NONE,
            access: vk::AccessFlags2::NONE,
        }
    }
}

struct SyncManagerInternal {
    buffers: HashMap<
        BufferResourceKey,
        ResourceRW<vk::DeviceSize, Interval<vk::DeviceSize>, ResourceAccessInfo>,
    >,
    images: HashMap<ImageResourceKey, ResourceRW<MipLevel, Interval<MipLevel>, ResourceAccessInfo>>,
    /// Invariant: All slots in the range map are filled.
    image_layouts: HashMap<ImageResourceKey, OptRangeMap<SmallArrayRangeMap<vk::ImageLayout>>>,
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

    fn add_buffer_access(
        &mut self,
        key: BufferResourceKey,
        max_size: vk::DeviceSize,
        access: BufferAccessInfo,
    ) -> Vec<ResourceAccessInfo> {
        let entry = self
            .buffers
            .entry(key)
            .or_insert_with(|| ResourceRW::new(interval::ie(0, max_size)));

        if access.is_write() {
            entry.add_write(
                access.range(),
                ResourceAccessInfo::Write {
                    stage: access.stage,
                    access: access.access,
                },
            )
        } else {
            entry.add_read(
                access.range(),
                ResourceAccessInfo::Read {
                    stage: access.stage,
                    access: access.access,
                },
                ResourceAccessInfo::into_combined,
            )
        }
    }

    fn add_image_access(
        &mut self,
        key: ImageResourceKey,
        mip_level_count: MipLevel,
        layout: vk::ImageLayout,
        access: ImageAccessInfo,
    ) -> Vec<(Interval<MipLevel>, vk::ImageLayout, Vec<ResourceAccessInfo>)> {
        let max_range = interval::ie(0, mip_level_count);
        assert!(
            access.subresource_range.base_array_layer == 0
                && access.subresource_range.layer_count == 1,
            "Array or 3D images are not supported"
        );
        let layout_entry = self.image_layouts.entry(key).or_insert_with(|| {
            let mut layouts = OptRangeMap::new(max_range);
            layouts.overwrite(max_range, vk::ImageLayout::UNDEFINED);
            layouts
        });

        let old_layouts = layout_entry.overwrite(access.range(), layout);
        assert!(
            old_layouts.len() > 0,
            "All slots in the range map should be filled"
        );
        assert!(old_layouts.iter().all(|(k, _)| k.is_valid()
            && access.range().contains(k.start())
            && access.range().contains(k.end())));
        assert!(old_layouts
            .iter()
            .any(|(k, _)| k.start() == access.range().start()));
        assert!(old_layouts
            .iter()
            .any(|(k, _)| k.end() == access.range().end()));

        // // print old and new layouts and the ranges
        // println!("old layouts:");
        // for (k, v) in old_layouts.iter() {
        //     println!("range: {:?}, layout: {:?}", k, v);
        // }
        // println!("new layout: {:?}", layout);
        // println!("new range: {:?}", access.range());

        let entry = self
            .images
            .entry(key)
            .or_insert_with(|| ResourceRW::new(max_range));

        old_layouts
            .into_iter()
            .map(|(range, old_layout)| {
                (
                    range,
                    old_layout,
                    if access.is_write(layout, Some(old_layout)) {
                        entry.add_write(
                            range,
                            ResourceAccessInfo::Write {
                                stage: access.stage,
                                access: access.access,
                            },
                        )
                    } else {
                        entry.add_read(
                            range,
                            ResourceAccessInfo::Read {
                                stage: access.stage,
                                access: access.access,
                            },
                            ResourceAccessInfo::into_combined,
                        )
                    },
                )
            })
            .collect::<Vec<_>>()
    }

    fn clear_all(&mut self) {
        // Clear the accesses, but not the layouts.
        self.buffers.clear();
        self.images.clear();
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
struct BufferResourceKey(u64);

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
struct ImageResourceKey(u64);

/// Stores the last write, and all *subsequent* reads.
struct ResourceRW<I, K, V>
where
    I: nodit::PointType,
    K: nodit::IntervalType<I> + Copy + std::fmt::Debug,
    V: Clone,
{
    /// The last write to the resource.
    write: OptRangeMap<RangeMap<I, K, V>>,
    /// All subsequent reads from the resource.
    reads: OptRangeMap<RangeMap<I, K, V>>,
}

impl<I, K, V> ResourceRW<I, K, V>
where
    I: nodit::PointType,
    K: nodit::IntervalType<I> + Clone + std::fmt::Debug,
    V: Clone,
{
    fn new(max_range: K) -> Self {
        Self {
            write: OptRangeMap::new(max_range),
            reads: OptRangeMap::new(max_range),
        }
    }

    fn add_write(&mut self, range: K, value: V) -> Vec<V> {
        let old_writes = self.write.overwrite(range, value);
        let old_reads = self.reads.cut(range);

        let mut range_map = RangeMap::new(range.clone());
        for (key, value) in old_writes {
            range_map.overwrite(key, value);
        }
        for (key, value) in old_reads {
            range_map.overwrite(key, value);
        }
        range_map
            .overlapping(&range)
            .map(|(_, v)| v.clone())
            .collect()
    }

    fn add_read(&mut self, range: K, value: V, combine_values: impl Fn(V, V) -> V) -> Vec<V> {
        let reads = self.reads.cut(range);
        for (k, v) in reads {
            let new_value = combine_values(v, value.clone());
            self.reads.overwrite(k, new_value);
        }
        self.write
            .overlapping(&range)
            .map(|(_, v)| v.clone())
            .collect()
    }
}
