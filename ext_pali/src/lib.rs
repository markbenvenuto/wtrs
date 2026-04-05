use std::{ptr, sync::Arc};

use crate::bindings::{
    WT_CONFIG_ARG, WT_CONNECTION, WT_ITEM, WT_PAGE_LOG, WT_PAGE_LOG_COMPLETE_CHECKPOINT_ARGS,
    WT_PAGE_LOG_DISCARD_ARGS, WT_PAGE_LOG_GET_ARGS, WT_PAGE_LOG_HANDLE, WT_PAGE_LOG_PUT_ARGS,
    WT_SESSION,
};
use tracing::{info, instrument};

mod bindings;

// ---------------------------------------------------------------------------
// WTPageLogHandleTrait
// ---------------------------------------------------------------------------

pub trait WTPageLogHandleTrait {
    fn put(
        &self,
        session: *mut WT_SESSION,
        page_id: u64,
        checkpoint_id: u64,
        args: *mut WT_PAGE_LOG_PUT_ARGS,
        buf: *const WT_ITEM,
    ) -> i32;
    fn get(
        &self,
        session: *mut WT_SESSION,
        page_id: u64,
        checkpoint_id: u64,
        args: *mut WT_PAGE_LOG_GET_ARGS,
        results_array: *mut WT_ITEM,
        results_count: *mut u32,
    ) -> i32;
    fn get_page_ids(
        &self,
        session: *mut WT_SESSION,
        checkpoint_lsn: u64,
        item: *mut WT_ITEM,
        size: *mut usize,
    ) -> i32;
    fn discard(
        &self,
        session: *mut WT_SESSION,
        page_id: u64,
        checkpoint_id: u64,
        args: *mut WT_PAGE_LOG_DISCARD_ARGS,
    ) -> i32;
    fn close(&self, session: *mut WT_SESSION) -> i32;
    fn cache_put(
        &self,
        session: *mut WT_SESSION,
        page_id: u64,
        checkpoint_id: u64,
        args: *mut WT_PAGE_LOG_PUT_ARGS,
        buf: *const WT_ITEM,
    ) -> i32;
    fn cache_has(
        &self,
        session: *mut WT_SESSION,
        page_id: u64,
        checkpoint_id: u64,
        args: *mut WT_PAGE_LOG_PUT_ARGS,
    ) -> i32;
    fn cache_del(
        &self,
        session: *mut WT_SESSION,
        page_id: u64,
        checkpoint_id: u64,
        args: *mut WT_PAGE_LOG_PUT_ARGS,
    ) -> i32;
    fn cache_available(&self, session: *mut WT_SESSION) -> bool;
}

// ---------------------------------------------------------------------------
// WTPageLogHandleHolder: #[repr(C)] so handle is at offset 0 for C type-punning
// ---------------------------------------------------------------------------

#[repr(C)]
struct WTPageLogHandleHolder {
    handle: WT_PAGE_LOG_HANDLE, // Must be first field
    inner: Arc<dyn WTPageLogHandleTrait + Send + Sync>,
}

macro_rules! handle_holder {
    ($plh:expr) => {
        unsafe { &*($plh as *const WTPageLogHandleHolder) }
    };
}

unsafe extern "C" fn wt_plh_put(
    plh: *mut WT_PAGE_LOG_HANDLE,
    session: *mut WT_SESSION,
    page_id: u64,
    checkpoint_id: u64,
    args: *mut WT_PAGE_LOG_PUT_ARGS,
    buf: *const WT_ITEM,
) -> i32 {
    handle_holder!(plh)
        .inner
        .put(session, page_id, checkpoint_id, args, buf)
}

unsafe extern "C" fn wt_plh_get(
    plh: *mut WT_PAGE_LOG_HANDLE,
    session: *mut WT_SESSION,
    page_id: u64,
    checkpoint_id: u64,
    args: *mut WT_PAGE_LOG_GET_ARGS,
    results_array: *mut WT_ITEM,
    results_count: *mut u32,
) -> i32 {
    handle_holder!(plh).inner.get(
        session,
        page_id,
        checkpoint_id,
        args,
        results_array,
        results_count,
    )
}

unsafe extern "C" fn wt_plh_get_page_ids(
    plh: *mut WT_PAGE_LOG_HANDLE,
    session: *mut WT_SESSION,
    checkpoint_lsn: u64,
    item: *mut WT_ITEM,
    size: *mut usize,
) -> i32 {
    handle_holder!(plh)
        .inner
        .get_page_ids(session, checkpoint_lsn, item, size)
}

unsafe extern "C" fn wt_plh_discard(
    plh: *mut WT_PAGE_LOG_HANDLE,
    session: *mut WT_SESSION,
    page_id: u64,
    checkpoint_id: u64,
    args: *mut WT_PAGE_LOG_DISCARD_ARGS,
) -> i32 {
    handle_holder!(plh)
        .inner
        .discard(session, page_id, checkpoint_id, args)
}

unsafe extern "C" fn wt_plh_close(plh: *mut WT_PAGE_LOG_HANDLE, session: *mut WT_SESSION) -> i32 {
    let ret = handle_holder!(plh).inner.close(session);
    unsafe { drop(Box::from_raw(plh as *mut WTPageLogHandleHolder)) };
    ret
}

unsafe extern "C" fn wt_plh_cache_put(
    plh: *mut WT_PAGE_LOG_HANDLE,
    session: *mut WT_SESSION,
    page_id: u64,
    checkpoint_id: u64,
    args: *mut WT_PAGE_LOG_PUT_ARGS,
    buf: *const WT_ITEM,
) -> i32 {
    handle_holder!(plh)
        .inner
        .cache_put(session, page_id, checkpoint_id, args, buf)
}

unsafe extern "C" fn wt_plh_cache_has(
    plh: *mut WT_PAGE_LOG_HANDLE,
    session: *mut WT_SESSION,
    page_id: u64,
    checkpoint_id: u64,
    args: *mut WT_PAGE_LOG_PUT_ARGS,
) -> i32 {
    handle_holder!(plh)
        .inner
        .cache_has(session, page_id, checkpoint_id, args)
}

unsafe extern "C" fn wt_plh_cache_del(
    plh: *mut WT_PAGE_LOG_HANDLE,
    session: *mut WT_SESSION,
    page_id: u64,
    checkpoint_id: u64,
    args: *mut WT_PAGE_LOG_PUT_ARGS,
) -> i32 {
    handle_holder!(plh)
        .inner
        .cache_del(session, page_id, checkpoint_id, args)
}

unsafe extern "C" fn wt_plh_cache_available(
    plh: *mut WT_PAGE_LOG_HANDLE,
    session: *mut WT_SESSION,
) -> bool {
    handle_holder!(plh).inner.cache_available(session)
}

// ---------------------------------------------------------------------------
// WTPageLogHandleImpl: placeholder
// ---------------------------------------------------------------------------

#[allow(dead_code)]
struct WTPageLogHandleImpl;

impl WTPageLogHandleTrait for WTPageLogHandleImpl {
    #[instrument(skip(self))]
    fn put(
        &self,
        session: *mut WT_SESSION,
        page_id: u64,
        checkpoint_id: u64,
        args: *mut WT_PAGE_LOG_PUT_ARGS,
        buf: *const WT_ITEM,
    ) -> i32 {
        info!(?session, page_id, checkpoint_id, ?args, ?buf, "put");
        todo!("put")
    }

    #[instrument(skip(self))]
    fn get(
        &self,
        session: *mut WT_SESSION,
        page_id: u64,
        checkpoint_id: u64,
        args: *mut WT_PAGE_LOG_GET_ARGS,
        results_array: *mut WT_ITEM,
        results_count: *mut u32,
    ) -> i32 {
        info!(
            ?session,
            page_id,
            checkpoint_id,
            ?args,
            ?results_array,
            ?results_count,
            "get"
        );
        todo!("get")
    }

    #[instrument(skip(self))]
    fn get_page_ids(
        &self,
        session: *mut WT_SESSION,
        checkpoint_lsn: u64,
        item: *mut WT_ITEM,
        size: *mut usize,
    ) -> i32 {
        info!(?session, checkpoint_lsn, ?item, ?size, "get_page_ids");
        todo!("get_page_ids")
    }

    #[instrument(skip(self))]
    fn discard(
        &self,
        session: *mut WT_SESSION,
        page_id: u64,
        checkpoint_id: u64,
        args: *mut WT_PAGE_LOG_DISCARD_ARGS,
    ) -> i32 {
        info!(?session, page_id, checkpoint_id, ?args, "discard");
        todo!("discard")
    }

    #[instrument(skip(self))]
    fn close(&self, session: *mut WT_SESSION) -> i32 {
        info!(?session, "close");
        todo!("close")
    }

    #[instrument(skip(self))]
    fn cache_put(
        &self,
        session: *mut WT_SESSION,
        page_id: u64,
        checkpoint_id: u64,
        args: *mut WT_PAGE_LOG_PUT_ARGS,
        buf: *const WT_ITEM,
    ) -> i32 {
        info!(?session, page_id, checkpoint_id, ?args, ?buf, "cache_put");
        todo!("cache_put")
    }

    #[instrument(skip(self))]
    fn cache_has(
        &self,
        session: *mut WT_SESSION,
        page_id: u64,
        checkpoint_id: u64,
        args: *mut WT_PAGE_LOG_PUT_ARGS,
    ) -> i32 {
        info!(?session, page_id, checkpoint_id, ?args, "cache_has");
        todo!("cache_has")
    }

    #[instrument(skip(self))]
    fn cache_del(
        &self,
        session: *mut WT_SESSION,
        page_id: u64,
        checkpoint_id: u64,
        args: *mut WT_PAGE_LOG_PUT_ARGS,
    ) -> i32 {
        info!(?session, page_id, checkpoint_id, ?args, "cache_del");
        todo!("cache_del")
    }

    #[instrument(skip(self))]
    fn cache_available(&self, session: *mut WT_SESSION) -> bool {
        info!(?session, "cache_available");
        todo!("cache_available")
    }
}

// ---------------------------------------------------------------------------
// WTPageLogTrait
// ---------------------------------------------------------------------------

pub trait WTPageLogTrait {
    fn add_reference(&self) -> i32;
    fn abandon_checkpoint(&self, session: *mut WT_SESSION) -> i32;
    fn begin_checkpoint(&self, session: *mut WT_SESSION, checkpoint_id: u64) -> i32;
    fn complete_checkpoint(
        &self,
        session: *mut WT_SESSION,
        args: *mut WT_PAGE_LOG_COMPLETE_CHECKPOINT_ARGS,
    ) -> i32;
    fn complete_checkpoint_ext(
        &self,
        session: *mut WT_SESSION,
        checkpoint_id: u64,
        checkpoint_timestamp: u64,
        checkpoint_metadata: *const WT_ITEM,
        lsnp: *mut u64,
    ) -> i32;
    fn get_complete_checkpoint(&self, session: *mut WT_SESSION, checkpoint_id: *mut u64) -> i32;
    fn get_complete_checkpoint_ext(
        &self,
        session: *mut WT_SESSION,
        checkpoint_lsn: *mut u64,
        checkpoint_id: *mut u64,
        checkpoint_timestamp: *mut u64,
        checkpoint_metadata: *mut WT_ITEM,
    ) -> i32;
    fn get_last_lsn(&self, session: *mut WT_SESSION, lsn: *mut u64) -> i32;
    fn get_open_checkpoint(&self, session: *mut WT_SESSION, checkpoint_id: *mut u64) -> i32;
    fn open_handle(
        &self,
        session: *mut WT_SESSION,
        table_id: u64,
    ) -> Box<dyn WTPageLogHandleTrait + Send + Sync>;
    fn set_last_materialized_lsn(&self, session: *mut WT_SESSION, lsn: u64) -> i32;
    fn trim_table(
        &self,
        session: *mut WT_SESSION,
        table_id: u64,
        start_lsn: u64,
        lsnp: *mut u64,
    ) -> i32;
    fn terminate(&self, session: *mut WT_SESSION) -> i32;
}

// ---------------------------------------------------------------------------
// WTPageLogHolder: #[repr(C)] so page_log is at offset 0 for C type-punning
// ---------------------------------------------------------------------------

#[repr(C)]
struct WTPageLogHolder {
    page_log: WT_PAGE_LOG, // Must be first field
    inner: Arc<dyn WTPageLogTrait + Send + Sync>,
}

macro_rules! holder {
    ($page_log:expr) => {
        unsafe { &*($page_log as *const WTPageLogHolder) }
    };
}

unsafe extern "C" fn wt_add_reference(page_log: *mut WT_PAGE_LOG) -> i32 {
    holder!(page_log).inner.add_reference()
}

unsafe extern "C" fn wt_abandon_checkpoint(
    page_log: *mut WT_PAGE_LOG,
    session: *mut WT_SESSION,
) -> i32 {
    holder!(page_log).inner.abandon_checkpoint(session)
}

unsafe extern "C" fn wt_begin_checkpoint(
    page_log: *mut WT_PAGE_LOG,
    session: *mut WT_SESSION,
    checkpoint_id: u64,
) -> i32 {
    holder!(page_log)
        .inner
        .begin_checkpoint(session, checkpoint_id)
}

// unsafe extern "C" fn wt_complete_checkpoint(
//     page_log: *mut WT_PAGE_LOG,
//     session: *mut WT_SESSION,
//     args: *mut WT_PAGE_LOG_COMPLETE_CHECKPOINT_ARGS,
// ) -> i32 {
//     holder!(page_log).inner.complete_checkpoint(session, args)
// }

unsafe extern "C" fn wt_complete_checkpoint_ext(
    page_log: *mut WT_PAGE_LOG,
    session: *mut WT_SESSION,
    checkpoint_id: u64,
    checkpoint_timestamp: u64,
    checkpoint_metadata: *const WT_ITEM,
    lsnp: *mut u64,
) -> i32 {
    holder!(page_log).inner.complete_checkpoint_ext(
        session,
        checkpoint_id,
        checkpoint_timestamp,
        checkpoint_metadata,
        lsnp,
    )
}

// unsafe extern "C" fn wt_get_complete_checkpoint(
//     page_log: *mut WT_PAGE_LOG,
//     session: *mut WT_SESSION,
//     checkpoint_id: *mut u64,
// ) -> i32 {
//     holder!(page_log).inner.get_complete_checkpoint(session, checkpoint_id)
// }

unsafe extern "C" fn wt_get_complete_checkpoint_ext(
    page_log: *mut WT_PAGE_LOG,
    session: *mut WT_SESSION,
    checkpoint_lsn: *mut u64,
    checkpoint_id: *mut u64,
    checkpoint_timestamp: *mut u64,
    checkpoint_metadata: *mut WT_ITEM,
) -> i32 {
    holder!(page_log).inner.get_complete_checkpoint_ext(
        session,
        checkpoint_lsn,
        checkpoint_id,
        checkpoint_timestamp,
        checkpoint_metadata,
    )
}

unsafe extern "C" fn wt_get_last_lsn(
    page_log: *mut WT_PAGE_LOG,
    session: *mut WT_SESSION,
    lsn: *mut u64,
) -> i32 {
    holder!(page_log).inner.get_last_lsn(session, lsn)
}

unsafe extern "C" fn wt_get_open_checkpoint(
    page_log: *mut WT_PAGE_LOG,
    session: *mut WT_SESSION,
    checkpoint_id: *mut u64,
) -> i32 {
    holder!(page_log)
        .inner
        .get_open_checkpoint(session, checkpoint_id)
}

unsafe extern "C" fn wt_open_handle(
    page_log: *mut WT_PAGE_LOG,
    session: *mut WT_SESSION,
    table_id: u64,
    plh: *mut *mut WT_PAGE_LOG_HANDLE,
) -> i32 {
    let inner = holder!(page_log).inner.open_handle(session, table_id);
    let holder = Box::new(WTPageLogHandleHolder {
        handle: WT_PAGE_LOG_HANDLE {
            page_log,
            plh_put: Some(wt_plh_put),
            plh_get: Some(wt_plh_get),
            plh_get_page_ids: None, //Some(wt_plh_get_page_ids),
            plh_discard: Some(wt_plh_discard),
            plh_close: Some(wt_plh_close),
            plh_cache_put: Some(wt_plh_cache_put),
            plh_cache_has: Some(wt_plh_cache_has),
            plh_cache_del: Some(wt_plh_cache_del),
            plh_cache_available: Some(wt_plh_cache_available),
        },
        inner: inner.into(),
    });
    unsafe { *plh = Box::into_raw(holder) as *mut WT_PAGE_LOG_HANDLE };
    0
}

unsafe extern "C" fn wt_set_last_materialized_lsn(
    page_log: *mut WT_PAGE_LOG,
    session: *mut WT_SESSION,
    lsn: u64,
) -> i32 {
    holder!(page_log)
        .inner
        .set_last_materialized_lsn(session, lsn)
}

unsafe extern "C" fn wt_trim_table(
    page_log: *mut WT_PAGE_LOG,
    session: *mut WT_SESSION,
    table_id: u64,
    start_lsn: u64,
    lsnp: *mut u64,
) -> i32 {
    holder!(page_log)
        .inner
        .trim_table(session, table_id, start_lsn, lsnp)
}

unsafe extern "C" fn wt_terminate(page_log: *mut WT_PAGE_LOG, session: *mut WT_SESSION) -> i32 {
    let ret = holder!(page_log).inner.terminate(session);
    unsafe { drop(Box::from_raw(page_log as *mut WTPageLogHolder)) };
    ret
}

// ---------------------------------------------------------------------------
// WTPageLogImpl: placeholder
// ---------------------------------------------------------------------------

struct WTPageLogImpl;

impl WTPageLogTrait for WTPageLogImpl {
    #[instrument(skip(self))]
    fn add_reference(&self) -> i32 {
        info!("add_reference");
        todo!("add_reference")
    }

    #[instrument(skip(self))]
    fn abandon_checkpoint(&self, session: *mut WT_SESSION) -> i32 {
        info!(?session, "abandon_checkpoint");
        todo!("abandon_checkpoint")
    }

    #[instrument(skip(self))]
    fn begin_checkpoint(&self, session: *mut WT_SESSION, checkpoint_id: u64) -> i32 {
        info!(?session, checkpoint_id, "begin_checkpoint");
        todo!("begin_checkpoint")
    }

    #[instrument(skip(self))]
    fn complete_checkpoint(
        &self,
        session: *mut WT_SESSION,
        args: *mut WT_PAGE_LOG_COMPLETE_CHECKPOINT_ARGS,
    ) -> i32 {
        info!(?session, ?args, "complete_checkpoint");
        todo!("complete_checkpoint")
    }

    #[instrument(skip(self))]
    fn complete_checkpoint_ext(
        &self,
        session: *mut WT_SESSION,
        checkpoint_id: u64,
        checkpoint_timestamp: u64,
        checkpoint_metadata: *const WT_ITEM,
        lsnp: *mut u64,
    ) -> i32 {
        info!(
            ?session,
            checkpoint_id,
            checkpoint_timestamp,
            ?checkpoint_metadata,
            ?lsnp,
            "complete_checkpoint_ext"
        );
        todo!("complete_checkpoint_ext")
    }

    #[instrument(skip(self))]
    fn get_complete_checkpoint(&self, session: *mut WT_SESSION, checkpoint_id: *mut u64) -> i32 {
        info!(?session, ?checkpoint_id, "get_complete_checkpoint");
        todo!("get_complete_checkpoint")
    }

    #[instrument(skip(self))]
    fn get_complete_checkpoint_ext(
        &self,
        session: *mut WT_SESSION,
        checkpoint_lsn: *mut u64,
        checkpoint_id: *mut u64,
        checkpoint_timestamp: *mut u64,
        checkpoint_metadata: *mut WT_ITEM,
    ) -> i32 {
        info!(
            ?session,
            ?checkpoint_lsn,
            ?checkpoint_id,
            ?checkpoint_timestamp,
            ?checkpoint_metadata,
            "get_complete_checkpoint_ext"
        );
        todo!("get_complete_checkpoint_ext")
    }

    #[instrument(skip(self))]
    fn get_last_lsn(&self, session: *mut WT_SESSION, lsn: *mut u64) -> i32 {
        info!(?session, ?lsn, "get_last_lsn");
        todo!("get_last_lsn")
    }

    #[instrument(skip(self))]
    fn get_open_checkpoint(&self, session: *mut WT_SESSION, checkpoint_id: *mut u64) -> i32 {
        info!(?session, ?checkpoint_id, "get_open_checkpoint");
        todo!("get_open_checkpoint")
    }

    #[instrument(skip(self))]
    fn open_handle(
        &self,
        session: *mut WT_SESSION,
        table_id: u64,
    ) -> Box<dyn WTPageLogHandleTrait + Send + Sync> {
        info!(?session, table_id, "open_handle");
        todo!("open_handle")
    }

    #[instrument(skip(self))]
    fn set_last_materialized_lsn(&self, session: *mut WT_SESSION, lsn: u64) -> i32 {
        info!(?session, lsn, "set_last_materialized_lsn");
        todo!("set_last_materialized_lsn")
    }

    #[instrument(skip(self))]
    fn trim_table(
        &self,
        session: *mut WT_SESSION,
        table_id: u64,
        start_lsn: u64,
        lsnp: *mut u64,
    ) -> i32 {
        info!(?session, table_id, start_lsn, ?lsnp, "trim_table");
        todo!("trim_table")
    }

    #[instrument(skip(self))]
    fn terminate(&self, session: *mut WT_SESSION) -> i32 {
        info!(?session, "terminate");
        todo!("terminate")
    }
}

// ---------------------------------------------------------------------------
// Extension entry point
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn wiredtiger_extension_init(
    connection: *mut WT_CONNECTION,
    _config: *mut WT_CONFIG_ARG,
) -> i32 {
    tracing_subscriber::fmt().init();
    info!("Rust extension loaded");

    let holder = Box::new(WTPageLogHolder {
        page_log: WT_PAGE_LOG {
            pl_add_reference: Some(wt_add_reference),
            pl_abandon_checkpoint: Some(wt_abandon_checkpoint),
            pl_begin_checkpoint: Some(wt_begin_checkpoint),
            pl_complete_checkpoint: None, //Some(wt_complete_checkpoint),
            pl_complete_checkpoint_ext: Some(wt_complete_checkpoint_ext),
            pl_get_complete_checkpoint: None, // Some(wt_get_complete_checkpoint),
            pl_get_complete_checkpoint_ext: Some(wt_get_complete_checkpoint_ext),
            pl_get_last_lsn: Some(wt_get_last_lsn),
            pl_get_open_checkpoint: Some(wt_get_open_checkpoint),
            pl_open_handle: Some(wt_open_handle),
            pl_set_last_materialized_lsn: Some(wt_set_last_materialized_lsn),
            pl_trim_table: Some(wt_trim_table),
            terminate: Some(wt_terminate),
        },
        inner: Arc::new(WTPageLogImpl),
    });

    unsafe {
        let add_pl = (*connection).add_page_log.unwrap();
        add_pl(
            connection,
            c"pali".as_ptr(),
            Box::into_raw(holder) as *mut WT_PAGE_LOG,
            ptr::null(),
        )
    }
}
