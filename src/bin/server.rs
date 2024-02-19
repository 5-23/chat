use core::time;
use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
    thread,
    time::{Duration, Instant},
};

use renet::{
    transport::{
        NetcodeServerTransport, ServerAuthentication, ServerConfig, NETCODE_USER_DATA_BYTES,
    },
    ClientId, ConnectionConfig, DefaultChannel, RenetServer, ServerEvent,
};

struct Username(String);

impl Username {
    fn from_user_data(user_data: &[u8; NETCODE_USER_DATA_BYTES]) -> Self {
        Self(String::from_utf8(user_data.to_vec()).unwrap())
    }
}

fn main() {
    let server_addr: SocketAddr = "127.0.0.1:4001".parse().unwrap();
    server(server_addr);
}

fn server(public_addr: SocketAddr) {
    let connection_config = ConnectionConfig::default();
    let mut server: RenetServer = RenetServer::new(connection_config);

    let current_time = time::Duration::from_secs(0);

    let server_config = ServerConfig {
        current_time, 
        max_clients: 64, 
        protocol_id: 7, 
        public_addresses: vec![public_addr],
        authentication: ServerAuthentication::Unsecure,
    };
    let socket: UdpSocket = UdpSocket::bind(public_addr).unwrap();

    let mut transport = NetcodeServerTransport::new(server_config, socket).unwrap();

    let mut usernames: HashMap<ClientId, String> = HashMap::new();
    let mut received_messages: Vec<(ClientId, String)> = vec![];
    let mut last_updated = Instant::now();

    loop {
        let now = Instant::now();
        let ping = now - last_updated;
        last_updated = now;


        transport
            .update(
                ping,
                &mut server,
            )
            .unwrap();

        received_messages.clear();
        let event = server.get_event();
        if event.is_some() {
            let event = event.unwrap();
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    let user_data = transport.user_data(client_id).unwrap();
                    let username = Username::from_user_data(&user_data);

                    usernames.insert(client_id, username.0);
                    println!("Client {} connected.", client_id);


                    server.broadcast_message_except(
                        client_id,
                        DefaultChannel::ReliableOrdered,
                        format!(
                            "\"{}\"님이 접속했어요!",
                            String::from_utf8(user_data.to_vec()).unwrap()
                        ),
                    );
                }
                ServerEvent::ClientDisconnected { client_id, reason } => {
                    println!("Client {} disconnected: {}", client_id, reason);
                    if let Some(username) = usernames.remove(&client_id) {
                        server.broadcast_message_except(
                            client_id,
                            DefaultChannel::ReliableOrdered,
                            format!("\"{}\"님이 나갔어요..", username),
                        );
                    }
                }
            }
        }

        for client_id in server.clients_id() {
            while let Some(message) =
                server.receive_message(client_id, DefaultChannel::ReliableOrdered)
            {
                let text = String::from_utf8(message.into()).unwrap();
                let username = usernames.get(&client_id).unwrap();
                println!("Client {} ({}) sent text: {}", username, client_id, text);
                let messages = format!("{}: {}", username, text);
                received_messages.push((client_id, messages));
            }
        }

        for message in received_messages.iter() {

            server.broadcast_message_except(
                message.0,
                DefaultChannel::ReliableOrdered,
                message.1.clone(),
            )
        }

        transport.send_packets(&mut server);
        thread::sleep(Duration::from_micros(50))
    }
}
