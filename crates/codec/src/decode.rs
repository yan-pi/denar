//! Decoder: bytes to [`CoreExpr`], the exact inverse of [`crate::encode`].

use btclisp_core::CoreExpr;

use crate::error::Error;

/// Decode a single canonical value, requiring all input to be consumed.
///
/// # Errors
///
/// Returns [`Error`] on truncated input, an over-long varint, a malformed list
/// header, or trailing bytes after a complete value.
///
/// # Example
///
/// ```
/// use btclisp_core::CoreExpr;
/// use btclisp_codec::decode;
///
/// assert_eq!(decode(&[0x05]).unwrap(), CoreExpr::atom(vec![0x05]));
/// assert_eq!(decode(&[0x00]).unwrap(), CoreExpr::Nil);
/// ```
pub fn decode(bytes: &[u8]) -> Result<CoreExpr, Error> {
    let mut reader = Reader { buf: bytes, pos: 0 };
    let value = decode_node(&mut reader)?;
    if reader.pos != bytes.len() {
        return Err(Error::TrailingBytes(bytes.len() - reader.pos));
    }
    Ok(value)
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn read_u8(&mut self) -> Result<u8, Error> {
        let byte = *self.buf.get(self.pos).ok_or(Error::UnexpectedEof)?;
        self.pos += 1;
        Ok(byte)
    }

    fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>, Error> {
        let end = self.pos.checked_add(n).ok_or(Error::UnexpectedEof)?;
        let slice = self.buf.get(self.pos..end).ok_or(Error::UnexpectedEof)?;
        self.pos = end;
        Ok(slice.to_vec())
    }

    fn read_varint(&mut self) -> Result<u64, Error> {
        let mut result: u64 = 0;
        let mut shift = 0u32;
        loop {
            let byte = self.read_u8()?;
            let payload = u64::from(byte & 0x7f);
            result = result
                .checked_add(payload.checked_shl(shift).ok_or(Error::VarintTooLong)?)
                .ok_or(Error::VarintTooLong)?;
            if byte & 0x80 == 0 {
                return Ok(result);
            }
            shift += 7;
            if shift >= 64 {
                return Err(Error::VarintTooLong);
            }
        }
    }
}

fn decode_node(reader: &mut Reader) -> Result<CoreExpr, Error> {
    let code = reader.read_u8()?;
    decode_with_code(code, reader)
}

fn decode_with_code(code: u8, reader: &mut Reader) -> Result<CoreExpr, Error> {
    if code & 0x80 != 0 {
        let inner = decode_with_code(code & 0x7f, reader)?;
        return Ok(CoreExpr::cons(CoreExpr::atom(vec![0x00]), inner));
    }
    match code {
        0x00 => Ok(CoreExpr::Nil),
        0x01..=0x33 => Ok(CoreExpr::atom(vec![code])),
        0x34 => {
            let len = usize::from(reader.read_u8()?);
            Ok(CoreExpr::atom(reader.read_bytes(len)?))
        }
        0x35..=0x73 => {
            let len = usize::from(code - 0x35) + 2;
            Ok(CoreExpr::atom(reader.read_bytes(len)?))
        }
        0x74 => {
            let len = usize::try_from(reader.read_varint()?).map_err(|_| Error::UnexpectedEof)?;
            Ok(CoreExpr::atom(reader.read_bytes(len)?))
        }
        0x75..=0x79 => {
            let n = usize::from(code - 0x75) + 1;
            decode_proper_list(n, reader)
        }
        0x7a..=0x7e => {
            let entries = usize::from(code - 0x7a) + 2;
            decode_improper_list(entries, reader)
        }
        0x7f => {
            let header = reader.read_varint()?;
            let proper = header & 1 == 1;
            let size = usize::try_from(header >> 1).map_err(|_| Error::UnexpectedEof)?;
            if proper {
                decode_proper_list(size, reader)
            } else {
                decode_improper_list(size, reader)
            }
        }
        0x80..=0xff => unreachable!("high-bit codes handled above"),
    }
}

fn decode_proper_list(n: usize, reader: &mut Reader) -> Result<CoreExpr, Error> {
    let mut elems = Vec::with_capacity(n);
    for _ in 0..n {
        elems.push(decode_node(reader)?);
    }
    let mut list = CoreExpr::Nil;
    for el in elems.into_iter().rev() {
        list = CoreExpr::cons(el, list);
    }
    Ok(list)
}

fn decode_improper_list(entries: usize, reader: &mut Reader) -> Result<CoreExpr, Error> {
    if entries < 2 {
        return Err(Error::MalformedList);
    }
    let mut elems = Vec::with_capacity(entries);
    for _ in 0..entries {
        elems.push(decode_node(reader)?);
    }
    let terminator = match elems.pop() {
        Some(term) => term,
        None => return Err(Error::MalformedList),
    };
    let mut list = terminator;
    for el in elems.into_iter().rev() {
        list = CoreExpr::cons(el, list);
    }
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_nil_and_literals() {
        assert_eq!(decode(&[0x00]).unwrap(), CoreExpr::Nil);
        assert_eq!(decode(&[0x05]).unwrap(), CoreExpr::atom(vec![0x05]));
    }

    #[test]
    fn decodes_leftover_single_byte() {
        assert_eq!(
            decode(&[0x34, 0x01, 0xff]).unwrap(),
            CoreExpr::atom(vec![0xff])
        );
    }

    #[test]
    fn decodes_quote_shorthand() {
        let expected = CoreExpr::cons(CoreExpr::atom(vec![0x00]), CoreExpr::atom(vec![0x05]));
        assert_eq!(decode(&[0x85]).unwrap(), expected);
    }

    #[test]
    fn decodes_proper_list() {
        let expected = CoreExpr::cons(
            CoreExpr::atom(vec![0x17]),
            CoreExpr::cons(CoreExpr::atom(vec![0x01]), CoreExpr::Nil),
        );
        assert_eq!(decode(&[0x76, 0x17, 0x01]).unwrap(), expected);
    }

    #[test]
    fn decodes_improper_pair() {
        let expected = CoreExpr::cons(CoreExpr::atom(vec![0x01]), CoreExpr::atom(vec![0x02]));
        assert_eq!(decode(&[0x7a, 0x01, 0x02]).unwrap(), expected);
    }

    #[test]
    fn rejects_truncated_input() {
        assert_eq!(decode(&[0x35, 0xaa]), Err(Error::UnexpectedEof));
    }

    #[test]
    fn rejects_trailing_bytes() {
        assert_eq!(decode(&[0x00, 0x00]), Err(Error::TrailingBytes(1)));
    }
}
