//! Type-safe Rust wrapper around WiredTiger.
//!
//! This module provides safe Rust abstractions over the WiredTiger C API,
//! including `Connection`, `Session`, `Cursor`, `Item`, and `Modify` types.

use std::ffi::{CStr, CString};
use std::fmt;
use std::marker::PhantomData;
use std::path::Path;
use std::ptr;
use std::slice;

// Re-export the raw bindings
pub use wt_sys;

// Re-export common constants
pub use wt_sys::{WT_CACHE_FULL, WT_DUPLICATE_KEY, WT_NOTFOUND, WT_ROLLBACK};

/// WiredTiger error type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    code: i32,
    message: String,
}

impl Error {
    /// Create a new error from a WiredTiger error code
    pub fn from_code(code: i32) -> Self {
        let message = unsafe {
            let msg_ptr = wt_sys::wiredtiger_strerror(code);
            if msg_ptr.is_null() {
                format!("WiredTiger error code {}", code)
            } else {
                CStr::from_ptr(msg_ptr).to_string_lossy().into_owned()
            }
        };
        Error { code, message }
    }

    /// Get the error code
    pub fn code(&self) -> i32 {
        self.code
    }

    /// Check if this is a "not found" error
    pub fn is_not_found(&self) -> bool {
        self.code == wt_sys::WT_NOTFOUND
    }

    /// Check if this is a duplicate key error
    pub fn is_duplicate_key(&self) -> bool {
        self.code == wt_sys::WT_DUPLICATE_KEY
    }

    /// Check if this is a rollback error
    pub fn is_rollback(&self) -> bool {
        self.code == wt_sys::WT_ROLLBACK
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WiredTiger error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for Error {}

/// Result type for WiredTiger operations
pub type Result<T> = std::result::Result<T, Error>;

/// Check a WiredTiger return code and convert to Result
fn check_error(code: i32) -> Result<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(Error::from_code(code))
    }
}

/// Helper to convert optional config string to C string
fn config_to_cstring(config: Option<&str>) -> Result<Option<CString>> {
    match config {
        Some(c) => Ok(Some(CString::new(c).map_err(|_| Error {
            code: -1,
            message: "Config contains null byte".to_string(),
        })?)),
        None => Ok(None),
    }
}

/// Helper to get config pointer from optional CString
fn config_ptr(config: &Option<CString>) -> *const std::os::raw::c_char {
    config.as_ref().map(|c| c.as_ptr()).unwrap_or(ptr::null())
}

/// A raw data item for WiredTiger operations.
///
/// This is a safe wrapper around `WT_ITEM`.
#[derive(Debug, Clone)]
pub struct Item {
    data: Vec<u8>,
}

impl Item {
    /// Create a new empty item
    pub fn new() -> Self {
        Item { data: Vec::new() }
    }

    /// Create an item from bytes
    pub fn from_bytes(data: &[u8]) -> Self {
        Item {
            data: data.to_vec(),
        }
    }

    /// Create an item from a string
    pub fn from_str(s: &str) -> Self {
        Item {
            data: s.as_bytes().to_vec(),
        }
    }

    /// Get the data as a byte slice
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Try to get the data as a string
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.data).ok()
    }

    /// Get the length of the data
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the item is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Convert to raw WT_ITEM for FFI
    pub(crate) fn to_raw(&self) -> wt_sys::WT_ITEM {
        wt_sys::WT_ITEM {
            data: self.data.as_ptr() as *const std::ffi::c_void,
            size: self.data.len(),
            mem: ptr::null_mut(),
            memsize: 0,
            flags: 0,
        }
    }

    /// Create from raw WT_ITEM (copies data)
    ///
    /// # Safety
    /// The raw WT_ITEM must contain valid data pointer and size.
    pub(crate) unsafe fn from_raw(raw: &wt_sys::WT_ITEM) -> Self {
        if raw.data.is_null() || raw.size == 0 {
            Item::new()
        } else {
            let slice = unsafe { slice::from_raw_parts(raw.data as *const u8, raw.size) };
            Item::from_bytes(slice)
        }
    }
}

impl Default for Item {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&[u8]> for Item {
    fn from(data: &[u8]) -> Self {
        Item::from_bytes(data)
    }
}

impl From<&str> for Item {
    fn from(s: &str) -> Self {
        Item::from_str(s)
    }
}

impl From<Vec<u8>> for Item {
    fn from(data: Vec<u8>) -> Self {
        Item { data }
    }
}

impl From<String> for Item {
    fn from(s: String) -> Self {
        Item {
            data: s.into_bytes(),
        }
    }
}

/// A modification operation for use with `Cursor::modify`.
///
/// This is a safe wrapper around `WT_MODIFY`.
#[derive(Debug, Clone)]
pub struct Modify {
    /// New data to insert
    pub data: Item,
    /// Byte offset in the value where the modification starts
    pub offset: usize,
    /// Number of bytes to replace (0 for insert)
    pub size: usize,
}

impl Modify {
    /// Create a new modification that inserts data at an offset
    pub fn insert(data: Item, offset: usize) -> Self {
        Modify {
            data,
            offset,
            size: 0,
        }
    }

    /// Create a new modification that replaces bytes at an offset
    pub fn replace(data: Item, offset: usize, size: usize) -> Self {
        Modify { data, offset, size }
    }

    /// Convert to raw WT_MODIFY for FFI
    pub(crate) fn to_raw(&self) -> wt_sys::WT_MODIFY {
        wt_sys::WT_MODIFY {
            data: self.data.to_raw(),
            offset: self.offset,
            size: self.size,
        }
    }
}

/// Timestamp transaction type for `Session::timestamp_transaction_uint`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TimestampType {
    /// Commit timestamp
    Commit = wt_sys::WT_TS_TXN_TYPE_WT_TS_TXN_TYPE_COMMIT,
    /// Durable timestamp
    Durable = wt_sys::WT_TS_TXN_TYPE_WT_TS_TXN_TYPE_DURABLE,
    /// Prepare timestamp
    Prepare = wt_sys::WT_TS_TXN_TYPE_WT_TS_TXN_TYPE_PREPARE,
    /// Read timestamp
    Read = wt_sys::WT_TS_TXN_TYPE_WT_TS_TXN_TYPE_READ,
    /// Rollback timestamp
    Rollback = wt_sys::WT_TS_TXN_TYPE_WT_TS_TXN_TYPE_ROLLBACK,
}

// ============================================================================
// Connection
// ============================================================================

/// A WiredTiger database connection.
///
/// This is a safe wrapper around `WT_CONNECTION`. The connection will be
/// automatically closed when dropped.
pub struct Connection {
    inner: *mut wt_sys::WT_CONNECTION,
}

// Connection can be sent between threads
unsafe impl Send for Connection {}
// Connection can be shared between threads (WiredTiger handles its own locking)
unsafe impl Sync for Connection {}

impl Connection {
    /// Open a connection to a WiredTiger database.
    ///
    /// # Arguments
    /// * `home` - Path to the database home directory
    /// * `config` - Optional configuration string
    ///
    /// # Example
    /// ```no_run
    /// use wtrs::Connection;
    ///
    /// let conn = Connection::open("/path/to/db", Some("create")).unwrap();
    /// ```
    pub fn open<P: AsRef<Path>>(home: P, config: Option<&str>) -> Result<Self> {
        let home_cstr = CString::new(home.as_ref().to_str().ok_or_else(|| Error {
            code: -1,
            message: "Invalid UTF-8 in path".to_string(),
        })?)
        .map_err(|_| Error {
            code: -1,
            message: "Path contains null byte".to_string(),
        })?;

        let config_cstr = config_to_cstring(config)?;
        let mut connection: *mut wt_sys::WT_CONNECTION = ptr::null_mut();

        let ret = unsafe {
            wt_sys::wiredtiger_open(
                home_cstr.as_ptr(),
                ptr::null_mut(), // event_handler - use default
                config_ptr(&config_cstr),
                &mut connection,
            )
        };

        check_error(ret)?;

        if connection.is_null() {
            return Err(Error {
                code: -1,
                message: "wiredtiger_open returned null connection".to_string(),
            });
        }

        Ok(Connection { inner: connection })
    }

    /// Get the home directory of this connection.
    pub fn get_home(&self) -> Option<&str> {
        unsafe {
            let conn = &*self.inner;
            let get_home_fn = conn.get_home?;
            let home_ptr = get_home_fn(self.inner);
            if home_ptr.is_null() {
                None
            } else {
                CStr::from_ptr(home_ptr).to_str().ok()
            }
        }
    }

    /// Check if this database was newly created.
    pub fn is_new(&self) -> bool {
        unsafe {
            let conn = &*self.inner;
            if let Some(is_new_fn) = conn.is_new {
                is_new_fn(self.inner) != 0
            } else {
                false
            }
        }
    }

    /// Reconfigure the connection.
    pub fn reconfigure(&self, config: &str) -> Result<()> {
        let config_cstr = CString::new(config).map_err(|_| Error {
            code: -1,
            message: "Config contains null byte".to_string(),
        })?;

        let ret = unsafe {
            let conn = &*self.inner;
            match conn.reconfigure {
                Some(f) => f(self.inner, config_cstr.as_ptr()),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "reconfigure function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Output debug information for various subsystems.
    pub fn debug_info(&self, config: &str) -> Result<()> {
        let config_cstr = CString::new(config).map_err(|_| Error {
            code: -1,
            message: "Config contains null byte".to_string(),
        })?;
        let ret = unsafe {
            let conn = &*self.inner;
            match conn.debug_info {
                Some(f) => f(self.inner, config_cstr.as_ptr()),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "debug_info function not available".to_string(),
                    });
                }
            }
        };
        check_error(ret)
    }
    /// Query the global transaction timestamp state.
    pub fn query_timestamp(&self, config: Option<&str>) -> Result<String> {
        let config_cstr = config_to_cstring(config)?;
        let mut buf = [0i8; 17]; // Hex-encoded 8-byte timestamp + null
        let ret = unsafe {
            let conn = &*self.inner;
            match conn.query_timestamp {
                Some(f) => f(self.inner, buf.as_mut_ptr(), config_ptr(&config_cstr)),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "query_timestamp function not available".to_string(),
                    });
                }
            }
        };
        check_error(ret)?;
        let cstr = unsafe { CStr::from_ptr(buf.as_ptr()) };
        Ok(cstr.to_string_lossy().into_owned())
    }
    /// Set a global transaction timestamp.
    pub fn set_timestamp(&self, config: &str) -> Result<()> {
        let config_cstr = CString::new(config).map_err(|_| Error {
            code: -1,
            message: "Config contains null byte".to_string(),
        })?;
        let ret = unsafe {
            let conn = &*self.inner;
            match conn.set_timestamp {
                Some(f) => f(self.inner, config_cstr.as_ptr()),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "set_timestamp function not available".to_string(),
                    });
                }
            }
        };
        check_error(ret)
    }
    /// Rollback tables to an earlier point in time.
    pub fn rollback_to_stable(&self, config: Option<&str>) -> Result<()> {
        let config_cstr = config_to_cstring(config)?;
        let ret = unsafe {
            let conn = &*self.inner;
            match conn.rollback_to_stable {
                Some(f) => f(self.inner, config_ptr(&config_cstr)),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "rollback_to_stable function not available".to_string(),
                    });
                }
            }
        };
        check_error(ret)
    }
    /// Load an extension.
    pub fn load_extension(&self, path: &str, config: Option<&str>) -> Result<()> {
        let path_cstr = CString::new(path).map_err(|_| Error {
            code: -1,
            message: "Path contains null byte".to_string(),
        })?;
        let config_cstr = config_to_cstring(config)?;
        let ret = unsafe {
            let conn = &*self.inner;
            match conn.load_extension {
                Some(f) => f(self.inner, path_cstr.as_ptr(), config_ptr(&config_cstr)),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "load_extension function not available".to_string(),
                    });
                }
            }
        };
        check_error(ret)
    }
    /// Open a new session on this connection.
    pub fn open_session(&self, config: Option<&str>) -> Result<Session<'_>> {
        let config_cstr = config_to_cstring(config)?;
        let mut session: *mut wt_sys::WT_SESSION = ptr::null_mut();
        let ret = unsafe {
            let conn = &*self.inner;
            match conn.open_session {
                Some(f) => f(
                    self.inner,
                    ptr::null_mut(), // event_handler
                    config_ptr(&config_cstr),
                    &mut session,
                ),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "open_session function not available".to_string(),
                    });
                }
            }
        };
        check_error(ret)?;
        if session.is_null() {
            return Err(Error {
                code: -1,
                message: "open_session returned null session".to_string(),
            });
        }
        Ok(Session {
            inner: session,
            _marker: PhantomData,
        })
    }
    /// Close the connection with optional configuration.
    pub fn close_with_config(self, config: Option<&str>) -> Result<()> {
        let config_cstr = config_to_cstring(config)?;
        let ret = unsafe {
            let conn = &*self.inner;
            match conn.close {
                Some(f) => f(self.inner, config_ptr(&config_cstr)),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "close function not available".to_string(),
                    });
                }
            }
        };
        // Prevent Drop from running since we already closed
        std::mem::forget(self);
        check_error(ret)
    }
    /// Get the raw WT_CONNECTION pointer.
    ///
    /// # Safety
    /// The caller must ensure the pointer is not used after the Connection is dropped.
    pub unsafe fn as_raw(&self) -> *mut wt_sys::WT_CONNECTION {
        self.inner
    }

    /// Add a page log service implementation.
    ///
    /// # Safety
    /// The `page_log` pointer must point to a valid WT_PAGE_LOG structure that remains
    /// valid for the lifetime of the connection.
    pub unsafe fn add_page_log(
        &self,
        name: &str,
        page_log: *mut wt_sys::WT_PAGE_LOG,
        config: Option<&str>,
    ) -> Result<()> {
        let name_cstr = CString::new(name).map_err(|_| Error {
            code: -1,
            message: "Name contains null byte".to_string(),
        })?;
        let config_cstr = config_to_cstring(config)?;

        let ret = unsafe {
            let conn = &*self.inner;
            match conn.add_page_log {
                Some(f) => f(
                    self.inner,
                    name_cstr.as_ptr(),
                    page_log,
                    config_ptr(&config_cstr),
                ),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "add_page_log function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Get a page log service implementation by name.
    ///
    /// Look up a page log service by name and return it. The returned page log service
    /// must be released by calling WT_PAGE_LOG::terminate.
    pub fn get_page_log(&self, name: &str) -> Result<PageLog> {
        let name_cstr = CString::new(name).map_err(|_| Error {
            code: -1,
            message: "Name contains null byte".to_string(),
        })?;

        let mut page_log: *mut wt_sys::WT_PAGE_LOG = ptr::null_mut();

        let ret = unsafe {
            let conn = &*self.inner;
            match conn.get_page_log {
                Some(f) => f(self.inner, name_cstr.as_ptr(), &mut page_log),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "get_page_log function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)?;

        if page_log.is_null() {
            return Err(Error {
                code: -1,
                message: "get_page_log returned null".to_string(),
            });
        }

        Ok(PageLog { inner: page_log })
    }

    /// Configure a key provider system.
    ///
    /// This method can only be called from an early loaded extension module.
    ///
    /// # Safety
    /// The `key_provider` pointer must point to a valid WT_KEY_PROVIDER structure that
    /// remains valid for the lifetime of the connection.
    pub unsafe fn set_key_provider(
        &self,
        key_provider: *mut wt_sys::WT_KEY_PROVIDER,
        config: Option<&str>,
    ) -> Result<()> {
        let config_cstr = config_to_cstring(config)?;

        let ret = unsafe {
            let conn = &*self.inner;
            match conn.set_key_provider {
                Some(f) => f(self.inner, key_provider, config_ptr(&config_cstr)),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "set_key_provider function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }
}
impl Drop for Connection {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                let conn = &*self.inner;
                if let Some(close_fn) = conn.close {
                    let _ = close_fn(self.inner, ptr::null());
                }
            }
        }
    }
}

// ============================================================================
// PageLog
// ============================================================================

/// A WiredTiger page log service.
///
/// This is a safe wrapper around `WT_PAGE_LOG`. Page log services are used
/// for custom page logging implementations.
pub struct PageLog {
    inner: *mut wt_sys::WT_PAGE_LOG,
}

// PageLog can be sent between threads
unsafe impl Send for PageLog {}
unsafe impl Sync for PageLog {}

impl PageLog {
    /// Add a reference to the page log service.
    pub fn add_reference(&self) -> Result<()> {
        let ret = unsafe {
            let pl = &*self.inner;
            match pl.pl_add_reference {
                Some(f) => f(self.inner),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "pl_add_reference function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Abandon an incomplete checkpoint.
    pub fn abandon_checkpoint(&self, session: &Session) -> Result<()> {
        let ret = unsafe {
            let pl = &*self.inner;
            match pl.pl_abandon_checkpoint {
                Some(f) => f(self.inner, session.inner),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "pl_abandon_checkpoint function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Begin checkpointing using the given checkpoint_id.
    pub fn begin_checkpoint(&self, session: &Session, checkpoint_id: u64) -> Result<()> {
        let ret = unsafe {
            let pl = &*self.inner;
            match pl.pl_begin_checkpoint {
                Some(f) => f(self.inner, session.inner, checkpoint_id),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "pl_begin_checkpoint function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Complete checkpointing using the given checkpoint_id.
    ///
    /// Returns the LSN of the checkpoint completion record.
    pub fn complete_checkpoint(
        &self,
        session: &Session,
        checkpoint_id: u64,
        checkpoint_timestamp: u64,
        checkpoint_oldest_timestamp: u64,
    ) -> Result<u64> {
        let mut args = wt_sys::WT_PAGE_LOG_COMPLETE_CHECKPOINT_ARGS {
            checkpoint_id,
            checkpoint_timestamp,
            checkpoint_metadata: ptr::null(),
            checkpoint_oldest_timestamp,
            lsn: 0,
        };

        let ret = unsafe {
            let pl = &*self.inner;
            match pl.pl_complete_checkpoint {
                Some(f) => f(self.inner, session.inner, &mut args),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "pl_complete_checkpoint function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)?;
        Ok(args.lsn)
    }

    /// Get the most recent completed checkpoint number.
    pub fn get_complete_checkpoint(&self, session: &Session) -> Result<u64> {
        let mut checkpoint_id: u64 = 0;

        let ret = unsafe {
            let pl = &*self.inner;
            match pl.pl_get_complete_checkpoint {
                Some(f) => f(self.inner, session.inner, &mut checkpoint_id),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "pl_get_complete_checkpoint function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)?;
        Ok(checkpoint_id)
    }

    /// Get the most recently opened checkpoint number.
    pub fn get_open_checkpoint(&self, session: &Session) -> Result<u64> {
        let mut checkpoint_id: u64 = 0;

        let ret = unsafe {
            let pl = &*self.inner;
            match pl.pl_get_open_checkpoint {
                Some(f) => f(self.inner, session.inner, &mut checkpoint_id),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "pl_get_open_checkpoint function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)?;
        Ok(checkpoint_id)
    }

    /// Get the last written page LSN.
    pub fn get_last_lsn(&self, session: &Session) -> Result<u64> {
        let mut lsn: u64 = 0;

        let ret = unsafe {
            let pl = &*self.inner;
            match pl.pl_get_last_lsn {
                Some(f) => f(self.inner, session.inner, &mut lsn),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "pl_get_last_lsn function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)?;
        Ok(lsn)
    }

    /// Set the last materialized LSN.
    pub fn set_last_materialized_lsn(&self, session: &Session, lsn: u64) -> Result<()> {
        let ret = unsafe {
            let pl = &*self.inner;
            match pl.pl_set_last_materialized_lsn {
                Some(f) => f(self.inner, session.inner, lsn),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "pl_set_last_materialized_lsn function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Open a handle for further operations on a table.
    pub fn open_handle(&self, session: &Session, table_id: u64) -> Result<PageLogHandle> {
        let mut handle: *mut wt_sys::WT_PAGE_LOG_HANDLE = ptr::null_mut();

        let ret = unsafe {
            let pl = &*self.inner;
            match pl.pl_open_handle {
                Some(f) => f(self.inner, session.inner, table_id, &mut handle),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "pl_open_handle function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)?;

        if handle.is_null() {
            return Err(Error {
                code: -1,
                message: "pl_open_handle returned null".to_string(),
            });
        }

        Ok(PageLogHandle { inner: handle })
    }

    /// Discard a table from the paging/logging service.
    pub fn trim_table(&self, session: &Session, table_id: u64, start_lsn: u64) -> Result<u64> {
        let mut lsn: u64 = 0;

        let ret = unsafe {
            let pl = &*self.inner;
            match pl.pl_trim_table {
                Some(f) => f(self.inner, session.inner, table_id, start_lsn, &mut lsn),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "pl_trim_table function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)?;
        Ok(lsn)
    }

    /// Terminate and release the page log service.
    pub fn terminate(self, session: &Session) -> Result<()> {
        let ret = unsafe {
            let pl = &*self.inner;
            match pl.terminate {
                Some(f) => f(self.inner, session.inner),
                None => {
                    // No terminate function, just forget
                    std::mem::forget(self);
                    return Ok(());
                }
            }
        };

        std::mem::forget(self);
        check_error(ret)
    }

    /// Get the raw WT_PAGE_LOG pointer.
    pub unsafe fn as_raw(&self) -> *mut wt_sys::WT_PAGE_LOG {
        self.inner
    }
}

// ============================================================================
// PageLogHandle
// ============================================================================

/// A handle for page log operations on a specific table.
///
/// This is a safe wrapper around `WT_PAGE_LOG_HANDLE`.
pub struct PageLogHandle {
    inner: *mut wt_sys::WT_PAGE_LOG_HANDLE,
}

unsafe impl Send for PageLogHandle {}

impl PageLogHandle {
    /// Get the raw WT_PAGE_LOG_HANDLE pointer.
    pub unsafe fn as_raw(&self) -> *mut wt_sys::WT_PAGE_LOG_HANDLE {
        self.inner
    }

    /// Close the page log handle.
    pub fn close(self, session: &Session) -> Result<()> {
        let ret = unsafe {
            let handle = &*self.inner;
            match handle.plh_close {
                Some(f) => f(self.inner, session.inner),
                None => {
                    std::mem::forget(self);
                    return Ok(());
                }
            }
        };

        std::mem::forget(self);
        check_error(ret)
    }
}

// ============================================================================
// Session
// ============================================================================
/// A WiredTiger session.
///
/// Sessions are used to perform operations on the database. Each session
/// can have at most one active transaction at a time.
pub struct Session<'conn> {
    inner: *mut wt_sys::WT_SESSION,
    _marker: PhantomData<&'conn Connection>,
}
// Session can be sent between threads
unsafe impl Send for Session<'_> {}
impl<'conn> Session<'conn> {
    /// Get the connection associated with this session.
    pub fn connection_ptr(&self) -> *mut wt_sys::WT_CONNECTION {
        unsafe { (*self.inner).connection }
    }
    /// Reconfigure the session.
    pub fn reconfigure(&self, config: &str) -> Result<()> {
        let config_cstr = CString::new(config).map_err(|_| Error {
            code: -1,
            message: "Config contains null byte".to_string(),
        })?;
        let ret = unsafe {
            let session = &*self.inner;
            match session.reconfigure {
                Some(f) => f(self.inner, config_cstr.as_ptr()),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "reconfigure function not available".to_string(),
                    });
                }
            }
        };
        check_error(ret)
    }
    /// Open a cursor on a data source.
    pub fn open_cursor(&self, uri: &str, config: Option<&str>) -> Result<Cursor<'_>> {
        let uri_cstr = CString::new(uri).map_err(|_| Error {
            code: -1,
            message: "URI contains null byte".to_string(),
        })?;
        let config_cstr = config_to_cstring(config)?;
        let mut cursor: *mut wt_sys::WT_CURSOR = ptr::null_mut();
        let ret = unsafe {
            let session = &*self.inner;
            match session.open_cursor {
                Some(f) => f(
                    self.inner,
                    uri_cstr.as_ptr(),
                    ptr::null_mut(), // to_dup
                    config_ptr(&config_cstr),
                    &mut cursor,
                ),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "open_cursor function not available".to_string(),
                    });
                }
            }
        };
        check_error(ret)?;
        if cursor.is_null() {
            return Err(Error {
                code: -1,
                message: "open_cursor returned null cursor".to_string(),
            });
        }
        Ok(Cursor {
            inner: cursor,
            _marker: PhantomData,
        })
    }

    /// Create a table, column group, index, or file.
    pub fn create(&self, name: &str, config: Option<&str>) -> Result<()> {
        let name_cstr = CString::new(name).map_err(|_| Error {
            code: -1,
            message: "Name contains null byte".to_string(),
        })?;
        let config_cstr = config_to_cstring(config)?;

        let ret = unsafe {
            let session = &*self.inner;
            match session.create {
                Some(f) => f(self.inner, name_cstr.as_ptr(), config_ptr(&config_cstr)),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "create function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Drop (delete) a table.
    pub fn drop_table(&self, name: &str, config: Option<&str>) -> Result<()> {
        let name_cstr = CString::new(name).map_err(|_| Error {
            code: -1,
            message: "Name contains null byte".to_string(),
        })?;
        let config_cstr = config_to_cstring(config)?;

        let ret = unsafe {
            let session = &*self.inner;
            match session.drop {
                Some(f) => f(self.inner, name_cstr.as_ptr(), config_ptr(&config_cstr)),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "drop function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Alter a table.
    pub fn alter(&self, name: &str, config: Option<&str>) -> Result<()> {
        let name_cstr = CString::new(name).map_err(|_| Error {
            code: -1,
            message: "Name contains null byte".to_string(),
        })?;
        let config_cstr = config_to_cstring(config)?;

        let ret = unsafe {
            let session = &*self.inner;
            match session.alter {
                Some(f) => f(self.inner, name_cstr.as_ptr(), config_ptr(&config_cstr)),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "alter function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Compact a live row- or column-store btree.
    pub fn compact(&self, name: &str, config: Option<&str>) -> Result<()> {
        let name_cstr = CString::new(name).map_err(|_| Error {
            code: -1,
            message: "Name contains null byte".to_string(),
        })?;
        let config_cstr = config_to_cstring(config)?;

        let ret = unsafe {
            let session = &*self.inner;
            match session.compact {
                Some(f) => f(self.inner, name_cstr.as_ptr(), config_ptr(&config_cstr)),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "compact function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Salvage a table (rebuild from corrupted state).
    pub fn salvage(&self, name: &str, config: Option<&str>) -> Result<()> {
        let name_cstr = CString::new(name).map_err(|_| Error {
            code: -1,
            message: "Name contains null byte".to_string(),
        })?;
        let config_cstr = config_to_cstring(config)?;

        let ret = unsafe {
            let session = &*self.inner;
            match session.salvage {
                Some(f) => f(self.inner, name_cstr.as_ptr(), config_ptr(&config_cstr)),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "salvage function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Verify a table.
    pub fn verify(&self, name: &str, config: Option<&str>) -> Result<()> {
        let name_cstr = CString::new(name).map_err(|_| Error {
            code: -1,
            message: "Name contains null byte".to_string(),
        })?;
        let config_cstr = config_to_cstring(config)?;

        let ret = unsafe {
            let session = &*self.inner;
            match session.verify {
                Some(f) => f(self.inner, name_cstr.as_ptr(), config_ptr(&config_cstr)),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "verify function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Truncate a file, table, or cursor range.
    pub fn truncate(
        &self,
        name: Option<&str>,
        start: Option<&Cursor>,
        stop: Option<&Cursor>,
        config: Option<&str>,
    ) -> Result<()> {
        let name_cstr = match name {
            Some(n) => Some(CString::new(n).map_err(|_| Error {
                code: -1,
                message: "Name contains null byte".to_string(),
            })?),
            None => None,
        };
        let config_cstr = config_to_cstring(config)?;

        let ret = unsafe {
            let session = &*self.inner;
            match session.truncate {
                Some(f) => f(
                    self.inner,
                    name_cstr
                        .as_ref()
                        .map(|c| c.as_ptr())
                        .unwrap_or(ptr::null()),
                    start.map(|c| c.inner).unwrap_or(ptr::null_mut()),
                    stop.map(|c| c.inner).unwrap_or(ptr::null_mut()),
                    config_ptr(&config_cstr),
                ),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "truncate function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Begin a transaction.
    pub fn begin_transaction(&self, config: Option<&str>) -> Result<()> {
        let config_cstr = config_to_cstring(config)?;

        let ret = unsafe {
            let session = &*self.inner;
            match session.begin_transaction {
                Some(f) => f(self.inner, config_ptr(&config_cstr)),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "begin_transaction function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Commit the current transaction.
    pub fn commit_transaction(&self, config: Option<&str>) -> Result<()> {
        let config_cstr = config_to_cstring(config)?;

        let ret = unsafe {
            let session = &*self.inner;
            match session.commit_transaction {
                Some(f) => f(self.inner, config_ptr(&config_cstr)),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "commit_transaction function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Prepare the current transaction.
    pub fn prepare_transaction(&self, config: Option<&str>) -> Result<()> {
        let config_cstr = config_to_cstring(config)?;

        let ret = unsafe {
            let session = &*self.inner;
            match session.prepare_transaction {
                Some(f) => f(self.inner, config_ptr(&config_cstr)),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "prepare_transaction function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Rollback the current transaction.
    pub fn rollback_transaction(&self, config: Option<&str>) -> Result<()> {
        let config_cstr = config_to_cstring(config)?;

        let ret = unsafe {
            let session = &*self.inner;
            match session.rollback_transaction {
                Some(f) => f(self.inner, config_ptr(&config_cstr)),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "rollback_transaction function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Query the session's transaction timestamp state.
    pub fn query_timestamp(&self, config: Option<&str>) -> Result<String> {
        let config_cstr = config_to_cstring(config)?;
        let mut buf = [0i8; 17];

        let ret = unsafe {
            let session = &*self.inner;
            match session.query_timestamp {
                Some(f) => f(self.inner, buf.as_mut_ptr(), config_ptr(&config_cstr)),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "query_timestamp function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)?;
        let cstr = unsafe { CStr::from_ptr(buf.as_ptr()) };
        Ok(cstr.to_string_lossy().into_owned())
    }

    /// Set a timestamp on a transaction.
    pub fn timestamp_transaction(&self, config: &str) -> Result<()> {
        let config_cstr = CString::new(config).map_err(|_| Error {
            code: -1,
            message: "Config contains null byte".to_string(),
        })?;

        let ret = unsafe {
            let session = &*self.inner;
            match session.timestamp_transaction {
                Some(f) => f(self.inner, config_cstr.as_ptr()),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "timestamp_transaction function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Set a timestamp on a transaction numerically.
    pub fn timestamp_transaction_uint(&self, which: TimestampType, ts: u64) -> Result<()> {
        let ret = unsafe {
            let session = &*self.inner;
            match session.timestamp_transaction_uint {
                Some(f) => f(self.inner, which as u32, ts),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "timestamp_transaction_uint function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Write a checkpoint.
    pub fn checkpoint(&self, config: Option<&str>) -> Result<()> {
        let config_cstr = config_to_cstring(config)?;

        let ret = unsafe {
            let session = &*self.inner;
            match session.checkpoint {
                Some(f) => f(self.inner, config_ptr(&config_cstr)),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "checkpoint function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Reset the snapshot used for database visibility.
    pub fn reset_snapshot(&self) -> Result<()> {
        let ret = unsafe {
            let session = &*self.inner;
            match session.reset_snapshot {
                Some(f) => f(self.inner),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "reset_snapshot function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Reset the session handle.
    pub fn reset(&self) -> Result<()> {
        let ret = unsafe {
            let session = &*self.inner;
            match session.reset {
                Some(f) => f(self.inner),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "reset function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Flush the log.
    pub fn log_flush(&self, config: Option<&str>) -> Result<()> {
        let config_cstr = config_to_cstring(config)?;

        let ret = unsafe {
            let session = &*self.inner;
            match session.log_flush {
                Some(f) => f(self.inner, config_ptr(&config_cstr)),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "log_flush function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Return the transaction ID range pinned by the session.
    pub fn transaction_pinned_range(&self) -> Result<u64> {
        let mut range: u64 = 0;

        let ret = unsafe {
            let session = &*self.inner;
            match session.transaction_pinned_range {
                Some(f) => f(self.inner, &mut range),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "transaction_pinned_range function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)?;
        Ok(range)
    }

    /// Close the session with optional configuration.
    pub fn close_with_config(self, config: Option<&str>) -> Result<()> {
        let config_cstr = config_to_cstring(config)?;

        let ret = unsafe {
            let session = &*self.inner;
            match session.close {
                Some(f) => f(self.inner, config_ptr(&config_cstr)),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "close function not available".to_string(),
                    });
                }
            }
        };

        std::mem::forget(self);
        check_error(ret)
    }

    /// Get the raw WT_SESSION pointer.
    pub unsafe fn as_raw(&self) -> *mut wt_sys::WT_SESSION {
        self.inner
    }
}

impl Drop for Session<'_> {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                let session = &*self.inner;
                if let Some(close_fn) = session.close {
                    let _ = close_fn(self.inner, ptr::null());
                }
            }
        }
    }
}

// ============================================================================
// Cursor
// ============================================================================

/// A WiredTiger cursor.
///
/// Cursors are used to search, iterate, and modify data in the database.
pub struct Cursor<'session> {
    inner: *mut wt_sys::WT_CURSOR,
    _marker: PhantomData<&'session Session<'session>>,
}

// Cursor can be sent between threads
unsafe impl Send for Cursor<'_> {}

impl<'session> Cursor<'session> {
    /// Get the URI of the data source for this cursor.
    pub fn uri(&self) -> Option<&str> {
        unsafe {
            let cursor = &*self.inner;
            if cursor.uri.is_null() {
                None
            } else {
                CStr::from_ptr(cursor.uri).to_str().ok()
            }
        }
    }

    /// Get the key format for this cursor.
    pub fn key_format(&self) -> Option<&str> {
        unsafe {
            let cursor = &*self.inner;
            if cursor.key_format.is_null() {
                None
            } else {
                CStr::from_ptr(cursor.key_format).to_str().ok()
            }
        }
    }

    /// Get the value format for this cursor.
    pub fn value_format(&self) -> Option<&str> {
        unsafe {
            let cursor = &*self.inner;
            if cursor.value_format.is_null() {
                None
            } else {
                CStr::from_ptr(cursor.value_format).to_str().ok()
            }
        }
    }

    /// Set the key for the next operation (for string keys).
    pub fn set_key_str(&mut self, key: &str) -> Result<()> {
        let key_cstr = CString::new(key).map_err(|_| Error {
            code: -1,
            message: "Key contains null byte".to_string(),
        })?;

        unsafe {
            let cursor = &*self.inner;
            if let Some(set_key_fn) = cursor.set_key {
                set_key_fn(self.inner, key_cstr.as_ptr());
                Ok(())
            } else {
                Err(Error {
                    code: -1,
                    message: "set_key function not available".to_string(),
                })
            }
        }
    }

    /// Set the key for the next operation (for raw bytes/u format).
    ///
    /// This directly sets the cursor's internal key field for use with
    /// `key_format=u` tables.
    pub fn set_key_item(&mut self, key: &Item) {
        let raw = key.to_raw();
        unsafe {
            let cursor = &*self.inner;
            if let Some(set_key_fn) = cursor.set_key {
                set_key_fn(self.inner, &raw as *const wt_sys::WT_ITEM);
            }
        }
    }

    /// Set the value for the next operation (for string values).
    pub fn set_value_str(&mut self, value: &str) -> Result<()> {
        let value_cstr = CString::new(value).map_err(|_| Error {
            code: -1,
            message: "Value contains null byte".to_string(),
        })?;

        unsafe {
            let cursor = &*self.inner;
            if let Some(set_value_fn) = cursor.set_value {
                set_value_fn(self.inner, value_cstr.as_ptr());
                Ok(())
            } else {
                Err(Error {
                    code: -1,
                    message: "set_value function not available".to_string(),
                })
            }
        }
    }

    /// Set the value for the next operation (for raw bytes/u format).
    ///
    /// This directly sets the cursor's internal value field for use with
    /// `value_format=u` tables.
    pub fn set_value_item(&mut self, value: &Item) {
        let raw = value.to_raw();
        unsafe {
            let cursor = &*self.inner;
            if let Some(set_value_fn) = cursor.set_value {
                set_value_fn(self.inner, &raw as *const wt_sys::WT_ITEM);
            }
        }
    }

    /// Get the key and value as raw Items.
    pub fn get_raw_key_value(&self) -> Result<(Item, Item)> {
        let mut key_item = wt_sys::WT_ITEM {
            data: ptr::null(),
            size: 0,
            mem: ptr::null_mut(),
            memsize: 0,
            flags: 0,
        };
        let mut value_item = wt_sys::WT_ITEM {
            data: ptr::null(),
            size: 0,
            mem: ptr::null_mut(),
            memsize: 0,
            flags: 0,
        };

        let ret = unsafe {
            let cursor = &*self.inner;
            match cursor.get_raw_key_value {
                Some(f) => f(self.inner, &mut key_item, &mut value_item),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "get_raw_key_value function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)?;

        let key = unsafe { Item::from_raw(&key_item) };
        let value = unsafe { Item::from_raw(&value_item) };
        Ok((key, value))
    }

    /// Get the key as a string (for cursors with key_format=S).
    pub fn get_key_str(&self) -> Result<String> {
        let mut key_ptr: *const std::os::raw::c_char = ptr::null();

        let ret = unsafe {
            let cursor = &*self.inner;
            match cursor.get_key {
                Some(f) => f(self.inner, &mut key_ptr),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "get_key function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)?;

        if key_ptr.is_null() {
            return Err(Error {
                code: -1,
                message: "get_key returned null".to_string(),
            });
        }

        let key = unsafe { CStr::from_ptr(key_ptr) };
        Ok(key.to_string_lossy().into_owned())
    }

    /// Get the value as a string (for cursors with value_format=S).
    pub fn get_value_str(&self) -> Result<String> {
        let mut value_ptr: *const std::os::raw::c_char = ptr::null();

        let ret = unsafe {
            let cursor = &*self.inner;
            match cursor.get_value {
                Some(f) => f(self.inner, &mut value_ptr),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "get_value function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)?;

        if value_ptr.is_null() {
            return Err(Error {
                code: -1,
                message: "get_value returned null".to_string(),
            });
        }

        let value = unsafe { CStr::from_ptr(value_ptr) };
        Ok(value.to_string_lossy().into_owned())
    }

    /// Return the next record.
    pub fn next(&mut self) -> Result<()> {
        let ret = unsafe {
            let cursor = &*self.inner;
            match cursor.next {
                Some(f) => f(self.inner),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "next function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Return the previous record.
    pub fn prev(&mut self) -> Result<()> {
        let ret = unsafe {
            let cursor = &*self.inner;
            match cursor.prev {
                Some(f) => f(self.inner),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "prev function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Reset the cursor (release resources, invalidate position).
    pub fn reset(&mut self) -> Result<()> {
        let ret = unsafe {
            let cursor = &*self.inner;
            match cursor.reset {
                Some(f) => f(self.inner),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "reset function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Search for an exact match.
    pub fn search(&mut self) -> Result<()> {
        let ret = unsafe {
            let cursor = &*self.inner;
            match cursor.search {
                Some(f) => f(self.inner),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "search function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Search for an exact or adjacent match.
    /// Returns: < 0 if smaller key returned, 0 if exact, > 0 if larger key returned.
    pub fn search_near(&mut self) -> Result<i32> {
        let mut exact: i32 = 0;

        let ret = unsafe {
            let cursor = &*self.inner;
            match cursor.search_near {
                Some(f) => f(self.inner, &mut exact),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "search_near function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)?;
        Ok(exact)
    }

    /// Insert a record (and optionally update if overwrite=true).
    pub fn insert(&mut self) -> Result<()> {
        let ret = unsafe {
            let cursor = &*self.inner;
            match cursor.insert {
                Some(f) => f(self.inner),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "insert function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Update an existing record.
    pub fn update(&mut self) -> Result<()> {
        let ret = unsafe {
            let cursor = &*self.inner;
            match cursor.update {
                Some(f) => f(self.inner),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "update function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Remove a record.
    pub fn remove(&mut self) -> Result<()> {
        let ret = unsafe {
            let cursor = &*self.inner;
            match cursor.remove {
                Some(f) => f(self.inner),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "remove function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Reserve an existing record.
    pub fn reserve(&mut self) -> Result<()> {
        let ret = unsafe {
            let cursor = &*self.inner;
            match cursor.reserve {
                Some(f) => f(self.inner),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "reserve function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Modify an existing record with a list of modifications.
    pub fn modify(&mut self, modifications: &[Modify]) -> Result<()> {
        let raw_mods: Vec<wt_sys::WT_MODIFY> = modifications.iter().map(|m| m.to_raw()).collect();

        let ret = unsafe {
            let cursor = &*self.inner;
            match cursor.modify {
                Some(f) => f(
                    self.inner,
                    raw_mods.as_ptr() as *mut _,
                    raw_mods.len() as i32,
                ),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "modify function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Compare two cursors.
    /// Returns: < 0 if this cursor is before other, 0 if equal, > 0 if after.
    pub fn compare(&self, other: &Cursor) -> Result<i32> {
        let mut cmp: i32 = 0;

        let ret = unsafe {
            let cursor = &*self.inner;
            match cursor.compare {
                Some(f) => f(self.inner, other.inner, &mut cmp),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "compare function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)?;
        Ok(cmp)
    }

    /// Check if two cursors point to the same key.
    pub fn equals(&self, other: &Cursor) -> Result<bool> {
        let mut equal: i32 = 0;

        let ret = unsafe {
            let cursor = &*self.inner;
            match cursor.equals {
                Some(f) => f(self.inner, other.inner, &mut equal),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "equals function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)?;
        Ok(equal != 0)
    }

    /// Get the table's largest key.
    pub fn largest_key(&mut self) -> Result<()> {
        let ret = unsafe {
            let cursor = &*self.inner;
            match cursor.largest_key {
                Some(f) => f(self.inner),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "largest_key function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Reconfigure the cursor.
    pub fn reconfigure(&mut self, config: &str) -> Result<()> {
        let config_cstr = CString::new(config).map_err(|_| Error {
            code: -1,
            message: "Config contains null byte".to_string(),
        })?;

        let ret = unsafe {
            let cursor = &*self.inner;
            match cursor.reconfigure {
                Some(f) => f(self.inner, config_cstr.as_ptr()),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "reconfigure function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Set range bounds on the cursor.
    pub fn bound(&mut self, config: &str) -> Result<()> {
        let config_cstr = CString::new(config).map_err(|_| Error {
            code: -1,
            message: "Config contains null byte".to_string(),
        })?;

        let ret = unsafe {
            let cursor = &*self.inner;
            match cursor.bound {
                Some(f) => f(self.inner, config_cstr.as_ptr()),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "bound function not available".to_string(),
                    });
                }
            }
        };

        check_error(ret)
    }

    /// Close the cursor.
    pub fn close(self) -> Result<()> {
        let ret = unsafe {
            let cursor = &*self.inner;
            match cursor.close {
                Some(f) => f(self.inner),
                None => {
                    return Err(Error {
                        code: -1,
                        message: "close function not available".to_string(),
                    });
                }
            }
        };

        std::mem::forget(self);
        check_error(ret)
    }

    /// Get the raw WT_CURSOR pointer.
    pub unsafe fn as_raw(&self) -> *mut wt_sys::WT_CURSOR {
        self.inner
    }
}

impl Drop for Cursor<'_> {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                let cursor = &*self.inner;
                if let Some(close_fn) = cursor.close {
                    let _ = close_fn(self.inner);
                }
            }
        }
    }
}
