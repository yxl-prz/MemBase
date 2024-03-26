// MemBase provided utilities
use crate::{function_signatures, memory_signatures, offsets, util, util::Dispatch};

// Any dependencies you might need
use winapi::um::winuser::VK_END;

pub fn start() -> Dispatch {
    loop {
        if util::is_key_pressed(VK_END) {
            break;
        }
    }

    Dispatch::Success
}
