use std::ptr;

use crate::bindings::{WT_CONFIG_ARG, WT_CONNECTION, WT_ENCRYPTOR, WT_SESSION};
use tracing::{info, instrument};

mod bindings;

#[repr(C)]
struct NopEncryptor {
    encryptor: WT_ENCRYPTOR, // Must be first field for C type-punning
}

#[instrument(skip_all)]
unsafe extern "C" fn nop_encrypt(
    _encryptor: *mut WT_ENCRYPTOR,
    _session: *mut WT_SESSION,
    src: *mut u8,
    src_len: usize,
    dst: *mut u8,
    dst_len: usize,
    result_lenp: *mut usize,
) -> ::std::os::raw::c_int {
    if dst_len < src_len {
        return libc::ENOMEM;
    }
    unsafe {
        ptr::copy_nonoverlapping(src, dst, src_len);
        *result_lenp = src_len;
    }
    0
}

#[instrument(skip_all)]
unsafe extern "C" fn nop_decrypt(
    _encryptor: *mut WT_ENCRYPTOR,
    _session: *mut WT_SESSION,
    src: *mut u8,
    _src_len: usize,
    dst: *mut u8,
    dst_len: usize,
    result_lenp: *mut usize,
) -> ::std::os::raw::c_int {
    // dst_len is the number of unencrypted bytes expected back
    unsafe {
        ptr::copy_nonoverlapping(src, dst, dst_len);
        *result_lenp = dst_len;
    }
    0
}

#[instrument(skip_all)]
unsafe extern "C" fn nop_sizing(
    _encryptor: *mut WT_ENCRYPTOR,
    _session: *mut WT_SESSION,
    expansion_constantp: *mut usize,
) -> ::std::os::raw::c_int {
    unsafe { *expansion_constantp = 0 };
    0
}

#[instrument(skip_all)]
unsafe extern "C" fn nop_customize(
    _encryptor: *mut WT_ENCRYPTOR,
    _session: *mut WT_SESSION,
    _encrypt_config: *mut WT_CONFIG_ARG,
    customp: *mut *mut WT_ENCRYPTOR,
) -> ::std::os::raw::c_int {
    // Returning NULL means WiredTiger uses the original encryptor for all keys
    unsafe { *customp = ptr::null_mut() };
    0
}

#[instrument(skip_all)]
unsafe extern "C" fn nop_terminate(
    encryptor: *mut WT_ENCRYPTOR,
    _session: *mut WT_SESSION,
) -> ::std::os::raw::c_int {
    unsafe { drop(Box::from_raw(encryptor as *mut NopEncryptor)) };
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn wiredtiger_extension_init(
    connection: *mut WT_CONNECTION,
    _config: *mut WT_CONFIG_ARG,
) -> ::std::os::raw::c_int {
    tracing_subscriber::fmt().init();
    // eprintln!("Testing...");
    info!("Rust extension loaded");

    let nop = Box::new(NopEncryptor {
        encryptor: WT_ENCRYPTOR {
            encrypt: Some(nop_encrypt),
            decrypt: Some(nop_decrypt),
            sizing: Some(nop_sizing),
            customize: Some(nop_customize),
            terminate: Some(nop_terminate),
        },
    });
    unsafe {
        let add_en = (*connection).add_encryptor.unwrap();
        add_en(
            connection,
            c"nop".as_ptr(),
            Box::into_raw(nop) as *mut WT_ENCRYPTOR,
            ptr::null(),
        )
    }
}
