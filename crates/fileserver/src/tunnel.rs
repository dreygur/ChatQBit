//! Tunnel management for exposing file server to the internet
//!
//! Supports multiple tunnel providers:
//! - localhost.run (SSH-based, using ssh2 library)
//! - Cloudflare Tunnel (requires cloudflared)
//! - Manual (user provides their own public URL)

use std::io::Read;
use std::net::TcpStream;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader as TokioBufReader};
use tokio::process::Command;

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

/// Start a tunnel to expose the local server
///
/// # Arguments
/// * `provider` - Tunnel provider to use
/// * `local_port` - Local port to tunnel
///
/// # Returns
/// * `Ok(TunnelInfo)` - Tunnel started successfully with public URL
/// * `Err(String)` - Failed to start tunnel
pub async fn start_tunnel(provider: TunnelProvider, local_port: u16) -> Result<TunnelInfo, String> {
    match provider {
        TunnelProvider::LocalhostRun => start_localhost_run_tunnel(local_port).await,
        TunnelProvider::Cloudflare => start_cloudflare_tunnel(local_port).await,
        TunnelProvider::None => Err("No tunnel provider configured".to_string()),
    }
}

/// Start localhost.run tunnel using ssh2 library
///
/// This creates an SSH connection and establishes a remote port forward
async fn start_localhost_run_tunnel(local_port: u16) -> Result<TunnelInfo, String> {
    tracing::info!("Starting localhost.run tunnel for port {} using ssh2", local_port);

    // Run the SSH connection in a blocking task since ssh2 is synchronous
    tokio::task::spawn_blocking(move || {
        // Connect to localhost.run
        let tcp = TcpStream::connect("localhost.run:22")
            .map_err(|e| format!("Failed to connect to localhost.run: {}", e))?;

        let mut sess = ssh2::Session::new()
            .map_err(|e| format!("Failed to create SSH session: {}", e))?;

        sess.set_tcp_stream(tcp);
        sess.handshake()
            .map_err(|e| format!("SSH handshake failed: {}", e))?;

        // Authenticate (localhost.run uses "nokey" username with no authentication)
        sess.userauth_password("nokey", "")
            .map_err(|e| format!("Authentication failed: {}", e))?;

        if !sess.authenticated() {
            return Err("Authentication failed".to_string());
        }

        tracing::debug!("Successfully authenticated to localhost.run");

        // Request remote port forwarding
        // localhost.run listens on remote port 80 and forwards to our local_port
        let (mut listener, _bound_port) = sess
            .channel_forward_listen(80, None, None)
            .map_err(|e| format!("Failed to request port forwarding: {}", e))?;

        tracing::debug!("Remote port forwarding established, waiting for URL...");

        // localhost.run sends the URL via SSH stderr messages
        // We need to read from the session's stderr channel
        let mut channel = sess
            .channel_session()
            .map_err(|e| format!("Failed to open session channel: {}", e))?;

        // Request a shell to receive stderr messages
        channel.request_pty("xterm", None, None)
            .map_err(|e| format!("Failed to request PTY: {}", e))?;
        channel.shell()
            .map_err(|e| format!("Failed to start shell: {}", e))?;

        // Read stderr for the URL message (localhost.run sends URL via stderr)
        let mut url = None;
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(30);

        while start.elapsed() < timeout {
            let mut buffer = vec![0u8; 8192];

            // Try to read from channel (PTY combines stdout/stderr)
            match channel.read(&mut buffer) {
                Ok(n) if n > 0 => {
                    if let Ok(text) = std::str::from_utf8(&buffer[..n]) {
                        tracing::debug!("SSH output: {}", text);

                        // Look for URL in the output
                        if (text.contains(".lhr.life") || text.contains(".localhost.run"))
                            && text.contains("https://") {
                            if let Some(found_url) = extract_url_from_line(text) {
                                if !found_url.contains("admin.localhost.run") &&
                                   !found_url.contains("localhost.run/docs") &&
                                   !found_url.contains("twitter.com") {
                                    url = Some(found_url);
                                    break;
                                }
                            }
                        }
                    }
                }
                _ => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }

        let public_url = url.ok_or("Timeout waiting for public URL from localhost.run")?;

        tracing::info!("✅ localhost.run tunnel established: {}", public_url);

        // Keep the session and listener alive
        std::thread::spawn(move || {
            // Keep session alive by accepting forwarded connections
            loop {
                if let Ok(_channel) = listener.accept() {
                    tracing::debug!("Accepted forwarded connection");
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });

        Ok(TunnelInfo {
            public_url,
            provider: "localhost.run".to_string(),
        })
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Start Cloudflare tunnel
async fn start_cloudflare_tunnel(local_port: u16) -> Result<TunnelInfo, String> {
    tracing::info!("Starting Cloudflare tunnel for port {}", local_port);

    // Check if cloudflared is available
    if !is_command_available("cloudflared").await {
        return Err(
            "cloudflared command not found. Install from: https://developers.cloudflare.com/cloudflare-one/connections/connect-apps/install-and-setup/installation/"
                .to_string(),
        );
    }

    // Spawn cloudflared tunnel process
    let mut child = Command::new("cloudflared")
        .args(&[
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
                        tracing::debug!("cloudflared output: {}", line);

                        // Look for URL in format: https://xxxxx.trycloudflare.com
                        if line.contains("trycloudflare.com") || line.contains("https://") {
                            if let Some(url) = extract_url_from_line(&line) {
                                public_url = Some(url);
                                break;
                            }
                        }
                    }
                    Ok(None) => {
                        return Err("cloudflared process ended unexpectedly".to_string());
                    }
                    Err(e) => {
                        return Err(format!("Error reading cloudflared output: {}", e));
                    }
                }
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                attempts += 1;
            }
        }
    }

    let url = public_url.ok_or("Failed to get public URL from Cloudflare after 30 seconds")?;

    tracing::info!("✅ Cloudflare tunnel established: {}", url);

    // Spawn a task to keep the process alive
    tokio::spawn(async move {
        if let Ok(status) = child.wait().await {
            tracing::warn!("Cloudflare tunnel process exited with status: {}", status);
        }
    });

    Ok(TunnelInfo {
        public_url: url,
        provider: "Cloudflare".to_string(),
    })
}

/// Extract URL from a line of text
fn extract_url_from_line(line: &str) -> Option<String> {
    // Look for https:// URLs
    if let Some(start) = line.find("https://") {
        let url_part = &line[start..];

        // Find the end of the URL (space, newline, or end of string)
        let end = url_part
            .find(|c: char| c.is_whitespace() || c == '\n' || c == '\r')
            .unwrap_or(url_part.len());

        let url = url_part[..end].trim().to_string();

        // Validate it's a reasonable URL
        if url.len() > 10 && !url.contains('"') {
            return Some(url);
        }
    }

    None
}

/// Check if a command is available in PATH (used for cloudflared)
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
