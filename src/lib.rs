#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::c_void;
use std::ptr;
use thiserror::Error;
use winapi::shared::minwindef::HMODULE;
use winapi::um::libloaderapi::{LoadLibraryA, GetProcAddress};

const DLL_NAME: &[u8] = b"dsmcdll.dll\0";
pub const SMC_NAND_BLOCK_SZ: i32 = 0x200;  // 512 bytes

#[derive(Debug, Error)]
pub enum DSmcError {
    #[error("Alignment error")]
    AlignmentError,
    #[error("This implementation is only compatible with version: 3, got: {0}")]
    InvalidVersion(i32),
    #[error("Generic error, HRESULT: {0:#x}")]
    GenericError(i32),
}

#[repr(C)]
pub struct DSMCObject {
    vtable: *const DSMCObjectVTable,
}

#[repr(C)]
struct DSMCObjectVTable {
    /* 0x00 */ GetInterfaceVersion: unsafe extern "C" fn(*mut DSMCObject) -> i32,
    /* 0x08 */ Release: unsafe extern "C" fn(*mut DSMCObject) -> c_void,

    /* 0x10 */ Initialize: unsafe extern "C" fn(*mut DSMCObject, i32) -> i32,
    /* 0x18 */ BeginProgramming: unsafe extern "C" fn(*mut DSMCObject) -> i32,
    /* 0x20 */ RegisterProgress: unsafe extern "C" fn(*mut DSMCObject, *mut c_void, *mut c_void) -> i32,
    /* 0x28 */ BlockWrite: unsafe extern "C" fn(*mut DSMCObject, i32, *mut c_void, i32) -> i32,
    /* 0x30 */ BlockRead: unsafe extern "C" fn(*mut DSMCObject, i32, *mut c_void, i32) -> i32,
    /* 0x38 */ EndProgramming: unsafe extern "C" fn(*mut DSMCObject) -> i32,
    /* 0x40 */ PowerButton: unsafe extern "C" fn(*mut DSMCObject) -> i32,
    /* 0x48 */ SetSafeTransferMode: unsafe extern "C" fn(*mut DSMCObject, bool) -> i32,
    /* 0x50 */ GetExpDigest1SMCBL: unsafe extern "C" fn(*mut DSMCObject, *mut c_void, *mut c_void) -> i32,
    /* 0x58 */ SetExitEvent: unsafe extern "C" fn(*mut DSMCObject) -> i32,
}

type CreateDSmcObjectPtr = unsafe extern "C" fn(*mut *mut DSMCObject) -> i32;

pub trait DSMCFunctions {
    fn get_interface_version(&self) -> Result<i32, DSmcError>;
    fn release(&self);

    fn initialize(&self, port_number: i32) -> Result<(), DSmcError>;
    fn begin_programming(&self) -> Result<(), DSmcError>;
    /// # Safety
    ///
    /// This function takes two unsafe arguments, callback is a function, unknown is some sort of state
    unsafe fn register_progress(&self, callback: *mut c_void, state: *mut c_void) -> Result<(), DSmcError>;
    fn block_write(&self, start_sector: i32, buf: &[u8]) -> Result<(), DSmcError>;
    fn block_read(&self, start_sector: i32, sector_count: i32) -> Result<Vec<u8>, DSmcError>;
    fn end_programming(&self) -> Result<(), DSmcError>;
    fn power_button(&self) -> Result<(), DSmcError>;
    fn set_safe_transfer_mode(&self, safe: bool) -> Result<(), DSmcError>;
    fn get_exp_digest_1smcbl(&self) -> Result<Vec<u8>, DSmcError>;
    fn set_exit_event(&self) -> Result<(), DSmcError>;
}

pub struct DSMC {
    object: *mut DSMCObject,
}

impl DSMC {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let module: HMODULE = unsafe { LoadLibraryA(DLL_NAME.as_ptr() as *const i8) };
        if module.is_null() {
            return Err("dsmcdll.dll failed to load!\nmake sure you have the SDK/FTDI drivers installed".into());
        }

        let create_dsmc_object: Option<CreateDSmcObjectPtr> = unsafe {
            std::mem::transmute(GetProcAddress(module, c"CreateDSmcObject".as_ptr()))
        };

        if let Some(create_dsmc_object) = create_dsmc_object {
            let mut dsmc: *mut DSMCObject = ptr::null_mut();
            let result = unsafe { create_dsmc_object(&mut dsmc) };

            if result != 0 {
                let msg = format!("CreateDSmcObject failed: {:#x}", result);
                return Err(msg.into());
            }

            Ok(Self {
                object: dsmc
            })
        } else {
            Err("error: GetProcAddress(CreateDSmcObject) failed!".into())
        }
    }
}

macro_rules! dsmc_call {
    ($self:expr, $fn:ident $(, $arg:expr)*) => {{
        let res = unsafe { ((*(*$self.object).vtable).$fn)($self.object $(, $arg)*) };
        if res != 0 {
            Err(DSmcError::GenericError(res))
        } else {
            Ok(())
        }
    }}
}

impl DSMCFunctions for DSMC {
    fn get_interface_version(&self) -> Result<i32, DSmcError> {
        let ret = unsafe { ((*(*self.object).vtable).GetInterfaceVersion)(self.object) };
        Ok(ret)
    }

    fn release(&self) {
        unsafe { ((*(*self.object).vtable).Release)(self.object) };
    }

    fn initialize(&self, port_number: i32) -> Result<(), DSmcError> {
        dsmc_call!(self, Initialize, port_number)
    }

    fn begin_programming(&self) -> Result<(), DSmcError> {
        dsmc_call!(self, BeginProgramming)
    }

    unsafe fn register_progress(&self, callback: *mut c_void, unknown: *mut c_void) -> Result<(), DSmcError> {
        dsmc_call!(self, RegisterProgress, callback, unknown)
    }

    fn block_write(&self, start_sector: i32, buf: &[u8]) -> Result<(), DSmcError> {
        if (buf.len() % SMC_NAND_BLOCK_SZ as usize) != 0 {
            return Err(DSmcError::AlignmentError);
        }
        dsmc_call!(self, BlockWrite, start_sector, buf.as_ptr() as *mut c_void, (buf.len() / SMC_NAND_BLOCK_SZ as usize) as i32)
    }

    fn block_read(&self, start_sector: i32, sector_count: i32) -> Result<Vec<u8>, DSmcError> {
        let mut buf = vec![0u8; (sector_count * SMC_NAND_BLOCK_SZ) as usize];
        let res = unsafe { ((*(*self.object).vtable).BlockRead)(self.object, start_sector, buf.as_mut_ptr() as *mut c_void, sector_count) };
        if res != 0 {
            return Err(DSmcError::GenericError(res));
        }

        Ok(buf)
    }

    fn end_programming(&self) -> Result<(), DSmcError> {
        dsmc_call!(self, EndProgramming)
    }

    fn power_button(&self) -> Result<(), DSmcError> {
        dsmc_call!(self, PowerButton)
    }

    fn set_safe_transfer_mode(&self, safe: bool) -> Result<(), DSmcError> {
        dsmc_call!(self, SetSafeTransferMode, safe)
    }

    fn get_exp_digest_1smcbl(&self) -> Result<Vec<u8>, DSmcError> {
        let mut digest = vec![0u8; 16];
        let mut unknown = vec![0u8; 100];
        let res = unsafe { ((*(*self.object).vtable).GetExpDigest1SMCBL)(
            self.object,
            digest.as_mut_ptr() as *mut c_void,
            unknown.as_mut_ptr() as *mut c_void
        ) };
        if res != 0 {
            return Err(DSmcError::GenericError(res));
        }

        Ok(digest)
    }

    fn set_exit_event(&self) -> Result<(), DSmcError> {
        dsmc_call!(self, SetExitEvent)
    }
}