# Parakeet TDT Transcription App - Specification

## 1. Project Overview

- **Project Name**: Parakeet TDT Transcription App
- **Project Type**: Cross-platform desktop application (Tauri + React)
- **Core Functionality**: Audio transcription app using NVIDIA Parakeet TDT 0.6B v2 model with a clean, modern interface
- **Target Users**: Users who need to transcribe audio files (mp4, mkv, ogg, mp3) to text

## 2. Technical Stack

### Backend (Rust)
- **Framework**: Tauri v2
- **ML Library**: parakeet-rs (NVIDIA Parakeet TDT model)
- **Audio Processing**: symphonia (audio decoding)
- **Build Targets**: Linux, Windows (with DirectML, CPU, WebGPU support)

### Frontend
- **Framework**: React + TypeScript
- **UI**: Clean light interface with file browser/drag-and-drop

## 3. UI/UX Specification

### Layout Structure
- **Container**: Centered, max-width 800px, responsive
- **Sections** (top to bottom):
  1. Header with app title
  2. File browser / drag-and-drop area
  3. Action buttons row
  4. Transcription display area

### Visual Design

#### Color Palette
- **Background**: #FFFFFF (white)
- **Surface**: #F8F9FA (light gray for cards)
- **Primary**: #3B82F6 (blue - action buttons)
- **Primary Hover**: #2563EB (darker blue)
- **Text Primary**: #1F2937 (dark gray)
- **Text Secondary**: #6B7280 (medium gray)
- **Border**: #E5E7EB (light border)
- **Success**: #10B981 (green)
- **Error**: #EF4444 (red)
- **Drop Zone Active**: #EFF6FF (light blue background when dragging)

#### Typography
- **Font Family**: "Inter", system-ui, -apple-system, sans-serif
- **Title**: 24px, font-weight 600
- **Body**: 14px, font-weight 400
- **Button Text**: 14px, font-weight 500
- **Caption/Time**: 12px, font-weight 400

#### Spacing
- **Container Padding**: 24px
- **Section Gap**: 20px
- **Button Gap**: 12px
- **Inner Padding**: 16px

### Components

#### 1. Header
- App icon (optional) + Title: "Parakeet TDT"
- Subtitle: "Audio Transcription"

#### 2. File Drop Zone
- **Size**: Full width, 160px height
- **Border**: 2px dashed #E5E7EB (idle), 2px dashed #3B82F6 (drag-over)
- **Background**: #F8F9FA (idle), #EFF6FF (drag-over)
- **Border Radius**: 12px
- **Content**:
  - Icon: Upload/file icon (centered)
  - Text: "Drag & drop audio file or click to browse"
  - Supported formats: "Supports: MP4, MKV, OGG, MP3"
- **States**:
  - Idle: Default dashed border
  - Hover: Slightly different background
  - Drag-over: Blue border, light blue background
  - File selected: Show filename, green checkmark

#### 3. Action Buttons Row
- **Layout**: Horizontal flex, gap 12px, align center
- **Buttons**:
  1. **Transcribe Button**: Primary blue, "Transcribe"
     - Width: Auto (min 120px)
     - Height: 44px
     - Border Radius: 8px
     - Disabled when no file selected or transcribing
  2. **Save as Text Button**: Secondary, "Save as Text"
     - Same dimensions as Transcribe
     - Disabled when no transcription result
  3. **Copy to Clipboard Button**: Secondary, "Copy"
     - Same dimensions as Transcribe
     - Disabled when no transcription result

#### 4. Transcription Display Area
- **Layout**: Below buttons, full width
- **Background**: #F8F9FA
- **Border Radius**: 12px
- **Padding**: 16px
- **Min Height**: 200px
- **Content**:
  - Placeholder text: "Transcription will appear here..."
  - Result: Display transcription text
- **Scroll**: Auto for long text

#### 5. Transcription Time Display
- **Position**: Next to other buttons (same row)
- **Format**: "Time: X.XXs" or "Time: Xm Xs"
- **Style**: Medium gray text, monospace font for numbers

### Animations
- **Button hover**: 150ms ease background-color transition
- **Drop zone drag**: 200ms ease border/background transition
- **Transcription loading**: Pulse animation on button

## 4. Functionality Specification

### Core Features

#### F1: File Selection
- Click to open native file dialog
- Drag and drop support
- Filter: mp4, mkv, ogg, mp3, wav, flac, m4a
- Display selected filename

#### F2: Audio Decoding
- Use symphonia to decode audio
- Convert to WAV 16kHz mono for parakeet-rs
- Support any sample rate and channel count

#### F3: Transcription
- Load parakeet-tdt-0.6b-v2 from ./tdt.int8/ folder
- Execution providers (configurable at build time):
  - Windows: DirectML, CPU, WebGPU
  - Linux: CPU, WebGPU
- Show transcription time in UI

#### F4: Save as Text
- Native save dialog
- Default extension: .txt
- Save to OS user-selected location

#### F5: Copy to Clipboard
- Copy transcription text to system clipboard
- Show success feedback (button text changes briefly)

### User Flow
1. User opens app
2. User selects audio file (drag/drop or file dialog)
3. User clicks "Transcribe"
4. App shows loading state
5. App displays transcription result and time
6. User can save as text or copy to clipboard

### Edge Cases
- No file selected: Disable buttons
- Invalid audio format: Show error message
- Transcription error: Show error message with details
- Empty transcription: Display "No speech detected"
- Long audio: Process in chunks to prevent memory issues

## 5. Model Configuration

### Model Path
- `./tdt.int8/` in project root
- Required files:
  - decoder_joint-model.int8.onnx
  - encoder-model.int8.onnx
  - encoder-model.onnx.data
  - tokenizer.json
  - vocab.txt

### Model Type
- Parakeet TDT (Transducer with Timestamp) v2
- Quantized: INT8

## 6. Build Configuration

### Windows
- Targets: x86_64-pc-windows-msvc
- Providers: DirectML (default), CPU, WebGPU
- Bundler: NSIS installer

### Linux
- Targets: x86_64-unknown-linux-gnu
- Providers: CPU, WebGPU

## 7. Acceptance Criteria

### Visual Checkpoints
- [ ] Clean light interface loads
- [ ] File drop zone visible with dashed border
- [ ] All three buttons visible in a row
- [ ] Time display visible next to buttons
- [ ] Transcription area visible below buttons

### Functional Checkpoints
- [ ] Can select audio file via click
- [ ] Can select audio file via drag-and-drop
- [ ] Transcribe button initiates transcription
- [ ] Transcription result displays in text area
- [ ] Time is displayed after transcription
- [ ] Save as text saves to file
- [ ] Copy to clipboard copies text

### Build Checkpoints
- [ ] Windows .exe builds successfully
- [ ] Linux binary builds successfully