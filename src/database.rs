use crate::err::Result;
use crate::path::FilePath;
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
        let path = Self::validated_path()?;
        let conn = SqliteConnection::establish(&path.to_string_lossy())?;

        conn.batch_execute(include_str!("../sql/schema.sql"))?;

        Ok(Self(conn))
    }

    pub fn validated_path() -> Result<PathBuf> {
        let mut path = FilePath::LocalData.validated_dir_path()?;
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
