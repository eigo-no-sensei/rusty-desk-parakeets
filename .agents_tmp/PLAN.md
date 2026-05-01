# 1. OBJECTIVE

Add YouTube URL transcription capability to the Parakeet TDT app. This new feature will:
- Accept a YouTube (or any yt-dlp supported) URL from the user
- Download mid-quality audio using yt-dlp to a temporary file
- Decode and transcribe the audio using the existing pipeline
- Display the transcription result in the same UI

- Add a checkbox option to keep the downloaded file (saves to user's downloads folder)
- Add a checkbox option to download video (not just audio) - saves to same location as kept files

The existing file-based transcription functionality must remain unchanged.

# 2. CONTEXT SUMMARY

The Parakeet TDT app is a Tauri-based desktop application for audio transcription using the parakeet-rs library.

**Backend files:**
- `src-tauri/src/commands.rs` - Contains existing Tauri commands: `init_model`, `transcribe_audio`, `get_last_transcript`
- `src-tauri/src/lib.rs` - Re-exports commands and defines `AppState` and `TranscriptionResult`
- `src-tauri/src/main.rs` - Registers commands with Tauri invoke handler

**Frontend files:**
- `src/App.tsx` - React UI with file drop zone and transcription controls
- `src/styles.css` - CSS styling for the interface

**Dependencies needed:**
- `tempfile` crate (already present in Cargo.toml)
- `url` crate - optional, can use built-in Rust URL parsing

**External tool (bundled):**
- `yt-dlp` - Bundled with the application in resources folder

# 3. APPROACH OVERVIEW

**Chosen approach:** Add a new Tauri command `transcribe_youtube` that:
1. Gets yt-dlp path from bundled resources
2. Validates the provided URL
3. Creates a temporary directory for the download
4. Spawns yt-dlp to download mid-quality audio OR video as WAV format (configurable)
5. Finds the downloaded file in the temp directory
6. If keep_file or download_video is true, move file to user's downloads folder
7. Reuses the existing `decode_audio` function from `audio.rs`
8. Applies the same chunking and transcription logic as `transcribe_audio`
9. Cleans up the temporary directory automatically when done

**Why this approach:**
- Maximizes code reuse - the transcription logic is identical to existing `transcribe_audio`
- Uses Rust's `tempfile` crate for automatic cleanup
- Error handling is consistent with existing patterns
- No changes needed to the core audio decoding pipeline

**Alternative considered and rejected:**
- Using yt-dlp with stdout output - more complex error handling and less reliable for large files
- Bundling yt-dlp inside Tauri resources - adds complexity to the build process, easier to require as external dependency

# 4. IMPLEMENTATION STEPS

## Step 1: Add yt-dlp to Tauri bundle resources
**Goal:** Bundle yt-dlp executable with the app
**Method:** Add yt-dlp to resources in Tauri config and ensure it's included in the build
**Reference:** `/workspace/project/rusty-desk-parakeets/src-tauri/tauri.conf.json`

## Step 2: Verify dependencies in Cargo.toml
**Goal:** Confirm required dependencies are present
**Method:** Verify `tempfile` crate is already in Cargo.toml (it is, version 3)
**Reference:** `/workspace/project/rusty-desk-parakeets/src-tauri/Cargo.toml`

## Step 2: Add transcribe_youtube command in commands.rs
**Goal:** Implement the new YouTube transcription command
**Method:** Add a new async function `transcribe_youtube` that accepts:
- `url: String` - The YouTube URL
- `keep_file: bool` (optional) - Whether to keep the downloaded file after transcription
- `download_video: bool` (optional) - Whether to download video instead of just audio

The function will:
- Get yt-dlp path from bundled resources (exe_dir/resources/yt-dlp)
- Validate the URL input
- Creates a temporary directory using tempfile
- Spawns yt-dlp to download mid-quality audio or video
- Finds the downloaded file
- If keep_file or download_video is true, move file to user's downloads folder
- Decodes using existing `decode_audio`
- Applies the same chunking and transcription loop as `transcribe_audio`
- Returns TranscriptionResult

**Reference:** `/workspace/project/rusty-desk-parakeets/src-tauri/src/commands.rs`

## Step 3: Update lib.rs to re-export transcribe_youtube
**Goal:** Make the new command accessible from main.rs
**Method:** Add `transcribe_youtube` to the public re-exports in lib.rs
**Reference:** `/workspace/project/rusty-desk-parakeets/src-tauri/src/lib.rs`

## Step 4: Register command in main.rs
**Goal:** Make the new command available to the frontend
**Method:** Add `transcribe_youtube` to the generate_handler! macro
**Reference:** `/workspace/project/rusty-desk-parakeets/src-tauri/src/main.rs`

## Step 5: Update frontend App.tsx with URL input UI
**Goal:** Add YouTube URL input and button to the frontend
**Method:** Add:
- State for youtubeUrl and isYoutubeLoading
- Checkbox to keep downloaded file (optional)
- Checkbox to download video instead of just audio
- YouTube icon component
- URL input field with styling
- "Transcribe from URL" button that calls the new command
- Loading state handling

**Reference:** `/workspace/project/rusty-desk-parakeets/src/App.tsx`

## Step 5b: Update backend command to support keep_file and download_video options
**Goal:** Add optional parameters to control download behavior
**Method:** Add parameters to transcribe_youtube command:
- `keep_file: bool` - If true, move downloaded file to user's downloads folder; if false, delete after transcription (default: false)
- `download_video: bool` - If true, download video file (mid-quality); if false, download audio only (default: false)
  - When download_video is true, saves to downloads folder

## Step 6: Add CSS styles for URL input section
**Goal:** Style the new UI components consistently
**Method:** Add CSS for:
- URL input field
- Section divider ("OR")
- Input group layout
**Reference:** `/workspace/project/rusty-desk-parakeets/src/styles.css`

# 5. TESTING AND VALIDATION

**Note:** yt-dlp is bundled with the application in the resources folder.

**To validate the implementation:**

1. **Build the application:**
   - Run `cargo build --release` in src-tauri directory
   - Verify compilation succeeds without errors

2. **Test the frontend:**
   - Run `npm run tauri dev` to start the development server
   - Verify the UI displays:
     - Existing file drop zone
     - New "OR" divider
     - YouTube URL input field
     - "Transcribe from URL" button

3. **Functional test:**
   - Enter a valid YouTube URL in the input field
   - Click "Transcribe from URL"
   - Verify:
     - Loading spinner appears during download
     - Audio is downloaded using yt-dlp
     - Transcription completes successfully
     - Result appears in the transcription area

4. **Error handling test:**
   - Enter an invalid URL → should show validation error
   - Click button without URL → button should be disabled
   - If bundled yt-dlp is missing → should show error message
