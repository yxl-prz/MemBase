# MemBase
A template project to easily develop internal cheats in Rust.

# Objective
Being able to develop an internal cheat after running a `git clone`, and importing the neccesary offsets without further complications on set-up.
- [x] Compile to `.dll`.
- [x] Import data such as offsets, memory signatures, and function signatures at compile-time from a [file](./imports.json)

# Usage
📑 Clone this repository
```
git clone https://github.com/yxl-prz/MemBase
```
⚙️ Add the neccesary data to [`imports.json`](./imports.json).
```jsonc
{
    "offsets": {
        // Hexadecimal validated at compile-time
        "PlayerOffset": "0x17E0A8",
        "HealthOffset": "0x18E0A9"
    },
    "memory_signature": {
        // Validated at compile-time.
        "SomeStructure": "8D 05 ? ? ? ? 48 89 4D"
    },
    "function_signatures": {
        // Argument & return types validated at compile-time
        "DoSomething": {
            "arguments": [
                [ "argument",
                   "*void" ],
                [ "other_argument",
                  "int" ]
            ],
            "return": "*void"
        }
    }
}
```
🧰 Start writing your cheat on [`/src/features/mod.rs`](./src/features/mod.rs) using [MemBase's API](#api).
```rs
// MemBase provided utilities
use crate::{
    function_signatures, memory_signatures, offsets,
    util::{self, Dispatch, Module},
};

// Any dependencies you might need
use winapi::um::winuser::VK_END;

pub fn start() -> Dispatch {
    // Find the module addresses
    let client = Module::get("client.dll").unwrap();

    // Need to add hooks?
    let interface = util::Interface::from_module(client, "InterfaceName");
    let new_fn: function_signatures::DoSomething = |...| { ... };
    interface.vmt.hook(10, new_fn);

    loop {
        // Implement key to unload if needed
        if util::is_key_pressed(VK_END) {
            break;
        }

        // 🖋 Match signatures
        let start_addr = client
            .scan_signature(memory_signatures::SomeStructure.to_vec())
            .unwrap();

        // 🔍 Find values with the .offset method on pointers
        let health = unsafe {
            client
                .offset(offsets::PlayerOffset)
                .offset(offsets::HealthOffset) as *mut i32
        };

        if unsafe { *health } == 50 {
            println!("Half Health!");
        }

        // ❌ Track possible errors
        if something_failed {
            return Dispatch::Error(String::from("Something failed"));
        }

        // Need to loopback?
        if loopback {
            return Dispatch::Loopback;
        }
    }

    Dispatch::Success
}
```

# API
## Structures
All these are available as functions/structures inside the `util` crate imported on the template file [`/src/features/mod.rs`](./src/features/mod.rs).
* `Module`: Get the address of a module.
* `Interface`: Have access to an interface as well as it's VMT.
* `VMT`: Hook functions to an interface from a class address. (Automatically built from Interface)
## Constants
* `crate::offsets::*`: Values inputted in [`imports.json`](./imports.json)
* `crate::memory_signatures::*`: Values inputted in [`imports.json`](./imports.json)
* `crate::config::*`: Data specified in [`config.json`](./config.json)
## Types
* `function_signatures::*`: Function signatures inputted in [`imports.json`](./imports.json)
# Credits
* [zorftw/Ion](https://github.com/zorftw/Ion): Some of the functions from `util.rs` were based on this project.
* [youtube.com/@cazz](https://www.youtube.com/@cazz)

# Disclaimer
This project is intended solely for educational purposes. I do not assume any responsibility for any misuse or unlawful activities that may arise from the use of this software. The primary objective of this project is to deepen the understanding of reverse engineering, with no intention to disrupt or interfere with any legitimate gaming experiences. Users are advised to use this software responsibly and in compliance with applicable laws and regulations.