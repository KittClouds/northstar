use memchr::memchr_iter;
use memmap2::Mmap;
use sha2::{Digest, Sha256};
use std::{fs::File, io, path::Path};

pub struct MappedSource {
    mmap: Mmap,
    line_starts: Vec<usize>,
}

impl MappedSource {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        // SAFETY: the campaign operates on immutable, hash-bound authority files.
        let mmap = unsafe { Mmap::map(&file)? };
        let mut line_starts = Vec::with_capacity(mmap.len() / 32);
        line_starts.push(0);
        line_starts.extend(
            memchr_iter(b'\n', &mmap)
                .map(|i| i + 1)
                .filter(|&i| i < mmap.len()),
        );
        Ok(Self { mmap, line_starts })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.mmap
    }

    pub fn sha256(&self) -> String {
        format!("{:x}", Sha256::digest(self.bytes()))
    }

    pub fn len(&self) -> usize {
        self.mmap.len()
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    pub fn text(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.bytes())
    }

    pub fn line_of(&self, needle: &str) -> Result<usize, String> {
        let offset = self
            .text()
            .map_err(|e| e.to_string())?
            .find(needle)
            .ok_or_else(|| format!("required source needle not found: {needle}"))?;
        Ok(self.line_starts.partition_point(|&start| start <= offset))
    }

    pub fn declared_inputs(&self) -> Result<Vec<String>, String> {
        let mut names = Vec::new();
        for line in self.text().map_err(|e| e.to_string())?.lines() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with("input ") || trimmed.starts_with("input group") {
                continue;
            }
            let declaration = trimmed
                .split_once('=')
                .map_or(trimmed, |(left, _)| left)
                .trim();
            let name = declaration
                .split_whitespace()
                .last()
                .ok_or_else(|| format!("malformed input declaration: {line}"))?;
            names.push(name.to_owned());
        }
        Ok(names)
    }

    pub fn buffer_bindings(&self) -> Result<Vec<(u8, String)>, String> {
        let mut bindings = Vec::with_capacity(54);
        for line in self.text().map_err(|e| e.to_string())?.lines() {
            let Some(rest) = line.trim_start().strip_prefix("SetIndexBuffer(") else {
                continue;
            };
            let mut parts = rest.split(',').map(str::trim);
            let Some(index) = parts.next().and_then(|s| s.parse::<u8>().ok()) else {
                continue;
            };
            let Some(name) = parts.next() else { continue };
            bindings.push((index, name.to_owned()));
        }
        bindings.sort_unstable_by_key(|(index, _)| *index);
        Ok(bindings)
    }
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::sha256_bytes;

    #[test]
    fn sha256_known_vector() {
        assert_eq!(
            sha256_bytes(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
