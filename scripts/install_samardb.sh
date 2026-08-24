#!/usr/bin/env bash
# ==============================================================================
# SamarDB v1.2.0 - One-Click Linux VM Service Installer
# ==============================================================================
set -euo pipefail

echo "[*] Installing SamarDB v1.2.0 Daemon as Linux Systemd Service..."

# Build if binary is missing
if [ ! -f "dist/samardb-server" ]; then
    bash scripts/build_linux_release.sh
fi

sudo cp dist/samardb-server /usr/local/bin/samardb-server
sudo mkdir -p /var/lib/samardb/data

# Create systemd service
cat << 'EOF' | sudo tee /etc/systemd/system/samardb.service
[Unit]
Description=SamarDB High-Performance PostgreSQL Wire Daemon
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/var/lib/samardb
ExecStart=/usr/local/bin/samardb-server --port 5432 --data-dir /var/lib/samardb/data
Restart=always
RestartSec=3
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable --now samardb

echo "[+] SamarDB v1.2.0 service installed and running on port 5432!"
sudo systemctl status samardb --no-pager
