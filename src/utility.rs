// See: https://github.com/ash-rs/ash/blob/master/examples/src/lib.rs#L30C1-L40C2
// Simple offset_of macro akin to C++ offsetof
#[macro_export]
macro_rules! offset_of {
    ($base:path, $field:ident) => {{
        #[allow(unused_unsafe)]
        unsafe {
            let b: $base = std::mem::zeroed();
            std::ptr::addr_of!(b.$field) as isize - std::ptr::addr_of!(b) as isize
        }
    }};
}

pub fn aligned_size(value: u32, alignment: u32) -> u32 {
    assert!(alignment.is_power_of_two());
    (value + alignment - 1) & !(alignment - 1)
}
