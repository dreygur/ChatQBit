# How to Start ChatQBit with Tunnel

## ✅ Prerequisites Verified

Your system is ready:
- ✅ SSH installed at `/usr/bin/ssh`
- ✅ qBittorrent running at `http://localhost:8080`
- ✅ `.env` configured with `TUNNEL_PROVIDER=localhost.run`

## 🚀 Start the Bot

### Option 1: Run directly
```bash
cd /home/rakib/Code/rust/ChatQBit
cargo run --release
```

### Option 2: Run in background with logs
```bash
cd /home/rakib/Code/rust/ChatQBit
cargo run --release > bot.log 2>&1 &
tail -f bot.log
```

### Option 3: Use the test script
```bash
cd /home/rakib/Code/rust/ChatQBit
./test_tunnel.sh
```

## 📝 What to Look For

After starting, you should see output like this:

```
2025-10-17T... INFO chatqbit: Bot started successfully!
2025-10-17T... INFO chatqbit: qBittorrent client authenticated
2025-10-17T... INFO chatqbit: qBittorrent download path: /home/rakib/Downloads
2025-10-17T... INFO chatqbit: File server will listen on 0.0.0.0:8081
2025-10-17T... INFO chatqbit: 🚇 Starting tunnel with provider: LocalhostRun
2025-10-17T... INFO fileserver::tunnel: Starting localhost.run tunnel for port 8081
2025-10-17T... INFO fileserver::tunnel: ✅ localhost.run tunnel established: https://abc123xyz.lhr.life
2025-10-17T... INFO chatqbit: ✅ Tunnel established successfully!
2025-10-17T... INFO chatqbit: 🌐 Public URL: https://abc123xyz.lhr.life
2025-10-17T... INFO chatqbit: 📡 Provider: localhost.run
2025-10-17T... INFO chatqbit: File server listening on 0.0.0.0:8081
2025-10-17T... INFO chatqbit: Bot commands menu registered
```

**The important lines are:**
- `✅ Tunnel established successfully!`
- `🌐 Public URL: https://abc123xyz.lhr.life` ← **Your public streaming URL!**

## 🎬 Testing Streaming

1. **Add a torrent:**
   ```
   Send to bot: /magnet
   Then paste a magnet link
   ```

2. **Get streaming links:**
   ```
   Send to bot: /list
   Copy the torrent hash
   Send to bot: /stream <hash>
   ```

3. **The streaming URLs will use your public tunnel URL!**
   ```
   🔗 [Click to Stream](https://abc123xyz.lhr.life/stream/token/video.mkv)
   ```

## ⏱️ Timing

- Bot startup: ~2-3 seconds
- Tunnel establishment: ~5-10 seconds
- Total: ~10-15 seconds until ready

## 🐛 Troubleshooting

### If you don't see tunnel messages:

1. **Check .env file:**
   ```bash
   grep TUNNEL_PROVIDER .env
   # Should show: TUNNEL_PROVIDER=localhost.run
   ```

2. **Check SSH is working:**
   ```bash
   ssh -V
   # Should show: OpenSSH_...
   ```

3. **Test SSH to localhost.run:**
   ```bash
   ssh -R 80:localhost:8081 nokey@localhost.run
   # Should show a URL like: https://xxxxx.lhr.life
   # Press Ctrl+C to exit
   ```

4. **Check logs for errors:**
   ```bash
   cargo run --release 2>&1 | grep -i "tunnel\|error"
   ```

### Common Issues:

**"ssh command not found"**
```bash
sudo apt-get install openssh-client
```

**"Connection refused"**
- Check firewall isn't blocking outbound SSH (port 22)
- Try Cloudflare instead: `TUNNEL_PROVIDER=cloudflare`

**"No tunnel configured"**
- The .env file has `TUNNEL_PROVIDER=none` or it's not set
- Make sure `.env` is in the project root
- Check for typos in the variable name

## 🔄 Switching Tunnel Providers

### To use Cloudflare instead:

1. **Install cloudflared:**
   ```bash
   wget https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb
   sudo dpkg -i cloudflared-linux-amd64.deb
   ```

2. **Update .env:**
   ```env
   TUNNEL_PROVIDER=cloudflare
   ```

3. **Restart bot**

### To disable tunnel (local only):

```env
TUNNEL_PROVIDER=none
```

## 📊 Performance

- **localhost.run**: Good for most users, free, unlimited
- **Cloudflare**: Better for production, more stable
- **none**: Fastest, but only works locally

## 🔒 Security Note

Your streaming URLs are protected by:
- Token-based authentication
- SHA-256 signed URLs
- `FILE_SERVER_SECRET` from .env

Keep your `FILE_SERVER_SECRET` strong (recommended: 32+ random characters)!

## 💡 Pro Tip

Run in background with systemd for automatic restart:

```bash
# Create service file
sudo nano /etc/systemd/system/chatqbit.service
```

```ini
[Unit]
Description=ChatQBit Telegram Bot
After=network.target

[Service]
Type=simple
User=rakib
WorkingDirectory=/home/rakib/Code/rust/ChatQBit
ExecStart=/home/rakib/Code/rust/ChatQBit/target/release/chatqbit
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable chatqbit
sudo systemctl start chatqbit
sudo systemctl status chatqbit
```

View logs:
```bash
journalctl -u chatqbit -f
```
