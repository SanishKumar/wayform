use std::io::{self, BufRead, Write};

use serde::Serialize;

const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

enum Frame {
    Json(Vec<u8>),
    Oversized,
}

/// Drain oversized frames through LF/EOF without retaining their remaining bytes.
fn read_frame(reader: &mut impl BufRead) -> io::Result<Option<Frame>> {
    let mut bytes = Vec::new();
    let mut oversized = false;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(if oversized {
                Some(Frame::Oversized)
            } else if bytes.is_empty() {
                None
            } else {
                Some(Frame::Json(bytes))
            });
        }
        let newline = available.iter().position(|&byte| byte == b'\n');
        let count = newline.unwrap_or(available.len());
        if !oversized {
            if count > MAX_FRAME_BYTES - bytes.len() {
                oversized = true;
                bytes.clear();
            } else {
                bytes.extend_from_slice(&available[..count]);
            }
        }
        reader.consume(count + usize::from(newline.is_some()));
        if newline.is_some() {
            return Ok(Some(if oversized {
                Frame::Oversized
            } else {
                Frame::Json(bytes)
            }));
        }
    }
}

pub fn serve<R: Serialize>(
    process: impl Fn(&[u8]) -> R,
    oversized: impl Fn() -> R,
) -> io::Result<()> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    while let Some(frame) = read_frame(&mut input)? {
        let response = match frame {
            Frame::Json(bytes) => process(&bytes),
            Frame::Oversized => oversized(),
        };
        serde_json::to_writer(&mut output, &response).map_err(io::Error::other)?;
        output.write_all(b"\n")?;
        output.flush()?;
    }
    Ok(())
}
