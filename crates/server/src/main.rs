use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use protocol::{ClientMessage, InputCommand, PlayerId, ServerMessage};
use shared::{GameWorld, TICK_DT_SECS, TICK_HZ};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, Mutex},
    time,
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

#[derive(Default)]
struct ServerState {
    world: GameWorld,
    next_player_id: PlayerId,
    clients: HashMap<PlayerId, mpsc::UnboundedSender<ServerMessage>>,
    pending_inputs: HashMap<PlayerId, Vec<InputCommand>>,
}

impl ServerState {
    fn allocate_player_id(&mut self) -> PlayerId {
        self.next_player_id += 1;
        self.next_player_id
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let bind = std::env::var("BIND").unwrap_or_else(|_| "127.0.0.1:4000".to_string());
    let listener = TcpListener::bind(&bind)
        .await
        .with_context(|| format!("failed to bind {bind}"))?;

    println!("server listening on {bind} @ {TICK_HZ}hz");

    let state = Arc::new(Mutex::new(ServerState::default()));
    tokio::spawn(tick_loop(Arc::clone(&state)));

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("client connected: {addr}");

        let state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(err) = handle_client(socket, state).await {
                eprintln!("client task error: {err:#}");
            }
        });
    }
}

async fn tick_loop(state: Arc<Mutex<ServerState>>) {
    let mut interval = time::interval(Duration::from_millis((1000 / TICK_HZ) as u64));

    loop {
        interval.tick().await;
        let frame_start = Instant::now();

        let (snapshot, client_senders) = {
            let mut state = state.lock().await;
            state.world.advance_tick();

            let player_ids: Vec<PlayerId> = state.clients.keys().copied().collect();
            for player_id in player_ids {
                if let Some(input) = state
                    .pending_inputs
                    .get_mut(&player_id)
                    .and_then(|queue| queue.pop())
                {
                    state.world.apply_input(player_id, &input, TICK_DT_SECS);
                }
            }

            let snapshot = state.world.snapshot();
            let senders: Vec<(PlayerId, mpsc::UnboundedSender<ServerMessage>)> = state
                .clients
                .iter()
                .map(|(id, tx)| (*id, tx.clone()))
                .collect();

            (snapshot, senders)
        };

        let mut disconnected = Vec::new();
        for (id, tx) in client_senders {
            if tx.send(ServerMessage::Snapshot(snapshot.clone())).is_err() {
                disconnected.push(id);
            }
        }

        if !disconnected.is_empty() {
            let mut state = state.lock().await;
            for id in disconnected {
                state.clients.remove(&id);
                state.pending_inputs.remove(&id);
                state.world.remove_player(id);
                println!("cleaned disconnected player {id}");
            }
        }

        let elapsed = frame_start.elapsed();
        if elapsed > Duration::from_millis((1000 / TICK_HZ) as u64) {
            eprintln!("tick overrun: {elapsed:?}");
        }
    }
}

async fn handle_client(socket: TcpStream, state: Arc<Mutex<ServerState>>) -> anyhow::Result<()> {
    let mut framed = Framed::new(socket, LengthDelimitedCodec::new());
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    let mut player_id: Option<PlayerId> = None;

    loop {
        tokio::select! {
            maybe_msg = framed.next() => {
                let Some(frame_result) = maybe_msg else {
                    break;
                };

                let frame = frame_result?;
                let client_msg: ClientMessage = bincode::deserialize(&frame)
                    .context("failed to deserialize client message")?;

                match client_msg {
                    ClientMessage::Join { name } => {
                        if player_id.is_some() {
                            let _ = tx.send(ServerMessage::Error { message: "already joined".to_string() });
                            continue;
                        }

                        let assigned_id = {
                            let mut state = state.lock().await;
                            let assigned = state.allocate_player_id();
                            state.world.add_player(assigned, name.clone());
                            state.clients.insert(assigned, tx.clone());
                            state.pending_inputs.entry(assigned).or_default();
                            assigned
                        };

                        player_id = Some(assigned_id);
                        let _ = tx.send(ServerMessage::Welcome {
                            player_id: assigned_id,
                            tick_hz: TICK_HZ,
                        });

                        println!("player {assigned_id} joined as {name}");
                    }
                    ClientMessage::Input(input) => {
                        let Some(id) = player_id else {
                            let _ = tx.send(ServerMessage::Error {
                                message: "must Join before sending input".to_string(),
                            });
                            continue;
                        };

                        let mut state = state.lock().await;
                        state.pending_inputs.entry(id).or_default().push(input);
                    }
                    ClientMessage::Ping { client_time_ms } => {
                        let _ = tx.send(ServerMessage::Pong { client_time_ms });
                    }
                }
            }
            outbound = rx.recv() => {
                let Some(outbound_msg) = outbound else {
                    break;
                };

                let data = bincode::serialize(&outbound_msg)
                    .context("failed to serialize server message")?;
                framed.send(Bytes::from(data)).await?;
            }
        }
    }

    if let Some(id) = player_id {
        let mut state = state.lock().await;
        state.clients.remove(&id);
        state.pending_inputs.remove(&id);
        state.world.remove_player(id);
        println!("player {id} disconnected");
    }

    Ok(())
}
