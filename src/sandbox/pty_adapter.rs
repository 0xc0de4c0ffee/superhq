//! Adapters to bridge shuru-sdk's ShellWriter/ShellReader to std::io::Read/Write
//! so they can be used with gpui-terminal.

use shuru_sdk::{ShellEvent, ShellReader, ShellWriter};
use std::io::{self, Read, Write};
use tokio::runtime::Handle as TokioHandle;

/// Wraps ShellWriter to implement std::io::Write.
/// gpui-terminal writes keyboard input here → forwarded to shuru VM.
pub struct ShuruPtyWriter {
    inner: ShellWriter,
}

impl ShuruPtyWriter {
    pub fn new(writer: ShellWriter) -> Self {
        Self { inner: writer }
    }
}

impl Write for ShuruPtyWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner
            .send_input(buf)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Wraps ShellReader to implement std::io::Read.
/// gpui-terminal reads terminal output from here ← received from shuru VM.
///
/// Since ShellReader::recv() is async, we use a background thread that
/// receives events and buffers them for synchronous reads.
pub struct ShuruPtyReader {
    buffer: Vec<u8>,
    buf_pos: usize,
    rx: std::sync::mpsc::Receiver<Vec<u8>>,
}

impl ShuruPtyReader {
    /// Create a new reader that drains events from the ShellReader in a background task.
    pub fn new(mut reader: ShellReader, tokio_handle: TokioHandle) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();

        // Spawn a tokio task to read from the async ShellReader and
        // forward output bytes to the synchronous channel.
        std::thread::Builder::new()
            .name("shuru-pty-bridge".into())
            .spawn(move || {
                tokio_handle.block_on(async move {
                    while let Some(event) = reader.recv().await {
                        match event {
                            ShellEvent::Output(data) => {
                                if tx.send(data).is_err() {
                                    break;
                                }
                            }
                            ShellEvent::Exit(_) | ShellEvent::Error(_) => {
                                break;
                            }
                        }
                    }
                });
            })
            .expect("Failed to spawn shuru PTY bridge thread");

        Self {
            buffer: Vec::new(),
            buf_pos: 0,
            rx,
        }
    }
}

impl Read for ShuruPtyReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // If we have buffered data, return it first
        if self.buf_pos < self.buffer.len() {
            let available = &self.buffer[self.buf_pos..];
            let to_copy = available.len().min(buf.len());
            buf[..to_copy].copy_from_slice(&available[..to_copy]);
            self.buf_pos += to_copy;
            if self.buf_pos >= self.buffer.len() {
                self.buffer.clear();
                self.buf_pos = 0;
            }
            return Ok(to_copy);
        }

        // Block waiting for next chunk from the VM
        match self.rx.recv() {
            Ok(data) => {
                let to_copy = data.len().min(buf.len());
                buf[..to_copy].copy_from_slice(&data[..to_copy]);
                if to_copy < data.len() {
                    self.buffer = data;
                    self.buf_pos = to_copy;
                }
                Ok(to_copy)
            }
            Err(_) => Ok(0), // Channel closed = EOF
        }
    }
}

/// Wraps ShellWriter for resize operations.
pub struct ShuruPtyResizer {
    writer: ShellWriter,
}

impl ShuruPtyResizer {
    pub fn new(writer: ShellWriter) -> Self {
        Self { writer }
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        let _ = self.writer.resize(rows, cols);
    }
}
