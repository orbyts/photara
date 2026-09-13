//! Offline developer codec for regenerating synthetic fixtures via JSON lines.
//! It reads stdin and writes stdout; it never opens a Project or database.
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::fmt::Write as _;
use std::io::{self, BufRead};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    for line in io::stdin().lock().lines() {
        let value: Value = serde_json::from_str(&line?)?;
        let bytes = photara_core::canonical_json(&value)?;
        let mut hex = String::with_capacity(bytes.len() * 2);
        for byte in &bytes {
            write!(&mut hex, "{byte:02x}")?;
        }
        println!(
            "{}",
            json!({"utf8":String::from_utf8(bytes.clone())?,"sha256":format!("{:x}",Sha256::digest(&bytes)),"byte_length":bytes.len().to_string(),"hex":hex})
        );
    }
    Ok(())
}
