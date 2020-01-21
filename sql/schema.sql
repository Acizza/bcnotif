PRAGMA locking_mode = EXCLUSIVE;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA wal_autocheckpoint = 50;

CREATE TABLE IF NOT EXISTS listener_avgs (
    id INTEGER NOT NULL PRIMARY KEY,
    last_seen DATE NOT NULL DEFAULT (date('now')),
    utc_0 INT,
    utc_4 INT,
    utc_8 INT,
    utc_12 INT,
    utc_16 INT,
    utc_20 INT
);