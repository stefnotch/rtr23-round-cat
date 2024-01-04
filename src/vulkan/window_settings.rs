use ash::vk;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]

pub enum PresentMode {
    Immediate,
    Mailbox,
    Fifo,
    FifoRelaxed,
}

impl From<PresentMode> for vk::PresentModeKHR {
    fn from(mode: PresentMode) -> Self {
        match mode {
            PresentMode::Immediate => vk::PresentModeKHR::IMMEDIATE,
            PresentMode::Mailbox => vk::PresentModeKHR::MAILBOX,
            PresentMode::Fifo => vk::PresentModeKHR::FIFO,
            PresentMode::FifoRelaxed => vk::PresentModeKHR::FIFO_RELAXED,
        }
    }
}
