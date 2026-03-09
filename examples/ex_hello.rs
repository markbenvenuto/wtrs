//! Example: Hello WiredTiger
//!
//! This is an example demonstrating how to create and connect to a database.
//! Rust port of wiredtiger/examples/c/ex_hello.c

use std::fs;
use wtrs::Connection;

fn main() -> wtrs::Result<()> {
    // Create a temporary directory for our database
    let home = "WT_HOME_HELLO";
    
    // Clean up any previous run
    let _ = fs::remove_dir_all(home);
    fs::create_dir_all(home).expect("Failed to create database directory");

    println!("Opening connection to WiredTiger database at '{}'", home);

    // Open a connection to the database, creating it if necessary.
    let conn = Connection::open(home, Some("create"))?;

    println!("Connection opened successfully!");
    println!("  Home directory: {:?}", conn.get_home());
    println!("  Is new database: {}", conn.is_new());

    // Open a session for the current thread's work.
    let session = conn.open_session(None)?;

    println!("Session opened successfully!");

    // Do some work...
    // (In this simple example, we don't do anything)

    // Note: closing the connection implicitly closes open session(s).
    // The session will be dropped automatically, then the connection.
    
    // Explicitly close with a message
    drop(session);
    println!("Session closed.");

    // Connection closes automatically on drop, but we can be explicit
    conn.close_with_config(None)?;
    println!("Connection closed.");

    // Clean up
    let _ = fs::remove_dir_all(home);

    println!("Example completed successfully!");
    Ok(())
}

