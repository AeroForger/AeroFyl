use std::fmt;

const ELF_HEADER_SIZE: usize = 64;
const PROGRAM_HEADER_SIZE: usize = 56;
const CODE_OFFSET: usize = 0x1000;
const IMAGE_BASE: u64 = 0x400000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ElfError {
    EmptyCode,
    EntryOutsideCode {
        entry_offset: usize,
        code_len: usize,
    },
    InvalidDataOffset,
    ImageTooLarge,
}

impl fmt::Display for ElfError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCode => {
                formatter.write_str("cannot create an executable without machine code")
            }
            Self::EntryOutsideCode {
                entry_offset,
                code_len,
            } => {
                write!(
                    formatter,
                    "entry offset {entry_offset} is outside {code_len}-byte code section"
                )
            }
            Self::ImageTooLarge => {
                formatter.write_str("ELF image length does not fit ELF64 fields")
            }
            Self::InvalidDataOffset => {
                formatter.write_str("read-only data offset overlaps code or is not page-aligned")
            }
        }
    }
}

impl std::error::Error for ElfError {}

/// Wraps already-lowered x86-64 machine code in a minimal Linux ELF executable.
/// This machine-level API makes no choice about the Aerofyl entry function or ABI.
pub fn write_executable(
    code: &[u8],
    entry_offset: usize,
    read_only_data: &[u8],
    data_offset: usize,
) -> Result<Vec<u8>, ElfError> {
    if code.is_empty() {
        return Err(ElfError::EmptyCode);
    }
    if entry_offset >= code.len() {
        return Err(ElfError::EntryOutsideCode {
            entry_offset,
            code_len: code.len(),
        });
    }
    if !read_only_data.is_empty()
        && (data_offset < code.len() || !data_offset.is_multiple_of(0x1000))
    {
        return Err(ElfError::InvalidDataOffset);
    }
    let code_end = CODE_OFFSET
        .checked_add(code.len())
        .ok_or(ElfError::ImageTooLarge)?;
    let image_len = if read_only_data.is_empty() {
        code_end
    } else {
        CODE_OFFSET
            .checked_add(data_offset)
            .and_then(|offset| offset.checked_add(read_only_data.len()))
            .ok_or(ElfError::ImageTooLarge)?
    };
    let entry = IMAGE_BASE + CODE_OFFSET as u64 + entry_offset as u64;

    let mut image = Vec::with_capacity(image_len);
    image.extend_from_slice(&[0x7f, b'E', b'L', b'F']);
    image.extend_from_slice(&[2, 1, 1, 0]); // ELF64, little endian, version, System V.
    image.extend_from_slice(&[0; 8]);
    push_u16(&mut image, 2); // ET_EXEC
    push_u16(&mut image, 62); // EM_X86_64
    push_u32(&mut image, 1);
    push_u64(&mut image, entry);
    push_u64(&mut image, ELF_HEADER_SIZE as u64);
    push_u64(&mut image, 0); // no section table
    push_u32(&mut image, 0);
    push_u16(&mut image, ELF_HEADER_SIZE as u16);
    push_u16(&mut image, PROGRAM_HEADER_SIZE as u16);
    push_u16(&mut image, if read_only_data.is_empty() { 1 } else { 2 });
    push_u16(&mut image, 0);
    push_u16(&mut image, 0);
    push_u16(&mut image, 0);

    push_u32(&mut image, 1); // PT_LOAD
    push_u32(&mut image, 5); // readable + executable
    push_u64(&mut image, 0);
    push_u64(&mut image, IMAGE_BASE);
    push_u64(&mut image, IMAGE_BASE);
    let code_end_u64 = u64::try_from(code_end).map_err(|_| ElfError::ImageTooLarge)?;
    push_u64(&mut image, code_end_u64);
    push_u64(&mut image, code_end_u64);
    push_u64(&mut image, 0x1000);

    if !read_only_data.is_empty() {
        let file_offset = CODE_OFFSET
            .checked_add(data_offset)
            .ok_or(ElfError::ImageTooLarge)?;
        let file_offset_u64 = u64::try_from(file_offset).map_err(|_| ElfError::ImageTooLarge)?;
        let data_len = u64::try_from(read_only_data.len()).map_err(|_| ElfError::ImageTooLarge)?;
        push_u32(&mut image, 1); // PT_LOAD
        push_u32(&mut image, 4); // readable
        push_u64(&mut image, file_offset_u64);
        push_u64(&mut image, IMAGE_BASE + file_offset_u64);
        push_u64(&mut image, IMAGE_BASE + file_offset_u64);
        push_u64(&mut image, data_len);
        push_u64(&mut image, data_len);
        push_u64(&mut image, 0x1000);
    }

    debug_assert_eq!(
        image.len(),
        ELF_HEADER_SIZE + PROGRAM_HEADER_SIZE * if read_only_data.is_empty() { 1 } else { 2 }
    );
    image.resize(CODE_OFFSET, 0);
    image.extend_from_slice(code);
    if !read_only_data.is_empty() {
        image.resize(CODE_OFFSET + data_offset, 0);
        image.extend_from_slice(read_only_data);
    }
    Ok(image)
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}
fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}
fn push_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_valid_elf64_layout_fields() {
        let image = write_executable(&[0xc3], 0, &[], 0x1000).unwrap();
        assert_eq!(&image[..4], b"\x7fELF");
        assert_eq!(image[4], 2);
        assert_eq!(u16::from_le_bytes([image[18], image[19]]), 62);
        assert_eq!(
            u64::from_le_bytes(image[24..32].try_into().unwrap()),
            IMAGE_BASE + CODE_OFFSET as u64
        );
        assert_eq!(image[CODE_OFFSET], 0xc3);
    }

    #[test]
    fn validates_entry_offset() {
        assert!(matches!(
            write_executable(&[0xc3], 1, &[], 0x1000),
            Err(ElfError::EntryOutsideCode { .. })
        ));
    }

    #[test]
    fn writes_read_only_data_in_a_separate_segment() {
        let image = write_executable(&[0xc3], 0, b"data", 0x1000).unwrap();
        assert_eq!(u16::from_le_bytes([image[56], image[57]]), 2);
        let second_header = ELF_HEADER_SIZE + PROGRAM_HEADER_SIZE;
        assert_eq!(
            u32::from_le_bytes(
                image[second_header + 4..second_header + 8]
                    .try_into()
                    .unwrap()
            ),
            4
        );
        assert_eq!(&image[CODE_OFFSET + 0x1000..], b"data");
    }
}
