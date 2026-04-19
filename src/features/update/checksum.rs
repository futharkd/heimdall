use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

/// Parse the first hex digest from a `sha256sum`-style file (`hash  name` or `hash *name`).
pub fn parse_sha256sum_file(content: &str) -> Result<[u8; 32]> {
    let line = content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .with_context(|| "empty or invalid .sha256 file")?;

    let digest_part = line
        .split_whitespace()
        .next()
        .with_context(|| format!("could not parse digest from line: {line}"))?;

    parse_hex_digest(digest_part).with_context(|| format!("invalid digest in line: {line}"))
}

pub fn parse_hex_digest(text: &str) -> Result<[u8; 32]> {
    let trimmed = text.trim();
    if trimmed.len() != 64 {
        bail!("expected 64 hex chars for sha256, got {}", trimmed.len());
    }
    let mut out = [0u8; 32];
    for (index, chunk) in trimmed.as_bytes().chunks(2).enumerate() {
        if chunk.len() != 2 {
            bail!("invalid digest hex");
        }
        let hi = from_hex(chunk[0])?;
        let lo = from_hex(chunk[1])?;
        out[index] = (hi << 4) | lo;
    }
    Ok(out)
}

fn from_hex(byte: u8) -> Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => bail!("invalid hex digit"),
    }
}

pub fn hex_lower(digest: &[u8; 32]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn sha256_file(path: &Path) -> Result<[u8; 32]> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .with_context(|| format!("read {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().into())
}

pub fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::{hex_lower, parse_hex_digest, parse_sha256sum_file, sha256_bytes};

    #[test]
    fn parses_space_separated_line() {
        let content = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  heimdall-linux-amd64\n";
        let parsed = parse_sha256sum_file(content).expect("parse");
        assert_eq!(
            hex_lower(&parsed),
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
    }

    #[test]
    fn parses_star_separated_line() {
        let content = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb *heimdall-linux-amd64\n";
        let parsed = parse_sha256sum_file(content).expect("parse");
        assert_eq!(
            hex_lower(&parsed),
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        );
    }

    #[test]
    fn parse_hex_digest_roundtrip() {
        let hex = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let digest = parse_hex_digest(hex).expect("digest");
        assert_eq!(hex_lower(&digest), hex);
    }

    #[test]
    fn sha256_bytes_known_vector() {
        let digest = sha256_bytes(b"");
        assert_eq!(
            hex_lower(&digest),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
