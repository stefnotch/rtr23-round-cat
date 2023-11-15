use std::sync::Arc;

use ash::vk;

use crate::context::Context;

pub struct Sampler {
    pub inner: vk::Sampler,
    context: Arc<Context>,
}

impl Sampler {
    pub fn new(sampler: vk::Sampler, context: Arc<Context>) -> Self {
        Self {
            inner: sampler,
            context,
        }
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        unsafe {
            self.context.device.destroy_sampler(self.inner, None);
        }
    }
}
