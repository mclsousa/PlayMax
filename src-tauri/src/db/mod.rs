pub mod catalog_filters;
pub mod catalog_fts;
pub mod channels;
pub mod epg;
pub mod favorites;
pub mod ids;
pub mod history;
pub mod migrations;
pub mod models;
pub mod movies;
pub mod profiles;
pub mod search;
pub mod series;
pub mod series_tracking;
pub mod settings;

use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

/// Dedicated read connections. WAL only allows readers to run concurrently
/// with the writer when they use their own connections; with a single shared
/// connection every tab query would wait for sync transactions to finish.
const READ_POOL_SIZE: usize = 3;

pub struct DbState {
    pub conn: Mutex<Connection>,
    readers: Vec<Mutex<Connection>>,
    next_reader: AtomicUsize,
}

/// Speeds up large catalog writes during sync. Call `restore_write_pragmas` after commit.
pub fn apply_bulk_write_pragmas(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "PRAGMA synchronous = OFF;
         PRAGMA temp_store = MEMORY;
         PRAGMA cache_size = -64000;",
    )?;
    Ok(())
}

pub fn restore_write_pragmas(conn: &Connection) -> AppResult<()> {
    conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
    Ok(())
}

impl DbState {
    pub fn new(path: PathBuf) -> AppResult<Self> {
        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;",
        )?;
        migrations::run(&conn)?;

        let mut readers = Vec::with_capacity(READ_POOL_SIZE);
        for _ in 0..READ_POOL_SIZE {
            let reader = Connection::open(&path)?;
            reader.execute_batch(
                "PRAGMA query_only = ON; PRAGMA busy_timeout = 5000;",
            )?;
            readers.push(Mutex::new(reader));
        }

        Ok(Self {
            conn: Mutex::new(conn),
            readers,
            next_reader: AtomicUsize::new(0),
        })
    }

    /// Single-connection state for in-memory databases (tests); reads fall
    /// back to the write connection.
    pub fn single(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
            readers: Vec::new(),
            next_reader: AtomicUsize::new(0),
        }
    }

    pub fn with_conn<F, T>(&self, f: F) -> AppResult<T>
    where
        F: FnOnce(&Connection) -> AppResult<T>,
    {
        let conn = self
            .conn
            .lock()
            .map_err(|_| AppError::msg("Database lock poisoned"))?;
        f(&conn)
    }

    /// Runs a read-only query on the reader pool, so catalog browsing never
    /// waits on sync transactions holding the write connection.
    pub fn with_read<F, T>(&self, f: F) -> AppResult<T>
    where
        F: FnOnce(&Connection) -> AppResult<T>,
    {
        if self.readers.is_empty() {
            return self.with_conn(f);
        }

        for reader in &self.readers {
            if let Ok(conn) = reader.try_lock() {
                return f(&conn);
            }
        }

        let index = self.next_reader.fetch_add(1, Ordering::Relaxed) % self.readers.len();
        let conn = self.readers[index]
            .lock()
            .map_err(|_| AppError::msg("Database lock poisoned"))?;
        f(&conn)
    }
}

pub fn init_db(app: &AppHandle) -> AppResult<()> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::msg(e.to_string()))?;
    std::fs::create_dir_all(&dir)?;
    let db_path = dir.join("playmax.db");
    let state = Arc::new(DbState::new(db_path)?);
    app.manage(state);
    Ok(())
}

pub type SharedDb = Arc<DbState>;
