use std::io::{self, Write};

pub const PACKED_CONTRACT: &str = "RG2_AUCTION_RESEARCH_PACKED_V1";
pub const PACKED_FORMAT_VERSION: u16 = 1;
pub const MAGIC: [u8; 16] = *b"NSTAR_RG2_AUC\0\0\0";
pub const ENDIAN_MARKER: u32 = 0x0102_0304;
pub const HEADER_SIZE: usize = 512;
pub const DIRECTORY_ENTRY_SIZE: usize = 128;
pub const SECTION_ALIGNMENT: u64 = 64;
pub const RELATIONS: [RelationKind; 7] = [
    RelationKind::Runs,
    RelationKind::Events,
    RelationKind::Attempts,
    RelationKind::Episodes,
    RelationKind::Context,
    RelationKind::Features,
    RelationKind::Transits,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum RelationKind {
    Runs = 1,
    Events = 2,
    Attempts = 3,
    Episodes = 4,
    Context = 5,
    Features = 6,
    Transits = 7,
}

impl RelationKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Runs => "runs",
            Self::Events => "events",
            Self::Attempts => "attempts",
            Self::Episodes => "episodes",
            Self::Context => "context",
            Self::Features => "features",
            Self::Transits => "transits",
        }
    }

    pub fn from_code(code: u16) -> Option<Self> {
        RELATIONS.into_iter().find(|kind| *kind as u16 == code)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Header {
    pub file_size: u64,
    pub source_corpus_sha256: [u8; 32],
    pub packed_semantic_sha256: [u8; 32],
    pub schema_dictionary_sha256: [u8; 32],
    pub metadata_sha256: [u8; 32],
    pub counts: [u64; 7],
    pub metadata_offset: u64,
    pub metadata_len: u64,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct DirectoryEntry {
    pub kind: u16,
    pub header_offset: u64,
    pub header_len: u64,
    pub data_offset: u64,
    pub data_len: u64,
    pub index_offset: u64,
    pub index_len: u64,
    pub row_count: u64,
    pub section_sha256: [u8; 32],
}

pub(crate) fn encode_header(header: &Header) -> [u8; HEADER_SIZE] {
    let mut output = [0_u8; HEADER_SIZE];
    output[..16].copy_from_slice(&MAGIC);
    put_u16(&mut output, 16, PACKED_FORMAT_VERSION);
    put_u16(&mut output, 18, HEADER_SIZE as u16);
    put_u32(&mut output, 20, ENDIAN_MARKER);
    put_u16(&mut output, 24, RELATIONS.len() as u16);
    put_u16(&mut output, 26, DIRECTORY_ENTRY_SIZE as u16);
    put_u64(&mut output, 32, header.file_size);
    output[40..72].copy_from_slice(&header.source_corpus_sha256);
    output[72..104].copy_from_slice(&header.packed_semantic_sha256);
    output[104..136].copy_from_slice(&header.schema_dictionary_sha256);
    output[136..168].copy_from_slice(&header.metadata_sha256);
    for (index, count) in header.counts.iter().enumerate() {
        put_u64(&mut output, 168 + index * 8, *count);
    }
    put_u64(&mut output, 224, HEADER_SIZE as u64);
    put_u64(&mut output, 232, header.metadata_offset);
    put_u64(&mut output, 240, header.metadata_len);
    output
}

pub(crate) fn encode_entry(entry: &DirectoryEntry) -> [u8; DIRECTORY_ENTRY_SIZE] {
    let mut output = [0_u8; DIRECTORY_ENTRY_SIZE];
    put_u16(&mut output, 0, entry.kind);
    put_u16(&mut output, 2, 1);
    put_u64(&mut output, 8, entry.header_offset);
    put_u64(&mut output, 16, entry.header_len);
    put_u64(&mut output, 24, entry.data_offset);
    put_u64(&mut output, 32, entry.data_len);
    put_u64(&mut output, 40, entry.index_offset);
    put_u64(&mut output, 48, entry.index_len);
    put_u64(&mut output, 56, entry.row_count);
    output[64..96].copy_from_slice(&entry.section_sha256);
    output
}

pub(crate) fn align_writer(writer: &mut impl Write, position: u64) -> io::Result<u64> {
    let aligned = position.div_ceil(SECTION_ALIGNMENT) * SECTION_ALIGNMENT;
    if aligned > position {
        writer.write_all(&vec![0_u8; (aligned - position) as usize])?;
    }
    Ok(aligned)
}

pub(crate) fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

pub(crate) fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

pub(crate) fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        bytes.get(offset..offset + 8)?.try_into().ok()?,
    ))
}

fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_is_explicit_little_endian_and_self_identifying() {
        let header = Header {
            file_size: 4096,
            source_corpus_sha256: [1; 32],
            packed_semantic_sha256: [2; 32],
            schema_dictionary_sha256: [3; 32],
            metadata_sha256: [4; 32],
            counts: [20, 38_957, 7_764, 1_462, 49_281, 7_764, 628],
            metadata_offset: 1_408,
            metadata_len: 512,
        };
        let bytes = encode_header(&header);
        assert_eq!(&bytes[..16], &MAGIC);
        assert_eq!(read_u16(&bytes, 16), Some(PACKED_FORMAT_VERSION));
        assert_eq!(read_u32(&bytes, 20), Some(ENDIAN_MARKER));
        assert_eq!(read_u64(&bytes, 32), Some(4096));
        assert_eq!(read_u64(&bytes, 168 + 5 * 8), Some(7_764));
    }
}
