# Trakbit

<div align="center">

![Trakbit Demo](public/demo.gif)

[![v1.0.0 Stable Release](https://img.shields.io/badge/version-v1.0.0-green.svg)](https://github.com/yourusername/trakbit/releases)

**The Keyboard-First Options Analytics Tool.**
<br>
Design option strategies simply using your keyboard. Run locally, securely, and privately.

[Get Started](#installation--prerequisite) • [Downloads](#downloads) • [Shortcuts](#keyboard-shortcuts)

</div>

## Privacy First Design

Trakbit is built to be transparent and secure by default.

*   **TUI Desktop Application**: Runs locally on your machine, leveraging the power of your own hardware.
*   **Direct API Connection**: Fetches data directly from Upstox API. No middleman servers involved.
*   **Local Token Storage**: Your access token stays strictly on your computer. We never see it.
*   **No Server Storage**: We don't store any of your personal data or trading history.
*   **Purely Analytics**: Read-only access. Trakbit cannot place trades on your behalf.

## Downloads

Available for all major platforms.

| Platform | Download | Note |
| :--- | :--- | :--- |
| **macOS** | [Download Binary](https://trakbit-releases.s3.ap-south-1.amazonaws.com/builds/macos-latest/trakbit) | Requires `chmod +x` |
| **Linux** | [Download Binary](https://trakbit-releases.s3.ap-south-1.amazonaws.com/builds/ubuntu-latest/trakbit) | Requires `chmod +x` |
| **Windows** | [Download Installer](https://trakbit-releases.s3.ap-south-1.amazonaws.com/builds/windows-latest/trakbit.exe) | `.exe` installer |

## Installation / Prerequisite

### 1. Upstox API Access Token
Required to fetch real-time data. You need to create an app to get your token.
[Get Access Token](https://account.upstox.com/developer/apps)

### 2. Quick Start (macOS & Linux)
Make the binary executable before running:

```bash
chmod +x trakbit
./trakbit
```

> **Recommendation**: For the best experience, we strongly recommend running this application using [Ghostty Terminal](https://ghostty.org/).

## Keyboard Shortcuts

Navigate, trade, and manage positions without ever touching your mouse.

### Expiry Switching
*   `Shift` + `←` : Switch to Previous Expiry
*   `Shift` + `→` : Switch to Next Expiry

### Position Management
*   `B` : Buy option at selected type
*   `S` : Sell option at selected type
*   `Del` / `⌫` : Remove position
*   `Space` : Select/Deselect position
*   `Shift` + `↑` : Move position to lower strike
*   `Shift` + `↓` : Move position to higher strike

## Disclaimer

Trakbit is an independent analytical tool and is **not affiliated, associated, or endorsed by Upstox** or any other broker. All trademarks belong to their respective owners.

**Risk Disclosure**: Trading options involves **substantial risk of loss** and is not suitable for every investor. This tool is for educational and analytical purposes only. We are not SEBI registered advisors. The developers and Trakbit Techsolutions LLP are not responsible for any financial losses incurred.
