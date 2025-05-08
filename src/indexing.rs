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

pub mod indexing {
use std::{fmt, fs, io};
    use std::collections::HashSet;
    use std::fmt::Formatter;
    use std::fs::DirEntry;
    use std::io::Error;
    use std::ops::Add;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use log::{debug, error, info, warn};
    use rusqlite::Connection;
    use sha2::{Digest, Sha256};

    use crate::db::db::{Database, DatabaseError};
    use crate::model::model::{abspath_to_path, Entry, path_to_string};

    fn traverse(dir: &Path, callback: &dyn Fn(&DirEntry) -> (), options: Option<&IndexingOptions>) -> Result<(), IndexingError> {
        let terminate_at: Option<SystemTime> = match options.is_some() {
            true => match options.unwrap().duration.is_some() {
                true => Some(SystemTime::now().add(Duration::from_secs(options.unwrap().duration.unwrap()))),
                false => None
            },
            false => None
        };

        if dir.is_dir() {
            let entries = match fs::read_dir(dir) {
                Ok(any) => any,
                Err(err) => {
                    error!("Error while attempting to read entries in {:?}! -> {}", dir, err);
                    return Err(
                        IndexingError::ExecutionError(
                            err, format!("Error while attempting to read entries in {:?}!", dir)
                        )
                    );
                }
            };
            for entry in entries {
                if entry.is_err() {
                    error!("Error! -> {}", entry.err().unwrap());
                    continue;
                }

                if terminate_at.is_some() && SystemTime::now() > terminate_at.unwrap() {
                    info!("Execution timed out.");
                    return Err(IndexingError::ExecutionTimeout);
                }

                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    return traverse(&path, callback, options);
                } else if path.is_file() {
                    callback(&entry)
                } else if path.is_symlink() {
                    // skip symlinks?
                } else {
                    // skip any other types?
                }
            }
        }
        Ok(())
    }

    fn verify_root_path(path: &Path) -> &Path {
        let path_str = path.to_str().unwrap();
        if !path.exists() {
            error!("Specified root directory does not exist: {}", path_str);
            panic!("Specified root directory does not exist: {}", path_str);
        } else if !path.is_dir() {
            error!("Specified root directory does not exist: {}", path_str);
            panic!("Specified root directory is not a directory: {}", path_str);
        }
        path
    }

    /// Find indexed files that no longer exist.
    fn remove_deleted_files(db: &Database, root_dir: &Path) -> Result<usize, rusqlite::Error> {
        let paths = db.select_all_paths()?;
        let paths_in_db: HashSet<String> = HashSet::from_iter(paths);

        let paths_on_disk: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
        let callback: &dyn Fn(&DirEntry) -> () = &|dir_entry| {
            let path_buf = dir_entry.path();
            let path = path_to_string(&path_buf);
            paths_on_disk.lock().unwrap().insert(path);
        };
        traverse(root_dir, callback, None);

        let _x = paths_on_disk.lock().unwrap().to_owned();
        let difference = paths_in_db.difference(&_x);
        info!("found difference -> {:?}", difference);

        let difference_as_paths = Vec::from_iter(difference.map(|x| -> String {
            abspath_to_path(root_dir, Path::new(x))
        }));
        info!("difference as paths: {:?}", difference_as_paths);

        let mut delete_count = 0;
        for path in difference_as_paths {
            debug!("Removing entry with key -> {}", path);
            db.remove_entry(&path).unwrap();
            delete_count += 1;
        }

        Ok(delete_count)
    }

    pub struct IndexingOptions {
        pub skip_delete_check: bool,
        pub duration: Option<u64>,
        pub no_sync: bool,
    }
    
    #[derive(Debug)]
    pub enum IndexingError {
        ExecutionError(Error, String),
        ExecutionTimeout,
    }

    impl fmt::Display for IndexingError {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            match self {
                IndexingError::ExecutionError(e, message) => write!(f, "Execution error: {} caused by: {}", message, e),
                IndexingError::ExecutionTimeout => write!(f, "Execution timed out."),
            }
        }
    }

    impl std::error::Error for IndexingError {}

    pub fn index(output_file: &Path, root_dir: &Path, options: &IndexingOptions) -> Result<(), Error> {
        let root = verify_root_path(root_dir);

        let now_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        let connection = Connection::open(output_file).unwrap();
        let db = Database::new(&connection);
        db.init_for(root.to_str().unwrap(), now_timestamp, options.no_sync).unwrap();

        let delete_count: i64 = match options.skip_delete_check {
            false => remove_deleted_files(&db, root_dir).unwrap() as i64,
            true => {
                info!("Skipping removal of deleted files from index.");
                -1
            },
        };

        let add_count = AtomicU64::new(0);
        let update_count = AtomicU64::new(0);
        let skip_count = AtomicU64::new(0);
        let error_count = AtomicU64::new(0);
        let callback: &dyn Fn(&DirEntry) -> () = &|dir_entry| {
            let path_buf = dir_entry.path();
            let key = abspath_to_path(root_dir, &path_buf);
            let found_entry = db.get_entry(&key);
            match found_entry {
                Ok(entry) => {
                    if is_newer_than_last_write(dir_entry, &entry) {
                        debug!("found, but file updated. -> {:?}", entry);
                        match add_entry(&db, &root, &path_buf, dir_entry, now_timestamp) {
                            Ok(_) => {
                                update_count.fetch_add(1, Ordering::Relaxed);
                            },
                            Err(any) => {
                                warn!("Error occurred during processing {} -> {}", path_to_string(path_buf.as_path()), any);
                                error_count.fetch_add(1, Ordering::Relaxed);
                            }
                        };
                    } else {
                        debug!("already found -> {:?}", entry);
                        skip_count.fetch_add(1, Ordering::Relaxed);
                    }
                },
                Err(DatabaseError::EntryNotFound) => {
                    match add_entry(&db, &root, &path_buf, dir_entry, now_timestamp) {
                        Ok(_) => {
                            add_count.fetch_add(1, Ordering::Relaxed);
                        },
                        Err(any) => {
                            warn!("Error occurred during processing {} -> {}", path_to_string(path_buf.as_path()), any);
                            error_count.fetch_add(1, Ordering::Relaxed);
                        }
                    };
                },
                Err(_any) => {
                    error!("Something went wrong! -> {:?}", key);
                    panic!("Something went wrong! -> {:?}", key);
                }
            }
        };
        match traverse(root, callback, Some(options)) {
            Ok(_) => { /* nothing to do */ }
            Err(any) => {
                warn!("Error occurred during processing. caused by: {}", any);
            }
        }

        info!(
            "Added: {}, Updated: {}, Deleted: {}, Skipped: {}, Errors: {}.",
            add_count.into_inner(),
            update_count.into_inner(),
            delete_count,
            skip_count.into_inner(),
            error_count.into_inner()
        );
        Ok(())
    }

    fn is_newer_than_last_write(dir_entry: &DirEntry, entry: &Entry) -> bool {
        let last_written_time = entry.updated;
        let modified_time = dir_entry.metadata().unwrap().modified().unwrap();
        let mod_timestamp = modified_time.duration_since(UNIX_EPOCH).unwrap().as_secs();

        // if file changed since last indexing, then return true
        last_written_time < mod_timestamp
    }

    fn add_entry(db: &Database, root: &Path, path_buf: &PathBuf, dir_entry: &DirEntry, now_timestamp: u64) -> Result<(), Error> {
        let modified_time = dir_entry.metadata().unwrap().modified().unwrap();
        let mod_timestamp = modified_time.duration_since(UNIX_EPOCH).unwrap().as_secs();
        let size = dir_entry.metadata().unwrap().len();

        let start_time = SystemTime::now();

        let hash = String::from_utf8(hash_file(&path_buf)?.to_vec()).unwrap();
        let entry = Entry::new(&path_buf, root, &hash, size, mod_timestamp, now_timestamp);
        let duration = SystemTime::now().duration_since(start_time).unwrap().as_micros();
        let processing_rate = size as f64 / duration as f64;

        info!("Processed in {} ms @ {} MB/s, adding entry -> {:?}", duration / 1000, processing_rate, entry);
        db.add_entry(&entry);

        Ok(())
    }

    fn hash_file(path: &PathBuf) -> Result<[u8; 64], Error> {
        let mut file = fs::File::open(&path)?;
        let mut hasher = Sha256::new();
        let _n = io::copy(&mut file, &mut hasher).unwrap();
        let hash = hasher.finalize();

        let mut hex_hash = [0u8; 64];
        let _res = match base16ct::lower::encode_str(&hash, &mut hex_hash) {
            Err(why) => {
                error!("Error occurred during stringifying the hash. Caused by {}", why);
                panic!("Error occurred during stringifying the hash. Caused by {}", why);
            },
            Ok(res) => res
        };

        Ok(hex_hash)
    }
}

