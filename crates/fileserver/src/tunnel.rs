//! Tunnel management for exposing file server to the internet
//!
//! Supports multiple tunnel providers:
//! - localhost.run (SSH-based, using russh async library)
//! - Cloudflare Tunnel (requires cloudflared)
//! - Manual (user provides their own public URL)

use russh::client::{self, Msg};
use russh::keys::ssh_key::PublicKey;
use russh::{Channel, ChannelId, Disconnect};
use std::future::Future;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader as TokioBufReader};
use tokio::net::TcpStream;
use tokio::process::Command;
use tokio::sync::{mpsc, watch, Mutex};

/// Tunnel provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TunnelProvider {
    /// localhost.run - SSH-based tunnel, no installation required
    LocalhostRun,
    /// Cloudflare Tunnel - requires cloudflared binary
    Cloudflare,
    /// No tunnel - use manual URL or local only
    None,
}

/// Result of starting a tunnel
#[derive(Debug, Clone)]
pub struct TunnelInfo {
    /// Public URL for accessing the server
    pub public_url: String,
    /// Provider name
    pub provider: String,
}

/// Handle for controlling a running tunnel
pub struct TunnelHandle {
    shutdown_tx: watch::Sender<bool>,
}

impl TunnelHandle {
    /// Signal the tunnel to shut down
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }
}

/// Start a tunnel to expose the local server
///
/// # Arguments
/// * `provider` - Tunnel provider to use
/// * `local_port` - Local port to tunnel
///
/// # Returns
/// * `Ok((TunnelInfo, TunnelHandle))` - Tunnel started successfully
/// * `Err(String)` - Failed to start tunnel
pub async fn start_tunnel(
    provider: TunnelProvider,
    local_port: u16,
) -> Result<(TunnelInfo, TunnelHandle), String> {
    match provider {
        TunnelProvider::LocalhostRun => start_localhost_run_tunnel(local_port).await,
        TunnelProvider::Cloudflare => start_cloudflare_tunnel(local_port).await,
        TunnelProvider::None => Err("No tunnel provider configured".to_string()),
    }
}

/// Shared state for the SSH client handler
struct SharedState {
    url_tx: mpsc::Sender<String>,
    banner_buffer: Mutex<String>,
}

/// Client handler for localhost.run SSH connection
struct LocalhostRunClient {
    local_port: u16,
    state: Arc<SharedState>,
}

impl client::Handler for LocalhostRunClient {
    type Error = russh::Error;

    /// Called when server sends auth banner - localhost.run sends URL here
    fn auth_banner(
        &mut self,
        banner: &str,
        _session: &mut client::Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let state = self.state.clone();
        let banner = banner.to_string();
        async move {
            tracing::debug!("SSH auth banner: {}", banner);
            let mut buffer = state.banner_buffer.lock().await;
            buffer.push_str(&banner);

            if let Some(url) = extract_tunnel_url(&buffer) {
                let _ = state.url_tx.send(url).await;
            }
            Ok(())
        }
    }

    /// Called when server sends channel data
    fn data(
        &mut self,
        _channel: ChannelId,
        data: &[u8],
        _session: &mut client::Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let state = self.state.clone();
        let data = data.to_vec();
        async move {
            if let Ok(text) = std::str::from_utf8(&data) {
                tracing::debug!("SSH data: {}", text);
                let mut buffer = state.banner_buffer.lock().await;
                buffer.push_str(text);

                if let Some(url) = extract_tunnel_url(&buffer) {
                    let _ = state.url_tx.send(url).await;
                }
            }
            Ok(())
        }
    }

    /// Called when server sends extended data (stderr)
    fn extended_data(
        &mut self,
        _channel: ChannelId,
        _ext: u32,
        data: &[u8],
        _session: &mut client::Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let state = self.state.clone();
        let data = data.to_vec();
        async move {
            if let Ok(text) = std::str::from_utf8(&data) {
                tracing::debug!("SSH extended data: {}", text);
                let mut buffer = state.banner_buffer.lock().await;
                buffer.push_str(text);

                if let Some(url) = extract_tunnel_url(&buffer) {
                    let _ = state.url_tx.send(url).await;
                }
            }
            Ok(())
        }
    }

    /// Handle incoming forwarded TCP connection
    fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: Channel<Msg>,
        _connected_address: &str,
        _connected_port: u32,
        _originator_address: &str,
        _originator_port: u32,
        _session: &mut client::Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let local_port = self.local_port;
        async move {
            tracing::info!(
                "Forwarded connection received, proxying to localhost:{}",
                local_port
            );

            // Spawn task to handle this connection
            tokio::spawn(async move {
                if let Err(e) = handle_forwarded_connection(channel, local_port).await {
                    tracing::warn!("Error handling forwarded connection: {}", e);
                }
            });

            Ok(())
        }
    }

    /// Accept all host keys (localhost.run is a known service)
    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> { Ok(true) }
}

/// Handle a forwarded TCP connection by proxying to local server
async fn handle_forwarded_connection(
    channel: Channel<Msg>,
    local_port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Connect to local server
    let local = TcpStream::connect(format!("127.0.0.1:{}", local_port)).await?;
    let (mut local_read, mut local_write) = tokio::io::split(local);

    // Get a stream from the channel for bidirectional I/O
    let (mut channel_read, channel_write) = channel.split();

    // Forward SSH -> Local using make_reader()
    let ssh_to_local = tokio::spawn(async move {
        let mut reader = channel_read.make_reader();
        let mut buf = vec![0u8; 8192];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if local_write.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                    let _ = local_write.flush().await;
                }
                Err(_) => break,
            }
        }
    });

    // Forward Local -> SSH using make_writer()
    let local_to_ssh = tokio::spawn(async move {
        let mut writer = channel_write.make_writer();
        let mut buf = vec![0u8; 8192];
        loop {
            match local_read.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if writer.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                    let _ = writer.flush().await;
                }
                Err(_) => break,
            }
        }
    });

    // Wait for both directions to complete
    let _ = tokio::join!(ssh_to_local, local_to_ssh);

    Ok(())
}

/// Start localhost.run tunnel using russh async library
async fn start_localhost_run_tunnel(local_port: u16) -> Result<(TunnelInfo, TunnelHandle), String> {
    tracing::info!("Starting localhost.run tunnel for port {}", local_port);

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let (url_tx, mut url_rx) = mpsc::channel::<String>(1);

    let config = Arc::new(client::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
        keepalive_interval: Some(std::time::Duration::from_secs(30)),
        keepalive_max: 3,
        ..Default::default()
    });

    let state = Arc::new(SharedState {
        url_tx: url_tx.clone(),
        banner_buffer: Mutex::new(String::new()),
    });

    let handler = LocalhostRunClient {
        local_port,
        state: state.clone(),
    };

    // Connect to localhost.run
    tracing::info!("Connecting to localhost.run:22...");
    let mut handle = client::connect(config, ("localhost.run", 22), handler)
        .await
        .map_err(|e| format!("Failed to connect to localhost.run: {}", e))?;

    tracing::info!("Connected, authenticating...");

    // Authenticate with "none" method (anonymous)
    let auth_result = handle
        .authenticate_none("nokey")
        .await
        .map_err(|e| format!("Authentication failed: {}", e))?;

    if !auth_result.success() {
        return Err("Authentication rejected by server".to_string());
    }

    tracing::info!("Authenticated, opening session channel...");

    // Open a session channel - localhost.run sends URL through this
    let channel = handle
        .channel_open_session()
        .await
        .map_err(|e| format!("Failed to open session channel: {}", e))?;

    tracing::info!("Session channel opened, requesting PTY...");

    // Request PTY (localhost.run needs this to send output)
    channel
        .request_pty(false, "xterm", 80, 24, 0, 0, &[])
        .await
        .map_err(|e| format!("Failed to request PTY: {}", e))?;

    tracing::info!("PTY requested, starting shell...");

    // Start shell to receive output
    channel
        .request_shell(false)
        .await
        .map_err(|e| format!("Failed to request shell: {}", e))?;

    tracing::info!("Shell started, requesting port forwarding...");

    // Request remote port forwarding
    // localhost.run will assign a random subdomain and send URL through the channel
    handle
        .tcpip_forward("localhost", 80)
        .await
        .map_err(|e| format!("Failed to request port forwarding: {}", e))?;

    tracing::info!("Port forwarding requested, waiting for URL...");

    // Spawn task to read from channel and extract URL
    let url_tx_clone = url_tx.clone();
    let (mut channel_read, _channel_write) = channel.split();
    tokio::spawn(async move {
        let mut reader = channel_read.make_reader();
        let mut buffer = Vec::new();
        let mut temp_buf = [0u8; 4096];

        loop {
            match reader.read(&mut temp_buf).await {
                Ok(0) => break,
                Ok(n) => {
                    buffer.extend_from_slice(&temp_buf[..n]);
                    if let Ok(text) = std::str::from_utf8(&buffer) {
                        tracing::debug!("Channel output: {}", text);
                        if let Some(url) = extract_tunnel_url(text) {
                            tracing::info!("Found tunnel URL: {}", url);
                            let _ = url_tx_clone.send(url).await;
                            break;
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!("Channel read error: {}", e);
                    break;
                }
            }
        }
    });

    // Wait for URL with timeout
    let url = tokio::select! {
        url = url_rx.recv() => {
            url.ok_or_else(|| "URL channel closed".to_string())?
        }
        _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
            return Err("Timeout waiting for tunnel URL".to_string());
        }
    };

    tracing::info!("localhost.run tunnel established: {}", url);

    // Spawn background task to keep connection alive
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        tracing::info!("Shutting down localhost.run tunnel");
                        let _ = handle.disconnect(Disconnect::ByApplication, "shutdown", "en").await;
                        break;
                    }
                }
                // Handle keeps running and processes forwarded connections via the Handler trait
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {}
            }
        }
    });

    Ok((
        TunnelInfo {
            public_url: url,
            provider: "localhost.run".to_string(),
        },
        TunnelHandle { shutdown_tx },
    ))
}

/// Extract tunnel URL from server output
fn extract_tunnel_url(text: &str) -> Option<String> {
    // localhost.run URLs look like: https://xxxx.lhr.life or https://xxxx.localhost.run
    for line in text.lines() {
        if let Some(url) = extract_url_from_line(line) {
            // Filter out admin/docs URLs
            if !url.contains("admin.localhost.run")
                && !url.contains("localhost.run/docs")
                && !url.contains("twitter.com")
                && (url.contains(".lhr.life") || url.contains(".localhost.run"))
            {
                return Some(url);
            }
        }
    }
    None
}

/// Start Cloudflare tunnel
async fn start_cloudflare_tunnel(local_port: u16) -> Result<(TunnelInfo, TunnelHandle), String> {
    tracing::info!("Starting Cloudflare tunnel for port {}", local_port);

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

    // Check if cloudflared is available
    if !is_command_available("cloudflared").await {
        return Err(
            "cloudflared command not found. Install from: https://developers.cloudflare.com/cloudflare-one/connections/connect-apps/install-and-setup/installation/"
                .to_string(),
        );
    }

    // Spawn cloudflared tunnel process
    let mut child = Command::new("cloudflared")
        .args([
            "tunnel",
            "--url",
            &format!("http://localhost:{}", local_port),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn cloudflared process: {}", e))?;

    // Read output to get the public URL
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
    let mut reader = TokioBufReader::new(stderr).lines();

    // Parse the URL from cloudflared output
    let mut public_url = None;
    let mut attempts = 0;
    const MAX_ATTEMPTS: u32 = 30;

    while attempts < MAX_ATTEMPTS {
        tokio::select! {
            line = reader.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        tracing::debug!("cloudflared: {}", line);

                        if line.contains("trycloudflare.com") || line.contains("https://") {
                            if let Some(url) = extract_url_from_line(&line) {
                                if url.contains("trycloudflare.com") {
                                    public_url = Some(url);
                                    break;
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        if let Ok(Some(status)) = child.try_wait() {
                            return Err(format!("cloudflared exited unexpectedly: {}", status));
                        }
                        return Err("cloudflared stderr closed unexpectedly".to_string());
                    }
                    Err(e) => {
                        return Err(format!("Error reading cloudflared output: {}", e));
                    }
                }
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                if let Ok(Some(status)) = child.try_wait() {
                    return Err(format!("cloudflared exited before tunnel ready: {}", status));
                }
                attempts += 1;
            }
        }
    }

    let url = public_url.ok_or("Timeout waiting for Cloudflare tunnel URL")?;
    tracing::info!("Cloudflare tunnel established: {}", url);

    // Spawn task to manage the process lifetime
    tokio::spawn(async move {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    tracing::info!("Shutting down Cloudflare tunnel");
                    let _ = child.kill().await;
                }
            }
            status = child.wait() => {
                if let Ok(status) = status {
                    tracing::warn!("Cloudflare tunnel exited: {}", status);
                }
            }
        }
    });

    Ok((
        TunnelInfo {
            public_url: url,
            provider: "Cloudflare".to_string(),
        },
        TunnelHandle { shutdown_tx },
    ))
}

/// Extract URL from a line of text
fn extract_url_from_line(line: &str) -> Option<String> {
    if let Some(start) = line.find("https://") {
        let url_part = &line[start..];
        let end = url_part
            .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
            .unwrap_or(url_part.len());

        let url = url_part[..end].trim().to_string();
        if url.len() > 10 {
            return Some(url);
        }
    }
    None
}

/// Check if a command is available in PATH
async fn is_command_available(cmd: &str) -> bool {
    #[cfg(target_os = "windows")]
    let which_cmd = "where";

    #[cfg(not(target_os = "windows"))]
    let which_cmd = "which";

    Command::new(which_cmd)
        .arg(cmd)
        .output()
        .await
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Parse tunnel provider from string
impl std::str::FromStr for TunnelProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "localhost.run" | "localhostrun" | "localhost-run" => Ok(TunnelProvider::LocalhostRun),
            "cloudflare" | "cf" => Ok(TunnelProvider::Cloudflare),
            "none" | "disabled" | "" => Ok(TunnelProvider::None),
            _ => Err(format!("Unknown tunnel provider: {}", s)),
        }
    }
}
