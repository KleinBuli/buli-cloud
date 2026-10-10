use cloud_core::instances::instance::Instance;
use cloud_core::logger::logger::{LogLevel, LogLevel::Info, log};
use futures_util::SinkExt;
use futures_util::StreamExt;
use reqwest::Client;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::{Error, Message};

pub(super) async fn list(http_client: &Client, url: &str) -> Result<bool, reqwest::Error> {
    let response = http_client.get(format!("{}instances/", url)).send().await?;
    let status = response.status();
    let body = response.text().await?;

    if status.is_success() {
        log(Info, &body.to_string())
    } else {
        log(LogLevel::Error, &format!("Error while requesting instances: {} - {}", body, status))
    }

    Ok(true)
}

pub(super) async fn console(url: &str, instance_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let ws_url = format!("{}instances/{}/console", url.replace("http://", "ws://"), instance_id);
    let (socket, response) = connect_async(ws_url).await.map_err(std::io::Error::other)?;
    println!("Connected: {}", response.status());

    let (mut write, mut read) = socket.split();
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    loop {
        tokio::select! {
            message = read.next() => {
                if handle_message(message) { break; };
            }

            input = handle_input(&mut lines, &mut write) => {
                if input? { break; }
            }
        }
    }

    Ok(true)
}

fn handle_message(message: Option<Result<Message, Error>>) -> bool {
    match message {
        Some(Ok(Message::Text(text))) => {
            println!("{text}");
            false
        }

        Some(Ok(Message::Close(_))) | None => true,

        Some(Err(error)) => {
            log(LogLevel::Error, &format!("WebSocket error: {error}"));

            true
        }

        _ => false,
    }
}

async fn handle_input(
    lines: &mut tokio::io::Lines<BufReader<tokio::io::Stdin>>,
    write: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        tokio_tungstenite::tungstenite::Message,
    >,
) -> Result<bool, Box<dyn std::error::Error>> {
    match lines.next_line().await? {
        Some(command) => {
            if command.eq_ignore_ascii_case("//exit") {
                return Ok(true);
            }

            write.send(tokio_tungstenite::tungstenite::Message::Text(command.into())).await?;

            Ok(false)
        }

        None => Ok(true),
    }
}

pub(super) async fn start(http_client: &Client, url: &str, group: String, template: String) -> Result<bool, reqwest::Error> {
    let response = http_client.post(format!("{}instances/new/{}/{}", url, group, template)).send().await?;

    let status = response.status();

    if !status.is_success() {
        let body = response.text().await?;
        log(LogLevel::Error, &format!("Error while creating instance: {} - {}", status, body));
        return Ok(true);
    }

    let (id, _) = response.json::<(String, Instance)>().await?;

    log(Info, &format!("Created Instance: {}", id));

    let response = http_client.post(format!("{}instances/{}/start", url, id)).send().await?;

    let status = response.status();

    if status.is_success() {
        log(Info, &format!("Started Instance: {}", id));
    } else {
        let body = response.text().await?;
        log(LogLevel::Error, &format!("Error while starting instance {}: {} - {}", id, status, body));
    }

    Ok(true)
}
