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

extern crate core;

use std::path::Path;
use clap::{Parser, Subcommand};
use rusqlite::Connection;
use crate::db::db::{Database, Which};

mod db;
mod model;
mod benchmark;
mod indexing;
use crate::indexing::indexing::{index, IndexingOptions};

#[derive(Parser)]
#[clap(author, version, about)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan and index files from specified root directory.
    Index {
        /// Skips checks if files were removed after indexing to database.
        /// When disabled, records for files that no longer exist will continue to exist in the database.
        #[clap(short = 'c', long, action, default_value_t = false)]
        skip_delete_check: bool,

        /// Processing duration in seconds (i.e. stops processing after N seconds.)
        #[clap(short = 'd', long, value_name = "DURATION")]
        duration: Option<u64>,

        /// Disables database file sync operations to reduce disk I/O.
        #[clap(short = 's', long, action, default_value_t = false)]
        no_sync: bool,

        /// Root directory to start the scan from.
        #[clap(value_name = "ROOT_DIR")]
        root: String,

        /// Output file (sqlite3 database file.)
        #[clap(value_name = "OUTPUT_FILE")]
        output_file: String,
    },
    /// Compare two indices
    Compare {
        #[clap(value_name = "FIRST")]
        first: String,

        #[clap(value_name = "SECOND")]
        second: String,
    },
    /// Find possible duplicate files.
    Dupe {
        #[clap(value_name = "DATABASE_FILE")]
        file: String
    },
    /// Get stats for database file.
    Stats {
        #[clap(value_name = "DATABASE_FILE")]
        file: String
    },
    /// Benchmark
    Benchmark {}
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Index { skip_delete_check, duration, no_sync, root, output_file } => {
            index(
                Path::new(output_file),
                Path::new(root),
                &IndexingOptions {
                    skip_delete_check: *skip_delete_check,
                    duration: *duration,
                    no_sync: *no_sync
                }
            ).unwrap();
        },
        Commands::Compare { first, second} => {
            compare(first, second);
        },
        Commands::Dupe { file} => {
            dupe(Path::new(file));
        },
        Commands::Stats { file} => {
            stats(Path::new(file));
        },
        Commands::Benchmark {} => {
            benchmark::benchmark::benchmark();
        }
    }
}

fn compare(first: &String, second: &String) {
    let connection = Connection::open(Path::new(first)).unwrap();
    let database = Database::new(&connection);
    database.bind_second(second);

    println!("Files in first: {}", database.get_count(Some(Which::First)).unwrap());
    println!("Files in second: {}", database.get_count(Some(Which::Second)).unwrap());

    let missing_files = database.find_missing().unwrap();
    let missing_in_first = missing_files.0;
    let missing_in_second = missing_files.1;
    println!("Missing in first ({}): {:?}", database.get_metadata(Some(Which::First)).unwrap().path, missing_in_first);
    println!("Missing in second ({}): {:?}", database.get_metadata(Some(Which::Second)).unwrap().path, missing_in_second);

    println!("Differences:");
    for entry in database.compare().unwrap() {
        println!("{:?}", entry);
    }

    println!("OK");
}

fn stats(file: &Path) {
    let connection = Connection::open(file).unwrap();
    let database = Database::new(&connection);

    let entries_in_file = database.get_count(Some(Which::First)).unwrap();
    println!("Entries in file: {}", entries_in_file);

    let size_in_bytes = database.get_size().unwrap();
    let size_in_mb = size_in_bytes / 1000000;
    println!("Total indexed file size: {} B ({} MB)", size_in_bytes, size_in_mb);

    let average_file_size = size_in_bytes as f64 / entries_in_file as f64;
    println!("Average file size: {} B ({} MB)", average_file_size, average_file_size / 1E6);
}

fn dupe(file: &Path) {
    let connection = Connection::open(file).unwrap();
    let database = Database::new(&connection);

    let dupes = database.find_dupes();
    println!("Dupes: {:?}", dupes);
}