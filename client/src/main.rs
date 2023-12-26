mod record;

use arboard::Clipboard;
use chrono::Local;
use clap::Parser;
use dotenv::dotenv;
use record::{create_record, Record};
use reqwest::{multipart, Body, Client};
use std::{env, path::Path};
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    file_path: String,

    #[arg(short, long)]
    directory: Option<String>,

    #[arg(short, long, default_value = "records.json")]
    records_path: String,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let args = Args::parse();
    let mut clipboard = Clipboard::new().unwrap();

    match upload_file(Path::new(&args.file_path), &args.records_path).await {
        Ok(url) => {
            clipboard.set_text(url).unwrap()
        }
        Err(err) => {
            eprintln!("Error: {}", err)
        }
    }
}

async fn upload_file(
    file_path: &Path,
    records_path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    if !file_path.exists() {
        return Err("File path does not exist".into());
    }

    let base_url = env::var("BASE_URL")?;

    let upload_url = Path::new(&base_url).join("upload");

    let file = File::open(file_path).await?;
    let file_len = file.metadata().await?.len();

    let fstream = FramedRead::new(file, BytesCodec::new());
    let fbody = Body::wrap_stream(fstream);

    let file_name_str = file_path.file_name().unwrap().to_str().unwrap().to_owned();

    let upload_file = multipart::Part::stream_with_length(fbody, file_len)
        .file_name(file_name_str.clone())
        .mime_str(
            &mime_guess::from_path(&file_name_str)
                .first()
                .unwrap()
                .to_string(),
        )?;

    let form = multipart::Form::new().part("file", upload_file);

    let client = Client::new();

    let res = client
        .post(upload_url.to_string_lossy().to_string())
        .header("ACCESS-KEY", env::var("ACCESS_KEY")?)
        .multipart(form)
        .send()
        .await?;

    if !res.status().is_success() {
        return Err("Upload failed".into());
    }

    let text = res.text().await?;
    let final_url = Path::new(&base_url).join("files").join(&text);

    create_record(
        records_path,
        Record {
            time: Local::now().format("%m/%d/%Y %H:%M:%S").to_string(),
            original_file_name: file_name_str,
            url_location: final_url.to_string_lossy().to_string(),
        },
    )
    .await?;

    Ok(final_url.to_string_lossy().to_string())
}
