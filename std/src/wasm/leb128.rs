pub struct Leb128;

impl Leb128 {
    /// Decodes an unsigned LEB128 integer (u32)
    pub fn decode_u32(data: &[u8], offset: &mut usize) -> u32 {
        let mut result = 0;
        let mut shift = 0;
        loop {
            let byte = data[*offset];
            *offset += 1;
            
            result |= ((byte & 0x7F) as u32) << shift;
            if (byte & 0x80) == 0 {
                break;
            }
            shift += 7;
        }
        result
    }

    /// Decodes a signed LEB128 integer (i32)
    pub fn decode_i32(data: &[u8], offset: &mut usize) -> i32 {
        Self::decode_i64(data, offset) as i32
    }

    /// Decodes an unsigned LEB128 integer (u64)
    pub fn decode_u64(data: &[u8], offset: &mut usize) -> u64 {
        let mut result = 0;
        let mut shift = 0;
        loop {
            let byte = data[*offset];
            *offset += 1;
            result |= ((byte & 0x7F) as u64) << shift;
            if (byte & 0x80) == 0 { break; }
            shift += 7;
        }
        result
    }

    /// Decodes a signed LEB128 integer (i64)
    pub fn decode_i64(data: &[u8], offset: &mut usize) -> i64 {
        let mut result = 0;
        let mut shift = 0;
        let mut byte;
        loop {
            byte = data[*offset];
            *offset += 1;
            result |= ((byte & 0x7F) as i64) << shift;
            shift += 7;
            if (byte & 0x80) == 0 { break; }
        }
        if shift < 64 && (byte & 0x40) != 0 {
            result |= !0 << shift;
        }
        result
    }
}
