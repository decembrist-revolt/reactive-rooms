use reactive_chat_rust::Server;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().unwrap();

    Server::run().await
}
