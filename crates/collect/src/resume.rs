//! A session the collector has already read, kept so the next run can skip it.
//!
//! The terminal index says which session file moved. The file itself is still
//! the record of tokens and moments, and a day is the sum of every session that
//! touched it, so a run that opened only the files that moved would publish a
//! partial day and the fuller reading already on disk would win. What is kept
//! here is each file's own contribution. The run replaces the ones that moved
//! and folds all of them back into the day.

use std::{
    collections::HashSet,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use github_personal_stats_core::TokenUsage;

use crate::error::CollectError;

const MAGIC: &[u8; 8] = b"gpscodex";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Moment {
    pub second: i64,
    pub day: String,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenRow {
    pub day: String,
    pub model: String,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Captured {
    pub updated_at_ms: i64,
    pub path: String,
    pub moments: Vec<Moment>,
    pub tokens: Vec<TokenRow>,
}

pub(crate) fn directory(state_dir: &Path) -> PathBuf {
    state_dir.join("resume").join("codex")
}

pub(crate) fn file_id(path: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in path.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

pub(crate) fn load(state_dir: &Path, path: &str) -> Option<Captured> {
    read_stored(&session_file(state_dir, path)).filter(|captured| captured.path == path)
}

pub(crate) fn read_stored(path: &Path) -> Option<Captured> {
    let bytes = fs::read(path).ok()?;
    decode(&bytes)
}

pub(crate) fn store(state_dir: &Path, captured: &Captured) -> Result<(), CollectError> {
    let directory = directory(state_dir);
    fs::create_dir_all(&directory).map_err(|error| unwritable(&directory, error))?;
    let destination = session_file(state_dir, &captured.path);
    let temporary = directory.join(format!(".{}.tmp", file_id(&captured.path)));
    {
        let mut file = File::create(&temporary).map_err(|error| unwritable(&temporary, error))?;
        file.write_all(&encode(captured))
            .map_err(|error| unwritable(&temporary, error))?;
    }
    fs::rename(&temporary, &destination).map_err(|error| unwritable(&destination, error))?;
    Ok(())
}

/// Drops contributions whose session is no longer in the index. A file left
/// behind would keep counting a session the index has forgotten.
pub(crate) fn retain(state_dir: &Path, kept: &HashSet<String>) -> Result<(), CollectError> {
    let directory = directory(state_dir);
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(unwritable(&directory, error)),
    };
    for entry in entries {
        let entry = entry.map_err(|error| unwritable(&directory, error))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.ends_with(".tmp") || kept.contains(name.trim_end_matches(".bin")) {
            continue;
        }
        if name.ends_with(".bin") {
            fs::remove_file(entry.path()).map_err(|error| unwritable(&entry.path(), error))?;
        }
    }
    Ok(())
}

fn session_file(state_dir: &Path, path: &str) -> PathBuf {
    directory(state_dir).join(format!("{}.bin", file_id(path)))
}

fn encode(captured: &Captured) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    write_i64(&mut out, captured.updated_at_ms);
    write_str(&mut out, &captured.path);
    write_u32(&mut out, captured.moments.len() as u32);
    for moment in &captured.moments {
        write_i64(&mut out, moment.second);
        write_str(&mut out, &moment.day);
        write_str(&mut out, &moment.model);
    }
    write_u32(&mut out, captured.tokens.len() as u32);
    for row in &captured.tokens {
        write_str(&mut out, &row.day);
        write_str(&mut out, &row.model);
        write_u64(&mut out, row.usage.input);
        write_u64(&mut out, row.usage.output);
        write_u64(&mut out, row.usage.cached);
    }
    out
}

fn decode(bytes: &[u8]) -> Option<Captured> {
    let mut cursor = Cursor::new(bytes);
    if cursor.take(MAGIC.len())? != MAGIC {
        return None;
    }
    let updated_at_ms = cursor.i64()?;
    let path = cursor.string()?;
    let moment_count = cursor.u32()?;
    let mut moments = Vec::with_capacity(moment_count as usize);
    for _ in 0..moment_count {
        moments.push(Moment {
            second: cursor.i64()?,
            day: cursor.string()?,
            model: cursor.string()?,
        });
    }
    let token_count = cursor.u32()?;
    let mut tokens = Vec::with_capacity(token_count as usize);
    for _ in 0..token_count {
        tokens.push(TokenRow {
            day: cursor.string()?,
            model: cursor.string()?,
            usage: TokenUsage {
                input: cursor.u64()?,
                output: cursor.u64()?,
                cached: cursor.u64()?,
            },
        });
    }
    cursor.done().then_some(Captured {
        updated_at_ms,
        path,
        moments,
        tokens,
    })
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn take(&mut self, length: usize) -> Option<&'a [u8]> {
        let end = self.at.checked_add(length)?;
        let slice = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(slice)
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }

    fn i64(&mut self) -> Option<i64> {
        Some(i64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }

    fn string(&mut self) -> Option<String> {
        let length = self.u32()? as usize;
        if length > 1024 * 1024 {
            return None;
        }
        let bytes = self.take(length)?;
        std::str::from_utf8(bytes).ok().map(str::to_owned)
    }

    fn done(self) -> bool {
        self.at == self.bytes.len()
    }
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_str(out: &mut Vec<u8>, value: &str) {
    write_u32(out, value.len() as u32);
    out.extend_from_slice(value.as_bytes());
}

fn unwritable(path: &Path, error: std::io::Error) -> CollectError {
    CollectError::Unreadable {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let directory = std::env::temp_dir().join(format!(
            "gps-resume-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn sample() -> Captured {
        Captured {
            updated_at_ms: 1_700_000_000_000,
            path: "/tmp/sessions/one.jsonl".to_owned(),
            moments: vec![Moment {
                second: 1_700_000_000,
                day: "2026-08-20".to_owned(),
                model: "gpt".to_owned(),
            }],
            tokens: vec![TokenRow {
                day: "2026-08-20".to_owned(),
                model: "gpt".to_owned(),
                usage: TokenUsage {
                    input: 3,
                    output: 4,
                    cached: 5,
                },
            }],
        }
    }

    #[test]
    fn a_stored_session_roundtrips_and_a_different_path_does_not_read_it() {
        let state = scratch();
        let captured = sample();
        store(&state, &captured).unwrap();

        assert_eq!(load(&state, &captured.path), Some(captured));
        assert_eq!(load(&state, "/tmp/sessions/other.jsonl"), None);
    }

    #[test]
    fn a_truncated_file_is_not_a_session() {
        let state = scratch();
        let captured = sample();
        store(&state, &captured).unwrap();
        let path = session_file(&state, &captured.path);
        let bytes = fs::read(&path).unwrap();
        fs::write(&path, &bytes[..bytes.len() - 1]).unwrap();

        assert_eq!(load(&state, &captured.path), None);
    }

    #[test]
    fn sessions_the_index_dropped_are_removed() {
        let state = scratch();
        store(&state, &sample()).unwrap();
        let mut other = sample();
        other.path = "/tmp/sessions/two.jsonl".to_owned();
        store(&state, &other).unwrap();

        let mut kept = HashSet::new();
        kept.insert(file_id(&other.path));
        retain(&state, &kept).unwrap();

        assert_eq!(load(&state, &sample().path), None);
        assert_eq!(
            load(&state, &other.path).map(|held| held.path),
            Some(other.path)
        );
    }
}
