mod server;

use dotenv::dotenv;

#[tokio::main]
async fn main() {
    dotenv().ok();

    server::start().await;
}
