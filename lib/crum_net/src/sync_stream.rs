//! Crumble (CRyptographic gaMBLE)
//!
//! Mental Poker (1979) implemented using Boneh–Lynn–Shacham (BLS) cryptography.
//! Designed by the Sonia Code & Gemini AI & Open AI (2026)
//!
//! Copyright (c) 2026 Sonia Code; See LICENSE file for license details.

use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

pub type SyncSeq = u64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncStreamConfig {
    pub stream_id: String,
}

impl SyncStreamConfig {
    pub fn new(stream_id: impl Into<String>) -> Self {
        Self {
            stream_id: stream_id.into(),
        }
    }

    pub fn validate(&self) -> Result<(), SyncStreamError> {
        if self.stream_id.trim().is_empty() {
            return Err(SyncStreamError::InvalidConfig(
                "stream_id must not be empty",
            ));
        }

        Ok(())
    }
}

impl Default for SyncStreamConfig {
    fn default() -> Self {
        Self::new("crum-sync-stream")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncFrame {
    pub sequence: SyncSeq,
    pub kind: SyncFrameKind,
    pub line: String,
}

impl SyncFrame {
    pub fn new(sequence: SyncSeq, kind: SyncFrameKind, line: impl Into<String>) -> Self {
        Self {
            sequence,
            kind,
            line: line.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncFrameKind {
    Config,
    HandStarted,
    Shuffle,
    Deal,
    Blind,
    Bet,
    Fold,
    Check,
    Call,
    Raise,
    CommunityCards,
    Showdown,
    HandFinished,
    State,
    Note,
    DemoLine,
    DemoFinished,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderedSyncState {
    config: SyncStreamConfig,
    next_sequence: SyncSeq,
    lines: Vec<String>,
}

impl OrderedSyncState {
    pub fn new(config: SyncStreamConfig) -> Result<Self, SyncStreamError> {
        config.validate()?;

        Ok(Self {
            config,
            next_sequence: 0,
            lines: Vec::new(),
        })
    }

    pub fn config(&self) -> &SyncStreamConfig {
        &self.config
    }

    pub const fn next_sequence(&self) -> SyncSeq {
        self.next_sequence
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn make_frame(&self, kind: SyncFrameKind, line: impl Into<String>) -> SyncFrame {
        SyncFrame::new(self.next_sequence, kind, line)
    }

    pub fn emit_frame(&mut self, kind: SyncFrameKind, line: impl Into<String>) -> SyncFrame {
        let frame = self.make_frame(kind, line);
        self.lines.push(frame.line.clone());
        self.next_sequence += 1;
        frame
    }

    pub fn apply_frame(&mut self, frame: SyncFrame) -> Result<(), SyncStreamError> {
        if frame.sequence != self.next_sequence {
            return Err(SyncStreamError::OutOfOrder {
                stream_id: self.config.stream_id.clone(),
                expected: self.next_sequence,
                actual: frame.sequence,
            });
        }

        self.lines.push(frame.line);
        self.next_sequence += 1;
        Ok(())
    }

    pub fn apply_frames<I>(&mut self, frames: I) -> Result<(), SyncStreamError>
    where
        I: IntoIterator<Item = SyncFrame>,
    {
        for frame in frames {
            self.apply_frame(frame)?;
        }

        Ok(())
    }

    pub fn apply_demo_frame(
        &mut self,
        frame: SyncFrame,
    ) -> Result<Option<String>, SyncStreamError> {
        let output = match frame.kind {
            SyncFrameKind::DemoLine => Some(frame.line.clone()),
            SyncFrameKind::DemoFinished => None,
            _ => Some(frame.line.clone()),
        };

        self.apply_frame(frame)?;
        Ok(output)
    }
}

pub fn encode_frame_line(frame: &SyncFrame) -> Result<String, SyncStreamError> {
    let mut encoded = serde_json::to_string(frame)?;
    encoded.push('\n');
    Ok(encoded)
}

pub fn decode_frame_line(line: &str) -> Result<SyncFrame, SyncStreamError> {
    let trimmed = line.trim_end_matches(['\r', '\n']);

    if trimmed.is_empty() {
        return Err(SyncStreamError::EmptyFrame);
    }

    Ok(serde_json::from_str(trimmed)?)
}

pub fn encode_frame_lines(frames: &[SyncFrame]) -> Result<String, SyncStreamError> {
    let mut encoded = String::new();

    for frame in frames {
        encoded.push_str(&encode_frame_line(frame)?);
    }

    Ok(encoded)
}

pub fn decode_frame_lines(lines: &str) -> Result<Vec<SyncFrame>, SyncStreamError> {
    lines
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(decode_frame_line)
        .collect()
}

#[derive(Debug)]
pub enum SyncStreamError {
    InvalidConfig(&'static str),
    EmptyFrame,
    OutOfOrder {
        stream_id: String,
        expected: SyncSeq,
        actual: SyncSeq,
    },
    Json(serde_json::Error),
}

impl fmt::Display for SyncStreamError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(message) => write!(formatter, "invalid sync config: {message}"),
            Self::EmptyFrame => write!(formatter, "empty sync frame"),
            Self::OutOfOrder {
                stream_id,
                expected,
                actual,
            } => {
                write!(
                    formatter,
                    "sync frame out of order for {stream_id}: expected {expected}, received {actual}"
                )
            }
            Self::Json(error) => write!(formatter, "sync JSON error: {error}"),
        }
    }
}

impl Error for SyncStreamError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for SyncStreamError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_frames_in_sequence() {
        let mut state = OrderedSyncState::new(SyncStreamConfig::new("test")).expect("valid config");

        state
            .apply_frame(SyncFrame::new(0, SyncFrameKind::HandStarted, "hand 1"))
            .expect("seq 0");
        state
            .apply_frame(SyncFrame::new(1, SyncFrameKind::Blind, "small blind"))
            .expect("seq 1");

        assert_eq!(state.next_sequence(), 2);
        assert_eq!(
            state.lines(),
            &["hand 1".to_string(), "small blind".to_string()]
        );
    }

    #[test]
    fn rejects_out_of_order_frames() {
        let mut state = OrderedSyncState::new(SyncStreamConfig::new("test")).expect("valid config");

        let err = state
            .apply_frame(SyncFrame::new(1, SyncFrameKind::Deal, "deal"))
            .expect_err("seq 1 before seq 0");

        assert!(matches!(
            err,
            SyncStreamError::OutOfOrder {
                expected: 0,
                actual: 1,
                ..
            }
        ));
        assert_eq!(state.next_sequence(), 0);
    }

    #[test]
    fn round_trips_newline_json_frames() {
        let frames = vec![
            SyncFrame::new(0, SyncFrameKind::Shuffle, "p0 shuffled"),
            SyncFrame::new(1, SyncFrameKind::Showdown, "p0 wins"),
        ];

        let encoded = encode_frame_lines(&frames).expect("encode");
        assert!(encoded.ends_with('\n'));
        assert_eq!(decode_frame_lines(&encoded).expect("decode"), frames);
    }
}
