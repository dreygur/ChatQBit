#!/bin/bash
# Test script to verify tunnel functionality

echo "🔍 Checking configuration..."
echo ""

# Check .env file
if grep -q "TUNNEL_PROVIDER=localhost.run" .env; then
    echo "✅ TUNNEL_PROVIDER is set to localhost.run"
else
    echo "❌ TUNNEL_PROVIDER not configured"
    exit 1
fi

# Check SSH
if command -v ssh &> /dev/null; then
    echo "✅ SSH is installed"
else
    echo "❌ SSH not found - required for localhost.run"
    exit 1
fi

# Check qBittorrent
if curl -s -o /dev/null -w "%{http_code}" http://localhost:8080 | grep -q "200"; then
    echo "✅ qBittorrent is running"
else
    echo "❌ qBittorrent not accessible at http://localhost:8080"
    exit 1
fi

echo ""
echo "🚀 Starting bot with tunnel..."
echo "⏳ Please wait 5-10 seconds for tunnel to establish..."
echo ""
echo "Look for these lines in the output:"
echo "  ✅ Tunnel established successfully!"
echo "  🌐 Public URL: https://xxxxx.lhr.life"
echo ""
echo "Press Ctrl+C to stop the bot"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Run the bot
RUST_LOG=info cargo run --release 2>&1 | grep -E "Tunnel|Public URL|File server|tunnel"
