// Copyright (c) 2022-2025 Chris Kroells
// 
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// 
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
// 
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

pub mod db {
    use std::fmt;
    use std::fmt::Formatter;
    use log::{error, info};
    use multimap::MultiMap;
    use rusqlite::{Connection, Row, Result};
    use model::Entry;
    use crate::model::model;

    pub struct Database<'a> {
        connection: &'a Connection,
    }

    #[derive(Debug)]
    pub enum DatabaseError {
        EntryNotFound,
        Unexpected,
    }

    impl fmt::Display for DatabaseError {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            // TODO implement proper error message.
            write!(f, "{:?}", self)
        }
    }

    impl std::error::Error for DatabaseError {}

    pub enum Which {
        First,
        Second
    }

    #[derive(Debug)]
    pub struct DatabaseMetadata {
        pub path: String,
        pub last_updated: u64,
    }

    impl DatabaseMetadata {
        pub fn new(path: String, last_updated: u64) -> DatabaseMetadata {
            DatabaseMetadata { path, last_updated }
        }
    }

    const ROW_TO_ENTRY: fn(&Row) -> Result<Entry, rusqlite::Error> = |row: &Row| {
        Ok(Entry {
            path: row.get(0)?,
            abspath: row.get(1)?,
            basename: row.get(2)?,
            dirname: row.get(3)?,
            signature: row.get(4)?,
            size: row.get(5)?,
            timestamp: row.get(6)?,
            updated: row.get(7)?,
        })
    };

    impl Database<'_> {
        pub fn new(connection: &Connection) -> Database {
            Database { connection }
        }

        pub fn init_for(&self, path: &str, now_timestamp: u64, no_sync: bool) -> Result<(), rusqlite::Error> {
            if no_sync {
                info!("Setting no sync to database.");
                self.setup_pragma_disable_sync();
            }
            self.create_metadata_table();
            if !self.has_metadata().unwrap() {
                self.insert_metadata(path, now_timestamp);
            } else {
                let metadata = self.get_metadata(None).unwrap();
                if !metadata.path.eq(path) {
                    error!("Existing database is for '{}', not '{}'", metadata.path, path);
                    panic!("Existing database is for '{}', not '{}'", metadata.path, path);
                }
            }
            info!("metadata path: {:?}", self.get_metadata(None)?);

            self.create_entries_table();
            self.create_entries_index();
            Ok(())
        }

        pub fn setup_pragma_disable_sync(&self) {
            match self.connection.execute("PRAGMA main.synchronous = OFF", []) {
                Ok(0) => {},
                Ok(updates) => {
                    error!("Unexpected number of changes when setting pragma: {}", updates);
                    panic!("Unexpected number of changes when setting pragma: {}", updates);
                },
                Err(why) => {
                    error!("Could not set pragma -> {}", why);
                    panic!("Could not set pragma -> {}", why);
                }
            }
        }

        pub fn bind_second(&self, path: &str) {
            match self.connection.execute("ATTACH ? AS second", [path]) {
                Ok(0) => {},
                Ok(updates) => {
                    error!("Unexpected number of changes attaching database: {}", updates);
                    panic!("Unexpected number of changes attaching database: {}", updates);
                },
                Err(why) => {
                    error!("Could not attach database: {} due to {}", path, why);
                    panic!("Could not attach database: {} due to {}", path, why);
                }
            }
        }

        fn create_metadata_table(&self) {
            match self.connection.execute(
                "CREATE TABLE IF NOT EXISTS metadata (
                        path         TEXT PRIMARY KEY,
                        last_updated INTEGER
                    )",
                (), // empty list of parameters.
            ) {
                Ok(0) => {},
                Ok(updates) => {
                    error!("Unexpected number of changes during entries table creation: {}", updates);
                    panic!("Unexpected number of changes during entries table creation: {}", updates);
                },
                Err(why) => {
                    error!("Unexpected error during entries table creation: {}", why);
                    panic!("Unexpected error during entries table creation: {}", why);
                }
            }
        }

        fn has_metadata(&self) -> Result<bool, DatabaseError> {
            let x = self.connection.query_row("SELECT COUNT(1) FROM metadata", [], |row: &Row| -> rusqlite::Result<u64> {
                Ok(row.get(0).unwrap())
            });
            match x {
                Ok(1) => Ok(true),
                Ok(0) => Ok(false),
                Ok(_) => Err(DatabaseError::Unexpected),
                Err(_) => Err(DatabaseError::Unexpected),
            }
        }

        fn insert_metadata(&self, path: &str, now_timestamp: u64) {
            match self.connection.execute(
                "INSERT INTO metadata (path, last_updated) VALUES (?1, ?2)", [path, now_timestamp.to_string().as_str()]) {
                Ok(1) => {},
                Ok(updates) => {
                    panic!("Unexpected number of changes when inserting into metadata table: {}", updates)
                },
                Err(why) => {
                    panic!("Unexpected error during inserting into metadata table -> {}", why)
                }
            }
        }

        pub fn get_metadata(&self, which: Option<Which>) -> Result<DatabaseMetadata> {
            let table_name = match which {
                None => "main.metadata",
                Some(Which::First) => "main.metadata",
                Some(Which::Second) => "second.metadata"
            };
            let sql = format!("SELECT path, last_updated FROM {}", table_name);
            Ok(self.connection.query_row(
                &sql, [], |row: &Row| {
                    Ok(DatabaseMetadata::new(
                        row.get(0)?,
                        row.get(1)?,
                    ))
                })?)
        }

        fn create_entries_table(&self) {
            match self.connection.execute(
                "CREATE TABLE IF NOT EXISTS entries (
                        path      TEXT PRIMARY KEY,
                        abspath   TEXT NOT NULL,
                        basename  TEXT NOT NULL,
                        dirname   TEXT NOT NULL,
                        signature TEXT NOT NULL,
                        size      INTEGER NOT NULL,
                        timestamp INTEGER NOT NULL,
                        updated   INTEGER NOT NULL
                    )",
                (), // empty list of parameters.
            ) {
                Ok(0) => {},
                Ok(1) => {}, // can be either 0 or 1 for some reason...?
                Ok(updates) => {
                    panic!("Unexpected number of changes during entries table creation: {}", updates)
                },
                Err(why) => {
                    panic!("Unexpected error during entries table creation: {}", why)
                }
            }
        }

        fn create_entries_index(&self) {
            match self.connection.execute(
                "CREATE INDEX IF NOT EXISTS idx_entries_signature ON entries (signature)",
                (), // empty list of parameters.
            ) {
                Ok(0) => {},
                Ok(1) => {}, // can be either 0 or 1 for some reason...?
                Ok(updates) => {
                    panic!("Unexpected number of changes during entries index creation: {}", updates)
                },
                Err(why) => {
                    panic!("Unexpected error during entries index creation: {}", why)
                }
            }
        }

        pub fn add_entry(&self, entry: &Entry) {
            match self.connection.execute(
                "INSERT INTO entries
                        (path, abspath, basename, dirname, signature, size, timestamp, updated)
                        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                    ON CONFLICT(path) DO UPDATE SET
                        abspath = ?2,
                        basename = ?3,
                        dirname = ?4,
                        signature = ?5,
                        size = ?6,
                        timestamp = ?7,
                        updated = ?8",
                (
                    &entry.path, &entry.abspath, &entry.basename, &entry.dirname,
                    &entry.signature, &entry.size, &entry.timestamp, &entry.updated),
            ) {
                Ok(_any) => {},
                Err(why) => {
                    panic!("Failed to add entry to table -> {}", why)
                }
            }
        }

        pub fn get_entry(&self, key: &String) -> Result<Entry, DatabaseError> {
            let mut statement = self.connection.prepare(
                "SELECT
                        path,
                        abspath,
                        basename,
                        dirname,
                        signature,
                        size,
                        timestamp,
                        updated
                    FROM entries
                    WHERE path = ?"
            ).unwrap();

            let found_entry = statement.query_row([key], ROW_TO_ENTRY);
            match found_entry {
                Ok(entry) => Ok(entry),
                Err(rusqlite::Error::QueryReturnedNoRows) => Err(DatabaseError::EntryNotFound),
                Err(_any) => Err(DatabaseError::Unexpected)
            }
        }

        pub fn remove_entry(&self, key: &String) -> Result<(), DatabaseError> {
            let mut statement = self.connection.prepare(
                "DELETE
                    FROM entries
                    WHERE path = ?"
            ).unwrap();

            match statement.execute([key]) {
                Ok(1) => Ok(()),
                Ok(updates) => panic!("Unexpected number of changes during entry removal: {}", updates),
                Err(why) => panic!("Unexpected error during entry removal: {}", why)
            }
        }

        pub fn get_count(&self, which: Option<Which>) -> Result<u64> {
            let table_name = match which {
                None => "main.entries",
                Some(Which::First) => "main.entries",
                Some(Which::Second) => "second.entries"
            };
            let mut statement = self.connection.prepare(
                format!("SELECT COUNT(1) FROM {}", table_name).as_str()
            )?;
            let count = statement.query_row([], |row: &Row| -> rusqlite::Result<u64> {
                row.get(0)
            }).unwrap();

            Ok(count)
        }

        pub fn get_size(&self) -> Result<u64> {
            let mut statement = self.connection.prepare(
                "SELECT SUM(size) FROM entries"
            )?;
            let count = statement.query_row([], |row: &Row| -> rusqlite::Result<u64> {
                row.get(0)
            }).unwrap();

            Ok(count)
        }

        // pub fn select_all(&self) {
        //     let mut statement = self.connection.prepare(
        //         "SELECT
        //                 path,
        //                 abspath,
        //                 basename,
        //                 dirname,
        //                 signature,
        //                 size,
        //                 timestamp,
        //                 updated
        //             FROM entries"
        //     )?;
        //     let entry_iter = statement.query_map([], ROW_TO_ENTRY)?;
        //     for entry in entry_iter {
        //         println!("Found entry {:?}", entry.unwrap());
        //     }
        // }

        pub fn select_all_paths(&self) -> Result<Vec<String>> {
            let mut statement = self.connection.prepare(
                "SELECT abspath FROM entries"
            )?;

            let result_iter = statement.query_map([], |row: &Row| {
                row.get(0)
            })?.map(|x| { x.unwrap() });

            Ok(Vec::from_iter(result_iter))
        }

        pub fn find_missing(&self) -> Result<(Vec<String>, Vec<String>)> {
            let mut statement = self.connection.prepare(
                "SELECT
                        main.entries.path,
                        second.entries.path
                    FROM main.entries
                    LEFT JOIN second.entries ON main.entries.path = second.entries.path
                    WHERE
                        second.entries.path IS NULL
                    UNION
                    SELECT
                        main.entries.path,
                        second.entries.path
                    FROM second.entries
                    LEFT JOIN main.entries ON second.entries.path = main.entries.path
                    WHERE
                        main.entries.path IS NULL"
            )?;
            let entry_iter = statement.query_map([], |row| {
                let first_path = get_row_value(row, 0);
                let second_path = get_row_value(row, 1);
                Ok((first_path, second_path))
            })?;

            let mut missing_in_first: Vec<String> = Vec::new();
            let mut missing_in_second: Vec<String> = Vec::new();

            for entry in entry_iter {
                let entry = entry.unwrap();
                if entry.0.is_none() {
                    missing_in_first.push(entry.1.unwrap());
                } else {
                    missing_in_second.push(entry.0.unwrap());
                }
            }

            Ok((missing_in_first, missing_in_second))
        }

        pub fn compare(&self) -> Result<Vec<(String, String, String, u64, String, String, u64)>> {
            let mut statement = self.connection.prepare(
                "SELECT
                        main.entries.path,
                        main.entries.abspath,
                        main.entries.signature,
                        main.entries.timestamp,
                        second.entries.abspath,
                        second.entries.signature,
                        second.entries.timestamp
                    FROM
                        main.entries
                    LEFT JOIN
                        second.entries ON main.entries.path = second.entries.path
                    WHERE
                        second.entries.path IS NOT NULL
                        AND main.entries.signature != second.entries.signature"
            )?;
            let entry_iter = statement.query_map([], |row| {
                let path: String = row.get(0).unwrap();
                let first_abspath: String = row.get(1).unwrap();
                let first_sig: String = row.get(2).unwrap();
                let first_timestamp: u64 = row.get(3).unwrap();
                let second_abspath: String = row.get(4).unwrap();
                let second_sig: String = row.get(5).unwrap();
                let second_timestamp: u64 = row.get(6).unwrap();
                Ok((path, first_abspath, first_sig, first_timestamp, second_abspath, second_sig, second_timestamp))
            })?;

            Ok(Vec::from_iter(entry_iter.map(|x| { x.unwrap() })))
        }

        pub fn find_dupes(&self) -> Result<MultiMap<String, Entry>> {
            let mut statement = self.connection.prepare(
                "SELECT
                        path,
                        abspath,
                        basename,
                        dirname,
                        signature,
                        size,
                        timestamp,
                        updated
                    FROM entries
                    WHERE signature IN (
                        SELECT
                            signature
                        FROM entries
                        GROUP BY signature
                        HAVING COUNT(*) > 1
                    )
                    ORDER BY signature"
            )?;
            let entry_iter = statement.query_map([], ROW_TO_ENTRY)?;

            let mut dupe_files = MultiMap::new();
            for entry in entry_iter {
                let entry = entry?;
                dupe_files.insert(entry.signature.clone(), entry);
            }

            Ok(dupe_files)
        }
    }

    fn get_row_value(row: &Row, index: usize) -> Option<String> {
        match row.get(index) {
            Ok(any) => Some(any),
            Err(_) => None
        }
    }
}

#[cfg(test)]
mod dupe_tests {
    use rusqlite::Connection;
    use crate::Database;
    use crate::model::model::Entry;

    #[test]
    fn has_dupes() {
        let connection = Connection::open(":memory:").unwrap();
        let database = Database::new(&connection);
        database.init_for("/path/to", 1000, false).unwrap();

        let entry1 = &Entry::new_simple(
            "to/file1", "/path/to/file1", "file1", "/path/to", "00deadbeef", 100, 100, 100
        );
        let entry2 = &Entry::new_simple(
            "to/file2", "/path/to/file2", "file2", "/path/to", "00deadbeef", 100, 100, 100
        );
        let entry3 = &Entry::new_simple(
            "to/file3", "/path/to/file3", "file3", "/path/to", "00cafecafe", 100, 100, 100
        );

        database.add_entry(entry1);
        database.add_entry(entry2);
        database.add_entry(entry3);
        assert_eq!(3, database.get_count(None).unwrap());

        let dupe_files = database.find_dupes().unwrap();
        assert_eq!(2, dupe_files.len());
        let entries = dupe_files.get_vec("00deadbeef").unwrap();
        assert_eq!(entry1.path, entries.get(0).unwrap().path);
        assert_eq!(entry2.path, entries.get(1).unwrap().path);
    }

    #[test]
    fn has_triple_dupes() {
        let connection = Connection::open(":memory:").unwrap();
        let database = Database::new(&connection);
        database.init_for("/path/to", 1000, false).unwrap();

        let entry1 = &Entry::new_simple(
            "to/file1", "/path/to/file1", "file1", "/path/to", "00deadbeef", 100, 100, 100
        );
        let entry2 = &Entry::new_simple(
            "to/file2", "/path/to/file2", "file2", "/path/to", "00deadbeef", 100, 100, 100
        );
        let entry3 = &Entry::new_simple(
            "to/file3", "/path/to/file3", "file3", "/path/to", "00deadbeef", 100, 100, 100
        );

        database.add_entry(entry1);
        database.add_entry(entry2);
        database.add_entry(entry3);
        assert_eq!(3, database.get_count(None).unwrap());

        let dupe_files = database.find_dupes().unwrap();
        assert_eq!(3, dupe_files.len());
        let entries = dupe_files.get_vec("00deadbeef").unwrap();
        assert_eq!(entry1.path, entries.get(0).unwrap().path);
        assert_eq!(entry2.path, entries.get(1).unwrap().path);
        assert_eq!(entry3.path, entries.get(2).unwrap().path);
    }

    #[test]
    fn has_no_dupes() {
        let connection = Connection::open(":memory:").unwrap();
        let database = Database::new(&connection);
        database.init_for("/path/to", 1000, false).unwrap();

        let entry1 = &Entry::new_simple(
            "to/file1", "/path/to/file1", "file1", "/path/to", "00deadbeef", 100, 100, 100
        );
        let entry2 = &Entry::new_simple(
            "to/file2", "/path/to/file2", "file2", "/path/to", "0000000000", 100, 100, 100
        );
        let entry3 = &Entry::new_simple(
            "to/file3", "/path/to/file3", "file3", "/path/to", "00cafecafe", 100, 100, 100
        );

        database.add_entry(entry1);
        database.add_entry(entry2);
        database.add_entry(entry3);
        assert_eq!(3, database.get_count(None).unwrap());

        let dupe_files = database.find_dupes().unwrap();
        assert_eq!(0, dupe_files.len());
    }
}