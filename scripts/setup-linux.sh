#!/usr/bin/env bash
# GraphPad Linux host setup — uinput permissions and libwacom metadata.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

if [[ $EUID -ne 0 ]]; then
  echo "Run as root: sudo $0"
  exit 1
fi

echo "==> Installing udev rule for /dev/uinput"
cat > /etc/udev/rules.d/99-graphpad-uinput.rules <<'EOF'
# Allow users in the input group to create virtual tablets via uinput
KERNEL=="uinput", MODE="0660", GROUP="input", OPTIONS+="static_node=uinput"
EOF
udevadm control --reload-rules
udevadm trigger

echo "==> Installing libwacom tablet descriptor"
install -Dm644 "$REPO_ROOT/libwacom/graphpad.tablet" /etc/libwacom/graphpad.tablet
install -Dm644 "$REPO_ROOT/libwacom/graphpad.svg" /usr/share/libwacom/graphpad.svg

echo "==> Adding current user to input group (if SUDO_USER is set)"
if [[ -n "${SUDO_USER:-}" ]]; then
  usermod -aG input "$SUDO_USER"
  echo "Added $SUDO_USER to group 'input'. Log out and back in for group membership to apply."
fi

echo "==> Done."
echo "Build the host: cd host && cargo build --release"
echo "Run: ./host/target/release/graphpad-host"
