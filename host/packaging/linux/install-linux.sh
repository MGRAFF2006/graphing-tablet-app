#!/usr/bin/env sh
set -eu

if [ "$(id -u)" -ne 0 ]; then
  echo "Run as root: sudo sh host/packaging/linux/install-linux.sh" >&2
  exit 1
fi

install -Dm644 host/packaging/linux/99-graphpad-uinput.rules /etc/udev/rules.d/99-graphpad-uinput.rules
install -Dm644 libwacom/graphpad-virtual.tablet /etc/libwacom/graphpad-virtual.tablet
install -Dm644 libwacom/graphpad-virtual.svg /etc/libwacom/layouts/graphpad-virtual.svg

udevadm control --reload-rules
udevadm trigger || true

echo "Installed GraphPad udev and libwacom metadata."
echo "Make sure your user belongs to the input group, then log out and back in."

