// Test program to debug russh tunnel connection
use std::sync::Arc;
use async_trait::async_trait;
use russh::*;
use russh::client::{Handler, Session};
use russh_keys::key;

struct DebugClient;

#[async_trait]
impl Handler for DebugClient {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        println!("✓ check_server_key called");
        Ok(true)
    }

    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: Channel<russh::client::Msg>,
        connected_address: &str,
        connected_port: u32,
        originator_address: &str,
        originator_port: u32,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        println!("✓ server_channel_open_forwarded_tcpip: {}:{} -> {}:{}",
            originator_address, originator_port, connected_address, connected_port);
        let _ = channel;
        Ok(())
    }

    async fn channel_open_confirmation(
        &mut self,
        id: ChannelId,
        max_packet_size: u32,
        window_size: u32,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        println!("✓ channel_open_confirmation: id={:?}, max_packet={}, window={}",
            id, max_packet_size, window_size);
        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        if let Ok(text) = std::str::from_utf8(data) {
            println!("✓ data on channel {:?}: {}", channel, text);
        } else {
            println!("✓ data on channel {:?}: {} bytes (binary)", channel, data.len());
        }
        Ok(())
    }

    async fn extended_data(
        &mut self,
        channel: ChannelId,
        ext: u32,
        data: &[u8],
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        if let Ok(text) = std::str::from_utf8(data) {
            println!("✓ extended_data on channel {:?} (type {}): {}", channel, ext, text);
        } else {
            println!("✓ extended_data on channel {:?} (type {}): {} bytes", channel, ext, data.len());
        }
        Ok(())
    }

    async fn channel_eof(
        &mut self,
        channel: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        println!("✓ channel_eof: {:?}", channel);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to localhost.run...");

    let config = russh::client::Config::default();
    let handler = DebugClient;

    let mut session = russh::client::connect(Arc::new(config), ("localhost.run", 22), handler).await?;
    println!("✓ Connected");

    let auth_result = session.authenticate_password("nokey", "").await?;
    println!("✓ Authenticated: {}", auth_result);

    let port = session.tcpip_forward("", 80).await?;
    println!("✓ Port forwarding established, assigned port: {}", port);

    let mut channel = session.channel_open_session().await?;
    println!("✓ Session channel opened: {:?}", channel.id());

    channel.request_shell(false).await?;
    println!("✓ Shell requested");

    println!("\nWaiting for 60 seconds to receive messages...\n");
    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

    println!("\nDisconnecting...");
    session.disconnect(russh::Disconnect::ByApplication, "", "en").await?;

    Ok(())
}
