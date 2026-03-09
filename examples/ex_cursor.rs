//! Example: Cursor Operations
//!
//! This example demonstrates cursor operations with WiredTiger using
//! the low-level FFI interface for key/value operations.
//!
//! Rust port of wiredtiger/examples/c/ex_cursor.c

use std::ffi::CString;
use std::fs;
use std::ptr;
use wtrs::{Connection, Result};

fn main() -> Result<()> {
    let home = "WT_HOME_CURSOR";

    // Clean up any previous run
    let _ = fs::remove_dir_all(home);
    fs::create_dir_all(home).expect("Failed to create database directory");

    println!("WiredTiger Cursor Example");
    println!("=========================\n");

    // Open a connection to the database
    let conn = Connection::open(home, Some("create,statistics=(fast)"))?;
    println!("Opened connection to '{}'", home);

    // Open a session
    let session = conn.open_session(None)?;
    println!("Opened session\n");

    // Create a simple string table
    println!("Creating table 'table:map' with key_format=S, value_format=S");
    session.create("table:map", Some("key_format=S,value_format=S"))?;

    // Open a cursor on the table
    let mut cursor = session.open_cursor("table:map", None)?;
    println!("Opened cursor on table:map");
    println!("  Key format: {:?}", cursor.key_format());
    println!("  Value format: {:?}", cursor.value_format());
    println!();

    // Insert data using the low-level FFI interface
    // For string format (S), we need to use the variadic set_key/set_value through FFI
    println!("Inserting data:");
    
    let keys = ["apple", "banana", "cherry", "foo"];
    let values = ["red", "yellow", "red", "bar"];

    for (key, value) in keys.iter().zip(values.iter()) {
        let key_cstr = CString::new(*key).unwrap();
        let value_cstr = CString::new(*value).unwrap();

        unsafe {
            let c = &*cursor.as_raw();
            // Call set_key with string format
            if let Some(set_key_fn) = c.set_key {
                set_key_fn(cursor.as_raw(), key_cstr.as_ptr());
            }
            // Call set_value with string format
            if let Some(set_value_fn) = c.set_value {
                set_value_fn(cursor.as_raw(), value_cstr.as_ptr());
            }
        }
        cursor.insert()?;
        println!("  Inserted: '{}' -> '{}'", key, value);
    }
    println!();

    // Reset cursor and scan forward
    cursor.reset()?;
    println!("Forward scan:");
    loop {
        match cursor.next() {
            Ok(()) => {
                // Get key and value using FFI
                let (key, value) = unsafe {
                    let c = &*cursor.as_raw();
                    let mut key_ptr: *const std::os::raw::c_char = ptr::null();
                    let mut value_ptr: *const std::os::raw::c_char = ptr::null();
                    
                    if let Some(get_key_fn) = c.get_key {
                        get_key_fn(cursor.as_raw(), &mut key_ptr);
                    }
                    if let Some(get_value_fn) = c.get_value {
                        get_value_fn(cursor.as_raw(), &mut value_ptr);
                    }
                    
                    let key = if key_ptr.is_null() {
                        "<null>".to_string()
                    } else {
                        std::ffi::CStr::from_ptr(key_ptr).to_string_lossy().into_owned()
                    };
                    let value = if value_ptr.is_null() {
                        "<null>".to_string()
                    } else {
                        std::ffi::CStr::from_ptr(value_ptr).to_string_lossy().into_owned()
                    };
                    (key, value)
                };
                println!("  key: '{}', value: '{}'", key, value);
            }
            Err(e) if e.is_not_found() => {
                println!("  (end of data)");
                break;
            }
            Err(e) => return Err(e),
        }
    }
    println!();

    // Search for a specific key
    println!("Searching for 'foo':");
    {
        let key_cstr = CString::new("foo").unwrap();
        unsafe {
            let c = &*cursor.as_raw();
            if let Some(set_key_fn) = c.set_key {
                set_key_fn(cursor.as_raw(), key_cstr.as_ptr());
            }
        }
        cursor.search()?;
        
        let value = unsafe {
            let c = &*cursor.as_raw();
            let mut value_ptr: *const std::os::raw::c_char = ptr::null();
            if let Some(get_value_fn) = c.get_value {
                get_value_fn(cursor.as_raw(), &mut value_ptr);
            }
            if value_ptr.is_null() {
                "<null>".to_string()
            } else {
                std::ffi::CStr::from_ptr(value_ptr).to_string_lossy().into_owned()
            }
        };
        println!("  Found: 'foo' -> '{}'", value);
    }
    println!();

    // Update 'foo'
    println!("Updating 'foo' to 'newbar':");
    {
        let key_cstr = CString::new("foo").unwrap();
        let value_cstr = CString::new("newbar").unwrap();
        unsafe {
            let c = &*cursor.as_raw();
            if let Some(set_key_fn) = c.set_key {
                set_key_fn(cursor.as_raw(), key_cstr.as_ptr());
            }
            if let Some(set_value_fn) = c.set_value {
                set_value_fn(cursor.as_raw(), value_cstr.as_ptr());
            }
        }
        cursor.update()?;
        println!("  Updated successfully");
    }
    println!();

    // Remove 'banana'
    println!("Removing 'banana':");
    {
        let key_cstr = CString::new("banana").unwrap();
        unsafe {
            let c = &*cursor.as_raw();
            if let Some(set_key_fn) = c.set_key {
                set_key_fn(cursor.as_raw(), key_cstr.as_ptr());
            }
        }
        cursor.remove()?;
        println!("  Removed successfully");
    }
    println!();

    // Final scan
    cursor.reset()?;
    println!("Final table state:");
    loop {
        match cursor.next() {
            Ok(()) => {
                let (key, value) = unsafe {
                    let c = &*cursor.as_raw();
                    let mut key_ptr: *const std::os::raw::c_char = ptr::null();
                    let mut value_ptr: *const std::os::raw::c_char = ptr::null();

                    if let Some(get_key_fn) = c.get_key {
                        get_key_fn(cursor.as_raw(), &mut key_ptr);
                    }
                    if let Some(get_value_fn) = c.get_value {
                        get_value_fn(cursor.as_raw(), &mut value_ptr);
                    }

                    let key = if key_ptr.is_null() {
                        "<null>".to_string()
                    } else {
                        std::ffi::CStr::from_ptr(key_ptr).to_string_lossy().into_owned()
                    };
                    let value = if value_ptr.is_null() {
                        "<null>".to_string()
                    } else {
                        std::ffi::CStr::from_ptr(value_ptr).to_string_lossy().into_owned()
                    };
                    (key, value)
                };
                println!("  key: '{}', value: '{}'", key, value);
            }
            Err(e) if e.is_not_found() => {
                println!("  (end of data)");
                break;
            }
            Err(e) => return Err(e),
        }
    }
    println!();

    // Close cursor
    cursor.close()?;
    println!("Cursor closed.");

    // Close connection
    drop(session);
    conn.close_with_config(None)?;
    println!("Connection closed.");

    // Clean up
    let _ = fs::remove_dir_all(home);

    println!("\nExample completed successfully!");
    Ok(())
}

