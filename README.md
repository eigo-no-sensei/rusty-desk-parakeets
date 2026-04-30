# Parakeet TDT Transcription App

A cross-platform desktop application for audio transcription using NVIDIA Parakeet TDT 0.6B v2 model, built with Tauri + React.

## Features

- **Clean Light Interface**: Modern, intuitive UI with file browser/drag-and-drop
- **Multi-format Audio Support**: Supports MP4, MKV, OGG, MP3, WAV, FLAC, M4A via Symphonia
- **Automatic Conversion**: Converts audio to 16kHz mono for optimal transcription
- **Model**: Loads parakeet-tdt-0.6b-v2 from `./tdt.int8/` folder
- **Cross-platform**: Builds for Linux and Windows
- **Windows Backends**: DirectML, CPU, WebGPU support
- **Export Options**: Save as text file or copy to clipboard

- <a href="https://www.flaticon.com/free-icons/parrot" title="parrot icons">Parrot icons created by B.Genesis - Flaticon</a>

## UI Layout

```
┌─────────────────────────────────────────┐
│         Parakeet TDT                   │
│         Audio Transcription              │
├─────────────────────────────────────────┤
│  ┌───────────────────────────────┐    │
│  │   File Drop Zone (160px)        │    │
│  │   Drag & drop or click to     │    │
│  │   browse                  │    │
│  │   Supports: MP4, MKV...   │    │
│  └───────────────────────────────┘    │
├─────────────────────────────────────────┤
│ [Transcribe] [Save as Text] [Copy]      │
│                              Time: Xs │
├─────────────────────────────────────────┤
│  Transcription output area...          │
└─────────────────────────────────────────┘
```

## Prerequisites

### System Dependencies

**Ubuntu/Debian:**
```bash
sudo apt-get update
sudo apt-get install -y pkg-config libssl-dev libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev
```

**macOS:**
```bash
brew install pkg-config openssl
```

**Windows:**
Install Visual Studio Build Tools with C++ workload.

### Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

### Node.js
```bash
# Using nvm recommended
nvm install 20
nvm use 20
```

## Project Structure

```
parakeet-tdt-app/
├── SPEC.md                    # Specification document
├── README.md                # This file
├── package.json             # Frontend dependencies
├── vite.config.ts          # Vite configuration
├── tsconfig.json          # TypeScript config
├── index.html             # HTML entry point
├── src/                  # React frontend
│   ├── main.tsx
│   ├── App.tsx
│   ├── styles.css
│   └── vite-env.d.ts
├── src-tauri/             # Tauri/Rust backend
│   ├── Cargo.toml        # Rust dependencies
│   ├── build.rs
│   ├── tauri.conf.json  # Tauri config
│   ├── capabilities/
│   │   └── default.json
│   └── src/
│       ├── main.rs     # Entry point
│       ├── lib.rs     # Library with commands
│       └── audio.rs  # Audio decoding
└── tdt.int8/          # Model files (add your own)
    ├── decoder_joint-model.int8.onnx
    ├── encoder-model.int8.onnx
    ├── encoder-model.onnx.data
    ├── tokenizer.json
    └── vocab.txt
```

## Building

### Build Frontend
```bash
npm install
npm run build
```

### Build Tauri App
```bash
cd src-tauri
cargo build --release
```

### Build Full Application
```bash
# Linux
npm run build
cd src-tauri && cargo build --release

# Windows (cross-compile or on Windows)
npm run build
cd src-tauri && cargo build --release --target x86_64-pc-windows-msvc
```

## Model Setup

1. Download the parakeet-tdt-0.6b-v2 INT8 quantized model
2. Place all model files in `./tdt.int8/` folder:
   - `decoder_joint-model.int8.onnx`
   - `encoder-model.int8.onnx`
   - `encoder-model.onnx.data` (if separate)
   - `tokenizer.json`
   - `vocab.txt`

## Usage

1. Launch the application
2. The model loads automatically on startup
3. Drag & drop an audio file or click to browse
4. Click "Transcribe" to transcribe
5. View transcription in the output area
6. Save as text or copy to clipboard

## Configuration

### Execution Providers (Windows)

The app uses CPU by default. For Windows with GPU:

- **DirectML**: For Windows GPUs (included in ort)
- **WebGPU**: For modern browsers/GPUs
- **CPU**: Fallback for all platforms

Edit `src-tauri/src/lib.rs` to change the provider:

```rust
let exec_config = ExecutionConfig::new()
    .with_execution_provider(ExecutionProvider::DirectML); // or WebGPU
```

### Chunk Duration

For long audio files, the model processes in chunks (default 10 seconds). Edit in `lib.rs`:

```rust
let chunk_duration = 10; // seconds
```

## Development

### Run in Development Mode
```bash
# Terminal 1: Frontend
npm run dev

# Terminal 2: Tauri
cd src-tauri && cargo run
```

### Hot Reload
```bash
npm run dev
cd src-tauri && cargo tauri dev
```

## Troubleshooting

### Missing System Libraries
If you see pkg-config errors, install the required system packages listed in Prerequisites.

### Model Not Found
Ensure all model files are in `./tdt.int8/` directory with correct names.

### WebView Not Loading (Linux)
Install WebKit2GTK:
```bash
sudo apt-get install libwebkit2gtk-4.1-dev
```

### GPU Not Available
The app falls back to CPU. For GPU acceleration, ensure your GPU drivers are installed and use the appropriate execution provider.

## License

MIT License
