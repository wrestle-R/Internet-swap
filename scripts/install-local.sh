#!/bin/sh
# Install a reviewed local package, without root or embedded credentials.
set -eu
if [ "$#" -ne 1 ] || [ ! -f "$1" ]; then
    echo 'Expected one local Internet Swap AppImage path.' >&2
    exit 2
fi
install -d "$HOME/.local/bin" "$HOME/.local/share/applications" "${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
install -m 755 "$1" "$HOME/.local/bin/internet-swap.AppImage"
cat > "${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/internet-swap.service" <<'UNIT'
[Unit]
Description=Internet Swap Wi-Fi companion
After=graphical-session-pre.target

[Service]
ExecStart=%h/.local/bin/internet-swap.AppImage --daemon
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
UNIT
cat > "$HOME/.local/share/applications/internet-swap.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Internet Swap
Comment=A small Wi-Fi companion for Linux
Exec="$HOME/.local/bin/internet-swap.AppImage"
Icon=network-wireless
Categories=Network;Utility;
Terminal=false
DESKTOP
systemctl --user daemon-reload
systemctl --user start internet-swap.service
