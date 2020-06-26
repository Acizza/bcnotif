use crate::path::FilePath;
use anyhow::{Context, Result};
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use std::path::PathBuf;

table! {
    listener_avgs {
        id -> Integer,
        last_seen -> BigInt,
        utc_0 -> Nullable<Integer>,
        utc_4 -> Nullable<Integer>,
        utc_8 -> Nullable<Integer>,
        utc_12 -> Nullable<Integer>,
        utc_16 -> Nullable<Integer>,
        utc_20 -> Nullable<Integer>,
    }
}

pub struct Database(SqliteConnection);

impl Database {
    pub fn open() -> Result<Self> {
        let path = Self::validated_path().context("getting database path failed")?;
        let conn = SqliteConnection::establish(&path.to_string_lossy())
            .context("opening database connection failed")?;

        conn.batch_execute(include_str!("../sql/schema.sql"))
            .context("executing database schema failed")?;

        Ok(Self(conn))
    }

    pub fn validated_path() -> Result<PathBuf> {
        let mut path = FilePath::LocalData
            .validated_dir_path()
            .context("getting local data path failed")?;

        path.push("data.sqlite");
        Ok(path)
    }

    #[inline(always)]
    pub fn conn(&self) -> &SqliteConnection {
        &self.0
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        self.conn().execute("PRAGMA optimize").ok();
    }
}

unsafe impl Sync for Database {}
