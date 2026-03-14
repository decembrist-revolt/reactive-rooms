use reactive_chat_rust::Server;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    Server::run().await
}
