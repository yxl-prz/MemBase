use winapi::{
    ctypes::c_void,
    shared::minwindef::{BOOL, DWORD, HMODULE, LPVOID, TRUE},
    um::{
        consoleapi::AllocConsole,
        handleapi::CloseHandle,
        libloaderapi::{DisableThreadLibraryCalls, FreeLibraryAndExitThread},
        processthreadsapi::CreateThread,
        wincon::{FreeConsole, SetConsoleTitleA},
        winnt::DLL_PROCESS_ATTACH,
    },
};

mod config;
mod features;
mod function_signatures;
mod memory_signatures;
mod offsets;
mod util;

pub unsafe extern "system" fn actual_dll_main(instance: *mut c_void) -> u32 {
    let res = std::panic::catch_unwind(|| {
        if config::CONSOLE {
            AllocConsole();
            SetConsoleTitleA(
                format!("{}\0", config::NAME)
                    .bytes()
                    .collect::<Vec<u8>>()
                    .as_ptr() as _,
            );
            println!("[DllMain] Console Allocated");
            println!("[DllMain] Calling start()");
        }
        loop {
            let d = features::start();
            match d {
                util::Dispatch::Success => {
                    println!("[Dll] start() exitted succesfuly");
                    break;
                }
                util::Dispatch::Error(r) => {
                    println!("[DllMain] start() returned an error\n     - '{}'", r);
                    break;
                }
                util::Dispatch::Loopback => {
                    continue;
                }
            }
        }
        if config::CONSOLE {
            println!("[DllMain] Unloading...");
            FreeConsole();
        }
        FreeLibraryAndExitThread(instance as _, 0)
    });

    match res {
        Err(e) => println!("Error: {:?}", e),
        _ => {}
    };

    0
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "system" fn DllMain(module: HMODULE, reason: DWORD, _: LPVOID) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        DisableThreadLibraryCalls(module);
        let thread_handle = CreateThread(
            std::ptr::null_mut(),
            0,
            Some(actual_dll_main),
            module as *mut _,
            0,
            std::ptr::null_mut(),
        );

        if !thread_handle.is_null() {
            CloseHandle(thread_handle);
        }
    }
    TRUE
}
