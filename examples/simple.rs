use bevy::{
    app::{App, EventReader, Events, ScheduleRunnerSettings},
    core::Time,
    ecs::prelude::*,
    MinimalPlugins,
};
use bevy_networking_turbulence::{NetworkEvent, NetworkResource, NetworkingPlugin, Packet};

use std::{net::SocketAddr, time::Duration};

mod utils;
use utils::*;

const SERVER_PORT: u16 = 14191;
const DESYNC_THRESH: f64 = 5.;
const PING_FREQ: f64 = 1.;

fn main() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Debug).expect("cannot initialize console_log");
        }
        else {
            simple_logger::SimpleLogger::from_env()
            .init()
            .expect("A logger was already initialized");
        }
    }

    App::build()
        // minimal plugins necessary for timers + headless loop
        .add_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .add_plugins(MinimalPlugins)
        // The NetworkingPlugin
        .add_plugin(NetworkingPlugin::default())
        // Our networking
        .add_resource(parse_args())
        .add_startup_system(startup.system())
        .add_system(send_packets.system())
        .init_resource::<NetworkReader>()
        .add_system(handle_packets.system())
        .run();
}

fn startup(mut net: ResMut<NetworkResource>, args: Res<Args>) {
    // cfg_if::cfg_if! {
        let mut socket_address: SocketAddr = "127.0.0.1:0".parse().unwrap();
        socket_address.set_port(SERVER_PORT);
        // if #[cfg(target_arch = "wasm32")] {
        //     // FIXME: set this address to your local machine
        //     let mut socket_address: SocketAddr = "127.0.0.1:0".parse().unwrap();
        //     socket_address.set_port(SERVER_PORT);
        // } else {
        //     let ip_address =
        //         bevy_networking_turbulence::find_my_ip_address().expect("can't find ip address");
        //     let socket_address = SocketAddr::new(ip_address, SERVER_PORT);
        // }
    // }

    #[cfg(not(target_arch = "wasm32"))]
    if args.is_server {
        log::info!("Starting server");
        net.listen(socket_address);
    }
    if !args.is_server {
        log::info!("Starting client");
        net.connect(socket_address);
    }
}

fn send_packets(mut net: ResMut<NetworkResource>, mut state: ResMut<NetworkReader>, time: Res<Time>, args: Res<Args>) {
    if !args.is_server && state.connected {
        let now = time.seconds_since_startup();
        if now > state.last_ping + PING_FREQ {
            log::info!("PING");
            net.broadcast(Packet::from("PING"));
            state.last_ping = now;
        }
    }
    if state.last_ping > state.last_pong + DESYNC_THRESH {
        log::warn!("Probably disconnected...")
    }
}

#[derive(Default)]
struct NetworkReader {
    connected: bool,
    last_ping: f64,
    last_pong: f64,
    network_events: EventReader<NetworkEvent>,
}

fn handle_packets(
    mut net: ResMut<NetworkResource>,
    time: Res<Time>,
    mut state: ResMut<NetworkReader>,
    network_events: Res<Events<NetworkEvent>>,
    args: Res<Args>,
) {
    let now = time.seconds_since_startup();
    for event in state.network_events.iter(&network_events) {
        match event {
            NetworkEvent::Connected(_handle) => {
                log::info!("Connected");
                state.connected = true;
            }
            NetworkEvent::Disconnected(_handle) => {
                log::info!("Disconnected");
                state.connected = false;
            }
            NetworkEvent::Packet(handle, packet) => {
                let message = String::from_utf8_lossy(packet);
                log::info!("Got packet on [{}]: {}", handle, message);
                if !args.is_server && message.starts_with("PONG") {
                    state.last_pong = now;
                } else if message == "PING" {
                    let message = format!("PONG @ {}", now);
                    match net.send(*handle, Packet::from(message)) {
                        Ok(()) => {
                            log::info!("Sent PONG");
                        }
                        Err(error) => {
                            log::info!("PONG send error: {}", error);
                        }
                    }
                }
            }
        }
    }
}
