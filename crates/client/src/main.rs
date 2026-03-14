use anyhow::Context;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use protocol::{ClientMessage, InputCommand, PlayerId, ServerMessage};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::TcpStream,
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = std::env::var("SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:4000".to_string());
    let name = std::env::var("PLAYER_NAME").unwrap_or_else(|_| "player".to_string());

    let socket = TcpStream::connect(&addr)
        .await
        .with_context(|| format!("failed to connect to {addr}"))?;

    println!("connected to {addr}");
    println!("controls: type one of w/a/s/d or combos like wd, then enter");

    let mut framed = Framed::new(socket, LengthDelimitedCodec::new());
    send(&mut framed, &ClientMessage::Join { name }).await?;

    let mut my_player_id: Option<PlayerId> = None;
    let mut next_seq: u32 = 1;

    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    loop {
        tokio::select! {
            line = lines.next_line() => {
                let Some(line) = line? else {
                    break;
                };

                let (move_x, move_y) = parse_input(&line);
                let cmd = InputCommand { seq: next_seq, move_x, move_y };
                next_seq = next_seq.wrapping_add(1);

                send(&mut framed, &ClientMessage::Input(cmd)).await?;
            }
            maybe_frame = framed.next() => {
                let Some(frame_result) = maybe_frame else {
                    println!("server closed connection");
                    break;
                };

                let frame = frame_result?;
                let msg: ServerMessage = bincode::deserialize(&frame)
                    .context("failed to deserialize server message")?;

                match msg {
                    ServerMessage::Welcome { player_id, tick_hz } => {
                        my_player_id = Some(player_id);
                        println!("joined as player #{player_id} (server tick: {tick_hz}hz)");
                    }
                    ServerMessage::Snapshot(snapshot) => {
                        if let Some(my_id) = my_player_id {
                            if let Some(me) = snapshot.players.iter().find(|p| p.id == my_id) {
                                println!(
                                    "tick={} pos=({:.2},{:.2}) ack_input={}",
                                    snapshot.tick,
                                    me.position.x,
                                    me.position.y,
                                    me.last_processed_input,
                                );
                            }
                        }
                    }
                    ServerMessage::Pong { client_time_ms } => {
                        println!("pong {client_time_ms}");
                    }
                    ServerMessage::Error { message } => {
                        eprintln!("server error: {message}");
                    }
                }
            }
        }
    }

    Ok(())
}

fn parse_input(line: &str) -> (f32, f32) {
    let mut x = 0.0;
    let mut y = 0.0;

    for ch in line.trim().chars() {
        match ch {
            'a' | 'A' => x -= 1.0,
            'd' | 'D' => x += 1.0,
            'w' | 'W' => y += 1.0,
            's' | 'S' => y -= 1.0,
            _ => {}
        }
    }

    (x, y)
}

async fn send(
    framed: &mut Framed<TcpStream, LengthDelimitedCodec>,
    msg: &ClientMessage,
) -> anyhow::Result<()> {
    let data = bincode::serialize(msg)?;
    framed.send(Bytes::from(data)).await?;
    Ok(())
}
