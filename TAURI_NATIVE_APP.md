# DataFold Native macOS App

This document explains how to build and run DataFold as a native macOS application using Tauri.

## 🎯 Overview

DataFold can now run as a **native macOS application** instead of just a web server. The native app:

- ✅ Runs as a standalone `.app` that you can launch from Applications
- ✅ Embeds the DataFold Rust backend server internally (no external dependencies)
- ✅ Uses the same React UI, served through a native window
- ✅ Stores data in `~/.datafold/data` (separate from the development database)
- ✅ Has a native app icon and appears in the Dock
- ✅ Small bundle size (~15-20MB vs 120MB+ for Electron)
- ✅ Better performance and security than Electron

## 🚀 Quick Start

### Development Mode

To run the app in development mode (with hot-reload):

```bash
./run_tauri_dev.sh
```

This will:
1. Build the React frontend
2. Start the DataFold server automatically
3. Open a native window with the UI

### Production Build

To create a production `.app` bundle:

```bash
./build_macos_app.sh
```

This creates:
- `DataFold.app` - The native macOS application
- `DataFold.dmg` - A disk image for distribution

The built app will be in:
```
src/datafold_node/static-react/src-tauri/target/release/bundle/macos/
```

## 📦 Installation

After building, you can:

1. **Copy to Applications:**
   ```bash
   cp -r src/datafold_node/static-react/src-tauri/target/release/bundle/macos/DataFold.app /Applications/
   ```

2. **Launch from Applications folder**

3. **Or double-click the DMG** and drag to Applications

## 🏗️ Architecture

### How It Works

```
┌─────────────────────────────────────────┐
│         DataFold Native App             │
│  ┌───────────────────────────────────┐  │
│  │   Tauri Window (Native macOS)     │  │
│  │  ┌─────────────────────────────┐  │  │
│  │  │   React UI (WebView)        │  │  │
│  │  │   http://localhost:9001     │  │  │
│  │  └─────────────────────────────┘  │  │
│  └───────────────────────────────────┘  │
│                    ↓                     │
│  ┌───────────────────────────────────┐  │
│  │  Embedded DataFold Server         │  │
│  │  (Rust Actix-web on localhost)    │  │
│  │  - Schema management              │  │
│  │  - Query/Mutation APIs            │  │
│  │  - AI ingestion                   │  │
│  └───────────────────────────────────┘  │
│                    ↓                     │
│  ┌───────────────────────────────────┐  │
│  │  FoldDB (Sled Database)           │  │
│  │  ~/.datafold/data                 │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

### Key Components

1. **Tauri Shell** (`src/datafold_node/static-react/src-tauri/`)
   - Native window management
   - macOS integration (dock, menubar, notifications)
   - App lifecycle management

2. **Embedded Server** (`src/datafold_node/embedded_server.rs`)
   - Starts DataFold HTTP server in background
   - Runs on localhost:9001
   - Automatic shutdown on app quit

3. **React Frontend** (existing UI)
   - Same UI as the web version
   - Rendered in native WebView
   - Connects to localhost:9001

## 🔧 Development

### Prerequisites

- Rust 1.70+
- Node.js 16+
- Xcode Command Line Tools (for macOS)

### Project Structure

```
fold_db/
├── src/datafold_node/
│   ├── embedded_server.rs      # New: Embeddable server module
│   ├── static-react/
│   │   ├── src-tauri/          # New: Tauri configuration
│   │   │   ├── src/
│   │   │   │   ├── main.rs     # App entry point
│   │   │   │   └── lib.rs      # Server integration
│   │   │   ├── Cargo.toml      # Tauri dependencies
│   │   │   └── tauri.conf.json # App configuration
│   │   └── package.json        # Added tauri:* scripts
├── run_tauri_dev.sh           # New: Dev mode script
└── build_macos_app.sh         # New: Build script
```

### Tauri Commands

The app exposes these commands to the frontend:

```typescript
// Get server status
const status = await invoke('get_server_status');
// Returns: { running: bool, port: number, url: string }

// Open data directory in Finder
await invoke('open_data_directory');

// Get app version
const version = await invoke('get_app_version');
```

### Configuration

App settings are in `src/datafold_node/static-react/src-tauri/tauri.conf.json`:

```json
{
  "productName": "DataFold",
  "version": "0.1.6",
  "identifier": "ai.datafold.app",
  "bundle": {
    "icon": ["icons/icon.icns"],
    "macOS": {
      "minimumSystemVersion": "10.13"
    }
  }
}
```

## 📝 Data Storage

The native app uses a **separate database** from the development environment:

- **Development:** `fold_db/data/`
- **Native App:** `~/.datafold/data/`

This keeps your development and production data separate.

## 🎨 Customization

### Changing the Icon

Replace the icon files in:
```
src/datafold_node/static-react/src-tauri/icons/
```

You need:
- `icon.icns` (macOS)
- `icon.ico` (Windows, if adding Windows support)
- PNG files for various sizes

### Window Settings

Edit `tauri.conf.json`:

```json
"windows": [{
  "title": "DataFold - Personal Database",
  "width": 1400,
  "height": 900,
  "minWidth": 1000,
  "minHeight": 700
}]
```

## 🚀 Distribution

### Building a Release

```bash
./build_macos_app.sh
```

### Code Signing (Optional)

For distribution outside the App Store, you'll need to sign the app:

1. Get an Apple Developer account
2. Create a Developer ID certificate
3. Update `tauri.conf.json`:
   ```json
   "macOS": {
     "signingIdentity": "Developer ID Application: Your Name"
   }
   ```

### Notarization (Optional)

For macOS 10.15+ distribution:

1. Sign the app (above)
2. Notarize with Apple:
   ```bash
   xcrun notarytool submit DataFold.dmg --apple-id YOUR_APPLE_ID --team-id YOUR_TEAM_ID
   ```

## 🐛 Troubleshooting

### Port Already in Use

If port 9001 is already in use, kill existing processes:
```bash
lsof -ti:9001 | xargs kill -9
```

### Build Fails

1. Clean the build:
   ```bash
   cd src/datafold_node/static-react
   rm -rf dist src-tauri/target
   cargo clean
   ```

2. Rebuild:
   ```bash
   npm run build
   npm run tauri:build
   ```

### App Won't Open (Security)

On first launch, macOS may block the app:

1. Go to **System Preferences → Security & Privacy**
2. Click "Open Anyway" for DataFold
3. Or run: `xattr -dr com.apple.quarantine /Applications/DataFold.app`

## 📚 Resources

- [Tauri Documentation](https://tauri.app/)
- [DataFold Web Server](./README.md)
- [Development Guide](./QUICK_START.md)

## 🎯 Next Steps

Future enhancements for the native app:

- [ ] Menu bar integration (File, Edit, View menus)
- [ ] Native notifications for query completion
- [ ] Auto-update system
- [ ] Windows and Linux builds
- [ ] System tray icon
- [ ] Keyboard shortcuts

---

**Enjoy DataFold as a native macOS app!** 🎉

