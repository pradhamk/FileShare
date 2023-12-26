use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Deserialize, Serialize, Debug)]
pub struct Record {
    pub time: String,
    pub original_file_name: String,
    pub url_location: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct Records {
    records: Vec<Record>,
}

pub async fn create_record(
    records_path: &str,
    record: Record,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut records_data: Records = if fs::metadata(records_path).await.is_ok() {
        serde_json::from_str(&fs::read_to_string(records_path).await?)?
    } else {
        Records {
            records: Vec::new(),
        }
    };

    records_data.records.push(record);

    fs::write(records_path, serde_json::to_string_pretty(&records_data)?).await?;

    Ok(())
}
