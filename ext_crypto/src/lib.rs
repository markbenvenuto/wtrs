use std::{ffi::CString, ptr};

use crate::bindings::{WT_CONFIG_ARG, WT_CONNECTION};

mod bindings;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[unsafe(no_mangle)]
pub extern "C" fn print_hello_from_rust() {
    println!("Hello from Rust");
}

#[unsafe(no_mangle)]
pub extern "C" fn wiredtiger_extension_init(
    _connection: *mut WT_CONNECTION,
    _config: *mut WT_CONFIG_ARG,
) -> ::std::os::raw::c_int {
    println!("Hello from Rust Extension");

    // let name = "nop";
    // unsafe {
    //     let add_en = (*connection).add_encryptor.unwrap();

    //     let name_cstr = CString::new(name).expect("TODO");

    //     // add_en(conn, "nop", nop_encryptor, 0);
    //     // let ret = add_en(connection, name_cstr.as_ptr(), 0, ptr::null());
    //     // if ret != 0 {
    //     //     panic!("Non zero error TODO")
    //     // }
    // }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
