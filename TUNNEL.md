# Tunnel Configuration Guide

ChatQBit supports automatic tunneling to expose your file streaming server to the internet. This allows you to stream torrents from anywhere, even when your bot is running behind NAT/firewall.

## 🌐 Supported Tunnel Providers

### 1. **localhost.run** (Recommended)
- ✅ **Free & Unlimited**: No bandwidth or time limits
- ✅ **No Installation**: Uses SSH (already installed on most systems)
- ✅ **No Account Required**: Works immediately
- ✅ **Automatic**: Just set `TUNNEL_PROVIDER=localhost.run`
- ⚠️ Requires: SSH client (OpenSSH)
- 📝 Random URL format: `https://xxxxx.lhr.life` or `https://xxxxx.localhost.run`

### 2. **Cloudflare Tunnel**
- ✅ **Free & Reliable**: Backed by Cloudflare's infrastructure
- ✅ **Fast**: Low latency worldwide
- ✅ **Stable**: More stable URLs
- ⚠️ Requires: `cloudflared` binary installation
- 📝 URL format: `https://xxxxx.trycloudflare.com`

### 3. **None** (Default)
- Uses the `FILE_SERVER_BASE_URL` from your `.env` file
- For local-only access or when using your own reverse proxy

## 📦 Installation Requirements

### localhost.run
**No installation needed!** Just requires SSH, which is pre-installed on:
- ✅ Linux (all distributions)
- ✅ macOS
- ✅ Windows 10+ (OpenSSH client)

To verify SSH is available:
```bash
which ssh
# or on Windows:
where ssh
```

### Cloudflare Tunnel
Install `cloudflared` binary:

**Linux:**
```bash
wget -q https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb
sudo dpkg -i cloudflared-linux-amd64.deb
```

**macOS:**
```bash
brew install cloudflare/cloudflare/cloudflared
```

**Windows:**
```powershell
winget install --id Cloudflare.cloudflared
```

## ⚙️ Configuration

Edit your `.env` file:

### Option 1: localhost.run (Easiest)
```env
TUNNEL_PROVIDER=localhost.run
```

### Option 2: Cloudflare Tunnel
```env
TUNNEL_PROVIDER=cloudflare
```

### Option 3: No Tunnel (Local Only)
```env
TUNNEL_PROVIDER=none
FILE_SERVER_BASE_URL=http://localhost:8081
```

## 🚀 Usage

1. **Configure tunnel in `.env`:**
   ```env
   TUNNEL_PROVIDER=localhost.run
   ```

2. **Start the bot:**
   ```bash
   ./target/release/chatqbit
   ```

3. **Check the logs for your public URL:**
   ```
   ✅ Tunnel established successfully!
   🌐 Public URL: https://abc123.lhr.life
   📡 Provider: localhost.run
   ```

4. **Use `/stream` command:**
   - The streaming URLs will automatically use your public tunnel URL
   - Share these URLs anywhere - they work from any network!

## 📱 Streaming from Mobile

With tunneling enabled:
1. Add a torrent via Telegram
2. Use `/stream <hash>` command
3. Tap the streaming link
4. Video opens in your browser or media player
5. **Works anywhere** - home, work, mobile data, anywhere!

## 🔒 Security Considerations

### localhost.run
- ✅ Temporary URLs that expire when bot restarts
- ✅ Authenticated via secure tokens in URLs
- ⚠️ URLs are random but **publicly accessible** if someone guesses them
- 💡 Use strong `FILE_SERVER_SECRET` for better security

### Cloudflare Tunnel
- ✅ Cloudflare's DDoS protection
- ✅ HTTPS encryption by default
- ✅ More stable URLs
- ⚠️ Same security model - URLs are protected by tokens

### Best Practices
1. **Strong Secret**: Set a strong `FILE_SERVER_SECRET` in `.env`
   ```env
   FILE_SERVER_SECRET=your-very-long-random-secret-here-min-32-chars
   ```

2. **Private Bot**: Keep your Telegram bot private
   - Only share with trusted users
   - Don't publish the bot token

3. **Monitor Access**: Check logs for unusual activity
   ```bash
   grep "stream" logs/chatqbit.log
   ```

## 🐛 Troubleshooting

### "ssh command not found"
**Solution:** Install OpenSSH client:
- **Ubuntu/Debian:** `sudo apt-get install openssh-client`
- **Fedora:** `sudo dnf install openssh-clients`
- **Windows:** Enable OpenSSH in Settings → Apps → Optional Features

### "cloudflared command not found"
**Solution:** Install cloudflared binary (see Installation Requirements above)

### "Failed to get public URL after 30 seconds"
**Possible causes:**
1. Firewall blocking outbound SSH/HTTP
2. Network connectivity issues
3. Tunnel service temporarily down

**Solution:**
1. Check internet connectivity
2. Try different tunnel provider
3. Check firewall/proxy settings
4. Wait a few minutes and retry

### Tunnel disconnects frequently
**For localhost.run:**
- Normal behavior - reconnects automatically
- SSH connection maintains tunnel via keepalive

**For Cloudflare:**
- More stable, rarely disconnects
- Consider switching to Cloudflare if localhost.run is unstable

### URLs not working from mobile
**Check:**
1. URL is complete and not truncated
2. Token is included in URL
3. Bot is still running
4. Tunnel is still active (check bot logs)

## 🆚 Comparison

| Feature | localhost.run | Cloudflare | None |
|---------|---------------|------------|------|
| Installation | ✅ None | ⚠️ Binary required | ✅ None |
| Speed | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| Stability | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| Setup | 1 line | Binary + 1 line | 1 line |
| Bandwidth | Unlimited | Unlimited | N/A |
| Remote Access | ✅ Yes | ✅ Yes | ❌ No |
| Cost | Free | Free | Free |

## 💡 Pro Tips

1. **Development**: Use `TUNNEL_PROVIDER=none` for faster local testing
2. **Production**: Use `localhost.run` for zero-config remote access
3. **Enterprise**: Use `cloudflare` for better reliability
4. **Custom**: Set up your own reverse proxy and use `none`

## 🔗 Alternative Options

If you need more control, consider:

1. **Your own VPS with reverse proxy:**
   ```env
   TUNNEL_PROVIDER=none
   FILE_SERVER_BASE_URL=https://your-domain.com
   ```

2. **Tailscale/Wireguard VPN:**
   - Run bot on VPN
   - Access via private VPN addresses
   - More secure, no public exposure

3. **ngrok (paid):**
   - More features
   - Custom domains
   - Better stability

## 📚 Learn More

- [localhost.run Documentation](https://localhost.run)
- [Cloudflare Tunnel Docs](https://developers.cloudflare.com/cloudflare-one/connections/connect-apps/)
- [Security Best Practices](./SECURITY.md)
