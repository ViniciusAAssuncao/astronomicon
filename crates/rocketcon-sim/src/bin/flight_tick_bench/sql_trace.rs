use libsqlite3_sys::{SQLITE_OK, SQLITE_TRACE_STMT, sqlite3_sql, sqlite3_trace_v2};
use sqlx::SqliteConnection;
use std::collections::HashMap;
use std::ffi::{CStr, c_int, c_uint, c_void};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

static ENABLED: AtomicBool = AtomicBool::new(false);
static READS: AtomicU64 = AtomicU64::new(0);
static WRITES: AtomicU64 = AtomicU64::new(0);
static OTHERS: AtomicU64 = AtomicU64::new(0);
static TABLES: OnceLock<Mutex<HashMap<String, u64>>> = OnceLock::new();

#[derive(Clone, Copy)]
pub struct SqlCounts {
    pub reads: u64,
    pub writes: u64,
    pub others: u64,
}

impl SqlCounts {
    pub fn since(self, earlier: Self) -> Self {
        Self {
            reads: self.reads - earlier.reads,
            writes: self.writes - earlier.writes,
            others: self.others - earlier.others,
        }
    }
}

pub fn snapshot() -> SqlCounts {
    SqlCounts {
        reads: READS.load(Ordering::Relaxed),
        writes: WRITES.load(Ordering::Relaxed),
        others: OTHERS.load(Ordering::Relaxed),
    }
}

pub fn enable() {
    if let Ok(mut tables) = TABLES.get_or_init(|| Mutex::new(HashMap::new())).lock() {
        tables.clear();
    }
    ENABLED.store(true, Ordering::Relaxed);
}

pub fn disable() {
    ENABLED.store(false, Ordering::Relaxed);
}

pub fn top_tables(limit: usize) -> Vec<(String, u64)> {
    let mut tables: Vec<_> = TABLES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map(|counts| {
            counts
                .iter()
                .map(|(key, value)| (key.clone(), *value))
                .collect()
        })
        .unwrap_or_default();
    tables.sort_by(|a, b| b.1.cmp(&a.1));
    tables.truncate(limit);
    tables
}

unsafe extern "C" fn trace_statement(
    _event: c_uint,
    _context: *mut c_void,
    statement: *mut c_void,
    _details: *mut c_void,
) -> c_int {
    if !ENABLED.load(Ordering::Relaxed) || statement.is_null() {
        return 0;
    }
    let sql = unsafe { sqlite3_sql(statement.cast()) };
    if sql.is_null() {
        return 0;
    }
    let bytes = unsafe { CStr::from_ptr(sql) }.to_bytes();
    let trimmed = bytes.trim_ascii_start();
    let token = trimmed
        .split(|byte| !byte.is_ascii_alphabetic())
        .next()
        .unwrap_or_default();
    let counter = if token.eq_ignore_ascii_case(b"select") || token.eq_ignore_ascii_case(b"with") {
        &READS
    } else if token.eq_ignore_ascii_case(b"insert")
        || token.eq_ignore_ascii_case(b"update")
        || token.eq_ignore_ascii_case(b"delete")
        || token.eq_ignore_ascii_case(b"replace")
    {
        &WRITES
    } else {
        &OTHERS
    };
    counter.fetch_add(1, Ordering::Relaxed);
    let sql_text = String::from_utf8_lossy(trimmed);
    let mut words = sql_text.split_whitespace();
    let mut table = None;
    while let Some(word) = words.next() {
        if word.eq_ignore_ascii_case("from")
            || word.eq_ignore_ascii_case("into")
            || word.eq_ignore_ascii_case("update")
        {
            table = words.next();
            break;
        }
    }
    if let Some(table) = table {
        let table = table.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_');
        if !table.is_empty() {
            if let Ok(mut counts) = TABLES.get_or_init(|| Mutex::new(HashMap::new())).lock() {
                *counts.entry(table.to_ascii_lowercase()).or_insert(0) += 1;
            }
        }
    }
    0
}

pub async fn install(connection: &mut SqliteConnection) -> Result<(), sqlx::Error> {
    let mut handle = connection.lock_handle().await?;
    let result = unsafe {
        sqlite3_trace_v2(
            handle.as_raw_handle().as_ptr(),
            SQLITE_TRACE_STMT as c_uint,
            Some(trace_statement),
            std::ptr::null_mut(),
        )
    };
    if result != SQLITE_OK {
        return Err(sqlx::Error::Protocol(format!(
            "sqlite3_trace_v2 failed: {result}"
        )));
    }
    Ok(())
}
