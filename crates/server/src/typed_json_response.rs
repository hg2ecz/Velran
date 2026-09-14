use language_core::AppError;
use serde_json::{Map, Number, Value};
use std::collections::BTreeSet;
use std::io::{self, Write};

const FRAME_MAGIC: &[u8; 4] = b"VRJ1";
const MAX_FIELDS: usize = 1024;
const MAX_STRING_BYTES: usize = 65_536;
const MAX_ARRAY_ITEMS: usize = 4096;

pub(super) fn decode_and_serialize(frame: &[u8], output_limit: usize) -> Result<String, AppError> {
    let mut cursor = Cursor::new(frame);
    if cursor.take(4)? != FRAME_MAGIC {
        return Err(AppError::Internal);
    }
    let field_count = usize::from(cursor.u16()?);
    if field_count == 0 || field_count > MAX_FIELDS {
        return Err(AppError::Internal);
    }
    let mut names = BTreeSet::new();
    let mut object = Map::new();
    for _ in 0..field_count {
        let name_len = usize::from(cursor.u16()?);
        if name_len == 0 || name_len > 128 {
            return Err(AppError::Internal);
        }
        let name = std::str::from_utf8(cursor.take(name_len)?).map_err(|_| AppError::Internal)?;
        if !valid_field_name(name) || !names.insert(name.to_owned()) {
            return Err(AppError::Internal);
        }
        let tag = cursor.u8()?;
        let value = match tag {
            1 => Value::Number(Number::from(cursor.i64()?)),
            2 => match cursor.u8()? {
                0 => Value::Bool(false),
                1 => Value::Bool(true),
                _ => return Err(AppError::Internal),
            },
            3 => {
                let value = f32::from_bits(cursor.u32()?);
                if !value.is_finite() {
                    return Err(AppError::Internal);
                }
                let number = Number::from_f64(f64::from(value)).ok_or(AppError::Internal)?;
                Value::Number(number)
            }
            4 => Value::String(cursor.string()?.to_owned()),
            5 => {
                let count = usize::try_from(cursor.u32()?).map_err(|_| AppError::Internal)?;
                if count > MAX_ARRAY_ITEMS {
                    return Err(AppError::Internal);
                }
                let mut values = Vec::with_capacity(count);
                for _ in 0..count {
                    values.push(Value::String(cursor.string()?.to_owned()));
                }
                Value::Array(values)
            }
            _ => return Err(AppError::Internal),
        };
        object.insert(name.to_owned(), value);
    }
    if !cursor.is_done() {
        return Err(AppError::Internal);
    }

    let mut writer = BoundedWriter::new(output_limit);
    if serde_json::to_writer(&mut writer, &Value::Object(object)).is_err() {
        return if writer.exceeded {
            Err(AppError::MemoryLimit)
        } else {
            Err(AppError::Internal)
        };
    }
    String::from_utf8(writer.bytes).map_err(|_| AppError::Internal)
}

fn valid_field_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }
    fn take(&mut self, len: usize) -> Result<&'a [u8], AppError> {
        let end = self.pos.checked_add(len).ok_or(AppError::Internal)?;
        let value = self.bytes.get(self.pos..end).ok_or(AppError::Internal)?;
        self.pos = end;
        Ok(value)
    }
    fn u8(&mut self) -> Result<u8, AppError> {
        Ok(*self.take(1)?.first().ok_or(AppError::Internal)?)
    }
    fn u16(&mut self) -> Result<u16, AppError> {
        let b: [u8; 2] = self.take(2)?.try_into().map_err(|_| AppError::Internal)?;
        Ok(u16::from_le_bytes(b))
    }
    fn u32(&mut self) -> Result<u32, AppError> {
        let b: [u8; 4] = self.take(4)?.try_into().map_err(|_| AppError::Internal)?;
        Ok(u32::from_le_bytes(b))
    }
    fn i64(&mut self) -> Result<i64, AppError> {
        let b: [u8; 8] = self.take(8)?.try_into().map_err(|_| AppError::Internal)?;
        Ok(i64::from_le_bytes(b))
    }
    fn string(&mut self) -> Result<&'a str, AppError> {
        let len = usize::try_from(self.u32()?).map_err(|_| AppError::Internal)?;
        if len > MAX_STRING_BYTES {
            return Err(AppError::MemoryLimit);
        }
        std::str::from_utf8(self.take(len)?).map_err(|_| AppError::Internal)
    }
    fn is_done(&self) -> bool {
        self.pos == self.bytes.len()
    }
}

struct BoundedWriter {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}
impl BoundedWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(limit.min(16 * 1024)),
            limit,
            exceeded: false,
        }
    }
}
impl Write for BoundedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let Some(end) = self.bytes.len().checked_add(buf.len()) else {
            self.exceeded = true;
            return Err(io::Error::other("JSON response size overflow"));
        };
        if end > self.limit {
            self.exceeded = true;
            return Err(io::Error::other("JSON response exceeds configured limit"));
        }
        self.bytes.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decodes_typed_frame_and_framework_escapes_json() {
        let mut frame = b"VRJ1".to_vec();
        frame.extend_from_slice(&2u16.to_le_bytes());
        frame.extend_from_slice(&4u16.to_le_bytes());
        frame.extend_from_slice(b"name");
        frame.push(4);
        frame.extend_from_slice(&3u32.to_le_bytes());
        frame.extend_from_slice(b"a\"b");
        frame.extend_from_slice(&5u16.to_le_bytes());
        frame.extend_from_slice(b"count");
        frame.push(1);
        frame.extend_from_slice(&7i64.to_le_bytes());
        let json = decode_and_serialize(&frame, 1024).unwrap();
        assert_eq!(json, r#"{"count":7,"name":"a\"b"}"#);
    }
    #[test]
    fn duplicate_fields_fail_closed() {
        let mut frame = b"VRJ1".to_vec();
        frame.extend_from_slice(&2u16.to_le_bytes());
        for _ in 0..2 {
            frame.extend_from_slice(&1u16.to_le_bytes());
            frame.extend_from_slice(b"x");
            frame.push(2);
            frame.push(1);
        }
        assert!(decode_and_serialize(&frame, 1024).is_err());
    }
    #[test]
    fn serializer_is_bounded() {
        let mut frame = b"VRJ1".to_vec();
        frame.extend_from_slice(&1u16.to_le_bytes());
        frame.extend_from_slice(&1u16.to_le_bytes());
        frame.extend_from_slice(b"x");
        frame.push(4);
        frame.extend_from_slice(&16u32.to_le_bytes());
        frame.extend_from_slice(b"0123456789abcdef");
        assert!(matches!(
            decode_and_serialize(&frame, 8),
            Err(AppError::MemoryLimit)
        ));
    }
}
