import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';

// Types
interface TranscriptionResult {
  text: string;
  duration_secs: number;
  success: boolean;
  error: string | null;
}

// Icons
const UploadIcon = () => (
  <svg className="drop-zone-icon" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
    <path strokeLinecap="round" strokeLinejoin="round" d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5m-13.5-9L12 3m0 0l4.5 4.5M12 3v13.5" />
  </svg>
);

const CheckIcon = () => (
  <svg className="drop-zone-icon" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
    <path strokeLinecap="round" strokeLinejoin="round" d="M9 12.75L11.25 15 15 9.75M21 12c0-1.05-.41-2.04-1.14-2.75C19.18 8.08 18.16 7.5 17 7.5c-1.58 0-2.92.92-3.5 1.75A2.252 2.252 0 0012.95 7.5c-1.16 0-2.18.58-2.86 1.75C9.41 9.96 9 10.95 9 12c0 1.05.41 2.04 1.14 2.75C10.82 15.92 11.84 16.5 13 16.5c1.58 0 2.92-.92 3.5-1.75a2.252 2.252 0 002.55 0c1.16 0 2.18-.58 2.86-1.75C20.59 14.04 21 13.05 21 12z" />
  </svg>
);

function App() {
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [transcription, setTranscription] = useState<string | null>(null);
  const [transcriptionTime, setTranscriptionTime] = useState<number | null>(null);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [isModelLoading, setIsModelLoading] = useState(false);
  const [modelStatus, setModelStatus] = useState<string | null>(null);
  const [isDragOver, setIsDragOver] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copySuccess, setCopySuccess] = useState(false);

  // Initialize model on mount
  useEffect(() => {
    const initModel = async () => {
      setIsModelLoading(true);
      setModelStatus('Loading model...');
      try {
        const result = await invoke<TranscriptionResult>('init_model');
        if (result.success) {
          setModelStatus('Model loaded');
          setTimeout(() => setModelStatus(null), 2000);
        } else {
          setError(result.error || 'Failed to initialize model');
        }
      } catch (e) {
        console.error('Model init error:', e);
        setError(String(e));
      } finally {
        setIsModelLoading(false);
      }
    };
    initModel();
  }, []);

  // Handle file selection
  const handleFileSelect = useCallback(async () => {
    try {
      const file = await open({
        multiple: false,
        filters: [{
          name: 'Audio Files',
          extensions: ['mp4', 'mkv', 'ogg', 'mp3', 'wav', 'flac', 'm4a']
        }]
      });
      if (file) {
        setSelectedFile(file as string);
        setTranscription(null);
        setTranscriptionTime(null);
        setError(null);
      }
    } catch (e) {
      console.error('File select error:', e);
      setError(String(e));
    }
  }, []);

  // Handle drag and drop
  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOver(true);
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOver(false);
  }, []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOver(false);

    const files = e.dataTransfer.files;
    if (files.length > 0) {
      const file = files[0];
      const validExtensions = ['mp4', 'mkv', 'ogg', 'mp3', 'wav', 'flac', 'm4a'];
      const ext = file.name.split('.').pop()?.toLowerCase();
      
      if (ext && validExtensions.includes(ext)) {
        setSelectedFile(file.name);
        setTranscription(null);
        setTranscriptionTime(null);
        setError(null);
      } else {
        setError(`Invalid file format. Supported: ${validExtensions.join(', ')}`);
      }
    }
  }, []);

  // Handle transcription
  const handleTranscribe = useCallback(async () => {
    if (!selectedFile) return;

    setIsTranscribing(true);
    setError(null);
    setTranscription(null);
    setTranscriptionTime(null);

    try {
      const result = await invoke<TranscriptionResult>('transcribe_audio', {
        audioPath: selectedFile
      });

      if (result.success) {
        setTranscription(result.text);
        setTranscriptionTime(result.duration_secs);
      } else {
        setError(result.error || 'Transcription failed');
      }
    } catch (e) {
      console.error('Transcribe error:', e);
      setError(String(e));
    } finally {
      setIsTranscribing(false);
    }
  }, [selectedFile]);

  // Handle save as text
  const handleSaveAsText = useCallback(async () => {
    if (!transcription) return;

    try {
      const filePath = await save({
        filters: [{
          name: 'Text Files',
          extensions: ['txt']
        }],
        defaultPath: 'transcription.txt'
      });

      if (filePath) {
        await writeTextFile(filePath, transcription);
      }
    } catch (e) {
      console.error('Save error:', e);
      setError(String(e));
    }
  }, [transcription]);

  // Handle copy to clipboard
  const handleCopyToClipboard = useCallback(async () => {
    if (!transcription) return;

    try {
      await writeText(transcription);
      setCopySuccess(true);
      setTimeout(() => setCopySuccess(false), 2000);
    } catch (e) {
      console.error('Copy error:', e);
      setError(String(e));
    }
  }, [transcription]);

  // Format time display
  const formatTime = (seconds: number): string => {
    if (seconds < 60) {
      return `${seconds.toFixed(2)}s`;
    }
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}m ${secs}s`;
  };

  return (
    <div className="app-container">
      {/* Header */}
      <header className="header">
        <h1 className="header-title">Parakeet TDT</h1>
        <p className="header-subtitle">Audio Transcription</p>
      </header>

      {/* Error Message */}
      {error && (
        <div className="error-message">
          {error}
        </div>
      )}

      {/* Model Status */}
      {modelStatus && (
        <div className="error-message" style={{ backgroundColor: '#eff6ff', color: '#1d4ed8' }}>
          {modelStatus}
        </div>
      )}

      {/* File Drop Zone */}
      <div
        className={`drop-zone ${isDragOver ? 'active' : ''} ${selectedFile ? 'has-file' : ''}`}
        onClick={handleFileSelect}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
      >
        {selectedFile ? (
          <>
            <CheckIcon />
            <p className="drop-zone-filename">{selectedFile}</p>
          </>
        ) : (
          <>
            <UploadIcon />
            <p className="drop-zone-text">Drag & drop audio file or click to browse</p>
            <p className="drop-zone-formats">Supports: MP4, MKV, OGG, MP3</p>
          </>
        )}
      </div>

      {/* Action Buttons Row */}
      <div className="action-row">
        <button
          className="btn btn-primary"
          onClick={handleTranscribe}
          disabled={!selectedFile || isTranscribing || isModelLoading}
        >
          {isTranscribing ? (
            <>
              <span className="spinner"></span>
              Transcribing...
            </>
          ) : (
            'Transcribe'
          )}
        </button>

        <button
          className={`btn btn-secondary ${copySuccess ? 'btn-copy-success' : ''}`}
          onClick={handleSaveAsText}
          disabled={!transcription}
        >
          {copySuccess ? 'Saved!' : 'Save as Text'}
        </button>

        <button
          className={`btn btn-secondary ${copySuccess ? 'btn-copy-success' : ''}`}
          onClick={handleCopyToClipboard}
          disabled={!transcription}
        >
          {copySuccess ? 'Copied!' : 'Copy'}
        </button>

        {transcriptionTime !== null && (
          <span className="time-display">
            Time: {formatTime(transcriptionTime)}
          </span>
        )}
      </div>

      {/* Transcription Display Area */}
      <div className="transcription-area">
        {transcription ? (
          <p className="transcription-text">{transcription}</p>
        ) : (
          <p className="transcription-placeholder">
            Transcription will appear here...
          </p>
        )}
      </div>
    </div>
  );
}

export default App;