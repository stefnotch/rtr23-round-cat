use std::sync::Arc;

use ash::vk;

use crate::context::Context;

pub struct Sampler {
    pub sampler: vk::Sampler,
    context: Arc<Context>,
}

impl Sampler {
    pub fn new(sampler: vk::Sampler, context: Arc<Context>) -> Self {
        Self { sampler, context }
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        unsafe {
            self.context.device.destroy_sampler(self.sampler, None);
        }
    }
}
