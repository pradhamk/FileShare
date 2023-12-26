use chrono::Utc;
use std::{fs, io, path};

pub fn log(log_type: &str, log_msg: &str) -> Result<(), io::Error> {
    let log_data = if path::Path::new("server.log").exists() {
        fs::read_to_string("server.log")?
    } else {
        String::new()
    };

    let log_statement = format!("[{}] | {}", log_type.to_uppercase(), log_msg);
    println!("{}", log_statement);

    let timestamp = Utc::now().format("%m/%d/%Y %H:%M:%S").to_string();
    let log_entry = format!("{} {}\n", timestamp, log_statement);

    fs::write("server.log", log_data + &log_entry)?;

    Ok(())
}
