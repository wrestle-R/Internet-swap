# Internet Swap

A small Wi-Fi companion for Linux, coz I’d rather leave things running than keep checking the connection.

[Get the app](https://github.com/wrestle-R/Internet-swap/releases/tag/v0.1.1) · [Website](https://internet-swap.vercel.app) · [Releases](https://github.com/wrestle-R/Internet-swap/releases)

I recently found that the 2.4 GHz and 5 GHz networks I use gave me different speeds. In a few short tests, 2.4 GHz had slightly better download and 5 GHz had better upload. That’s an observation from my setup, not a rule about either band.

Internet Swap watches saved, secured Wi-Fi networks that are currently available. It uses recent speed measurements to look for a better connection and repeated internet checks to detect trouble. A cooldown limits repeated switches, and a failed candidate connection attempts to restore the previous Wi-Fi.

## The little window

- Current Wi-Fi, live traffic and saved networks nearby.
- HyprTrack’s Plum palette, with saved light and dark appearance.
- Timestamped download/upload test results. Missing results stay missing.
- One persistent toggle for automatic switching and active tests.
- Background service that keeps running when the window closes.
- Matching controls in my Hyprland sidebar, alongside HyprTrack’s existing passive traffic display.

## A few limits

One adapter generally joins one Wi-Fi network at a time. Testing another network or switching can interrupt downloads, calls and chat requests. This doesn’t combine bandwidth or guarantee uninterrupted sessions.

Alternative networks are tested on demand or during recovery. Healthy connections aren’t interrupted just to refresh another network’s speed. Automatic speed-based selection needs recent measurements, a meaningful improvement and repeated confirmation. Signal strength alone doesn’t prove that internet is faster.

While enabled, the app checks internet access over HTTPS and runs a bounded test on the current Wi-Fi roughly every two minutes. Successful speed tests transfer about 1.25 MiB each, plus protocol overhead. USB tethering is outside its selection logic. It doesn’t change wired routes or their priority.

Linux x86_64, NetworkManager, curl and coreutils. First release is a preview; see the release notes for package requirements and verification coverage.

## Inside

`tauri/` holds the desktop app and Rust service. `next/` holds the website. `release/` keeps release notes. Version tags build Linux AppImage and Debian packages through GitHub Actions.

[MIT](LICENSE). Passwords stay with NetworkManager; local connection names and measurements stay on your device. Internet checks contact Google/Cloudflare, and speed tests contact Cloudflare. No analytics or account signup.
