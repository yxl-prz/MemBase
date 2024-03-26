// Utility functions
#![allow(dead_code)]

use winapi::{
    ctypes::{c_char, c_int, c_void},
    shared::minwindef::{HINSTANCE__, HMODULE},
    um::{
        libloaderapi::{FreeLibrary, GetModuleHandleA, GetModuleHandleExW, GetProcAddress},
        memoryapi::VirtualProtect,
        processthreadsapi::GetCurrentProcess,
        psapi::{GetModuleInformation, MODULEINFO},
        winnt::PAGE_READWRITE,
        winuser::GetAsyncKeyState,
    },
};

pub enum Dispatch {
    Success,       // Deatach
    Loopback,      // Invoke again
    Error(String), // Print & Deatach
}

// Functions
fn get_proc_address(module: HMODULE, name: *const u8) -> *const c_void {
    unsafe { return GetProcAddress(module, name as _) as _ }
}

fn get_module_handle(name: &str) -> HMODULE {
    let name = format!("{}\0", name);
    let name: Vec<i8> = (name).bytes().map(|x| x.to_owned() as i8).collect();

    unsafe { return GetModuleHandleA(name.to_vec().as_ptr()) }
}

pub fn capture_interface(module: HMODULE, interface: *const u8) -> *const c_void {
    unsafe {
        let fn_addr = get_proc_address(module, b"CreateInterface\0".as_ptr());

        let fn_capture_interface = std::mem::transmute::<
            *const c_void,
            extern "system" fn(*const c_char, *const c_int) -> *const c_void,
        >(fn_addr);

        let interface_addr = fn_capture_interface(interface as _, std::ptr::null_mut());

        if !interface_addr.is_null() {
            return interface_addr;
        }
    }
    std::ptr::null_mut()
}

pub fn get_virtual_function(base: *mut usize, idx: isize) -> *mut usize {
    unsafe { { *base as *mut usize }.offset(idx).read() as *mut usize }
}

pub fn is_key_pressed(key: i32) -> bool {
    unsafe { (GetAsyncKeyState(key) as u16 & 0x8000) != 0 }
}

// Stuctures
#[derive(Debug)]
pub struct VMT {
    new_vtable: Vec<usize>,
    vtable: Vec<usize>,
    class_address: *mut usize,
}

unsafe impl Send for VMT {}
impl VMT {
    pub fn new(class_addr: *mut usize) -> Self {
        let mut vtable: Vec<usize> = Vec::new();
        let class = class_addr as *mut *mut usize;

        let mut offset: isize = 0;
        unsafe {
            while class.read().offset(offset).read() > 0 {
                vtable.push(class.read().offset(offset).read());
                offset += 1;
            }
        }

        VMT {
            class_address: class_addr,
            vtable,
            new_vtable: vec![0; offset as usize],
        }
    }

    pub fn hook(&mut self, idx: isize, new_fn: usize) {
        self.new_vtable[idx as usize] = new_fn;

        unsafe {
            let class = self.class_address as *mut *mut usize;

            let mut protection = 0;
            VirtualProtect(
                class.read().offset(idx) as _,
                4,
                PAGE_READWRITE,
                &mut protection,
            );

            class.read().offset(idx).write(new_fn);

            VirtualProtect(
                class.read().offset(idx) as _,
                4,
                protection,
                std::ptr::null_mut(),
            );
        }
    }

    pub fn reset(&mut self, idx: isize) {
        let original_fn = self.vtable[idx as usize];
        self.hook(idx, original_fn);
    }

    pub fn get_original(&self, idx: isize) -> usize {
        self.vtable[idx as usize]
    }
}

impl Drop for VMT {
    // Reset the VMT of the interface when value dropped
    fn drop(&mut self) {
        let vtable = self.vtable.clone();
        for (idx, _fn_ptr) in vtable.iter().enumerate() {
            self.reset(idx as isize);
        }
    }
}

pub struct Interface {
    pub handle: Module,
    pub interface: *mut usize,
    pub vmt: VMT,
}

impl Interface {
    pub fn new(module_name: &str, interface_name: &str) -> Self {
        let h = Module::get(module_name).unwrap();
        Self::from_module(h, interface_name)
    }
    pub fn from_module(module: Module, interface_name: &str) -> Self {
        let h = module;
        let i = capture_interface(
            h.handle,
            { format!("{}\0", interface_name).bytes().collect::<Vec<u8>>() }.as_ptr(),
        ) as *mut usize;
        let v = VMT::new(i);
        Self {
            handle: h,
            interface: i,
            vmt: v,
        }
    }
}

pub struct Module {
    handle: *mut HINSTANCE__,
    limits: (*mut u8, *mut u8),
}

impl Module {
    pub fn get(module_name: &str) -> Option<Self> {
        unsafe {
            let module_name: Vec<u16> = module_name
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let mut module: *mut HINSTANCE__ = std::ptr::null_mut();
            let start: *mut u8;
            let end: *mut u8;
            if GetModuleHandleExW(0, module_name.as_ptr(), &mut module) == 0 {
                return None;
            }
            let mut info: MODULEINFO = std::mem::zeroed();
            if GetModuleInformation(
                GetCurrentProcess(),
                module,
                &mut info as *mut MODULEINFO,
                std::mem::size_of::<MODULEINFO>() as u32,
            ) == 0
            {
                FreeLibrary(module);
                return None;
            }

            start = info.lpBaseOfDll as *mut u8;
            end = start.offset(info.SizeOfImage as isize).offset(-1);

            Some(Self {
                handle: module,
                limits: (start, end),
            })
        }
    }
    pub fn scan_signature(&self, signature: Vec<Option<u8>>) -> Option<*mut u8> {
        if self.limits.0.is_null() || self.limits.1.is_null() {
            return None;
        }
        let mut res: Option<*mut u8> = None;
        let mut current = self.limits.0;
        let mut offset = 0;
        unsafe {
            while current <= self.limits.1 {
                if signature[offset] == None || signature[offset] == Some(*self.limits.1) {
                    if signature.len() <= offset + 1 {
                        if res.is_some() {
                            return None;
                        }
                        res = Some(current.offset(-(offset as isize)));
                        current = current.offset(-(offset as isize));
                        offset = 0;
                    } else {
                        offset += 1;
                    }
                } else {
                    current = current.offset(-(offset as isize));
                    offset = 0;
                }

                current = current.offset(1);
            }
        }

        None
    }
    pub fn offset(&self, i: isize) -> *mut u8 {
        unsafe { self.handle.offset(i) as *mut u8 }
    }
}

impl Drop for Module {
    fn drop(&mut self) {
        unsafe {
            FreeLibrary(self.handle);
        }
    }
}

// Macros
#[macro_export]
macro_rules! CStruct {
    ($name:ident { $($field:ident : $type:ty),* $(,)? }) => {
        #[repr(C)]
        #[derive(Debug)]
        pub struct $name {
            $(pub $field: $type),*
        }
    };
}
