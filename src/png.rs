use crate::render::Framebuffer;

const MAX_STORED_BLOCK: usize = 65535;

pub fn encode(framebuffer: &Framebuffer) -> Vec<u8> {
    let (width, height) = (framebuffer.width, framebuffer.height);
    let mut raw = Vec::with_capacity((width * 3 + 1) * height);
    for row in framebuffer.pixels.chunks(width) {
        raw.push(0);
        for pixel in row {
            raw.extend_from_slice(&[(pixel >> 16) as u8, (pixel >> 8) as u8, *pixel as u8]);
        }
    }

    let mut output = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut header = Vec::new();
    header.extend_from_slice(&(width as u32).to_be_bytes());
    header.extend_from_slice(&(height as u32).to_be_bytes());
    header.extend_from_slice(&[8, 2, 0, 0, 0]);
    write_chunk(&mut output, b"IHDR", &header);
    write_chunk(&mut output, b"IDAT", &zlib_stored(&raw));
    write_chunk(&mut output, b"IEND", &[]);
    output
}

pub fn write(framebuffer: &Framebuffer, path: &str) -> Result<(), String> {
    std::fs::write(path, encode(framebuffer)).map_err(|error| format!("{}: {}", path, error))
}

fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut output = vec![0x78, 0x01];
    let mut blocks = data.chunks(MAX_STORED_BLOCK).peekable();
    if blocks.peek().is_none() {
        output.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
    }
    while let Some(block) = blocks.next() {
        let is_last = blocks.peek().is_none();
        output.push(is_last as u8);
        output.extend_from_slice(&(block.len() as u16).to_le_bytes());
        output.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        output.extend_from_slice(block);
    }
    output.extend_from_slice(&adler32(data).to_be_bytes());
    output
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for byte in data {
        a = (a + *byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn write_chunk(output: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = output.len();
    output.extend_from_slice(kind);
    output.extend_from_slice(data);
    let checksum = crc32(&output[start..]);
    output.extend_from_slice(&checksum.to_be_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in data {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}
