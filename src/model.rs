pub mod model {
    use std::ffi::OsStr;
    use std::path::{Path, PathBuf};

    #[derive(Debug)]
    pub struct Entry {
        pub path: String,
        pub abspath: String,
        pub basename: String,
        pub dirname: String,
        pub signature: String,
        pub size: u64,
        pub timestamp: u64,
        pub updated: u64,
    }

    pub fn path_to_string(path: &Path) -> String {
        match path.to_str() {
            Some(any) => any.to_string(),
            None => {
                let lossy_path = path.to_string_lossy().to_string();
                eprintln!("found path with non-UTF8 characters -> {}", lossy_path);
                lossy_path
            }
        }
    }
    pub fn osstr_to_string(osstr: &OsStr) -> String {
        match osstr.to_str() {
            Some(any) => any.to_string(),
            None => {
                let lossy_path = osstr.to_string_lossy().to_string();
                eprintln!("found path with non-UTF8 characters -> {}", lossy_path);
                lossy_path
            }
        }
    }

    pub fn abspath_to_path(root: &Path, abspath: &Path) -> String {
        path_to_string(abspath.strip_prefix(root).unwrap())
    }

    impl Entry {
        pub fn new(path_buf: &PathBuf, root: &Path, hash: &String, size: u64, mod_timestamp: u64, now_timestamp: u64) -> Entry {
            Entry {
                path: abspath_to_path(root, path_buf),
                abspath: path_to_string(path_buf),
                basename: osstr_to_string(path_buf.file_name().unwrap()),
                dirname: path_to_string(path_buf.parent().unwrap()),
                signature: String::from(hash),
                size: size,
                timestamp: mod_timestamp,
                updated: now_timestamp,
            }
        }

        #[cfg(test)]
        pub fn new_simple(path: &str, abspath: &str, basename: &str, dirname: &str, signature: &str, size: u64, mod_timestamp: u64, now_timestamp: u64) -> Entry {
            Entry {
                path: String::from(path),
                abspath: String::from(abspath),
                basename: String::from(basename),
                dirname: String::from(dirname),
                signature: String::from(signature),
                size,
                timestamp: mod_timestamp,
                updated: now_timestamp,
            }
        }
    }
}