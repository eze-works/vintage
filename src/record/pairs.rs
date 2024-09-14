use crate::error::Error;
use std::collections::BTreeMap;
use std::io::{self, Cursor, Read, Write};

type Pairs = BTreeMap<String, String>;

// The high-order bit of the first byte of a length indicates the length's encoding. A high-order
// zero implies a one-byte encoding, a one a four-byte encoding.
fn read_pair_len<R: Read>(reader: &mut R) -> Result<u32, Error> {
    let mut sentinel = [0u8; 1];

    reader
        .read_exact(&mut sentinel)
        .map_err(|_| Error::MalformedRecordPayload("Params"))?;

    if sentinel[0] <= 127 {
        return Ok(sentinel[0] as u32);
    }

    let mut len_bytes = [sentinel[0] & 0b0111_1111, 0, 0, 0];
    reader
        .read_exact(&mut len_bytes[1..])
        .map_err(|_| Error::MalformedRecordPayload("Params"))?;

    let len = u32::from_be_bytes(len_bytes);
    Ok(len)
}

fn write_pair_len<W: Write>((key, value): (&str, &str), writer: &mut W) -> Result<(), io::Error> {
    if key.len() > 127 {
        let mut len_bytes = (key.len() as u32).to_be_bytes();
        len_bytes[0] |= 0b1000_0000;
        writer.write_all(&len_bytes)?;
    } else {
        writer.write_all(&(key.len() as u8).to_be_bytes())?;
    }

    if value.len() > 127 {
        let mut len_bytes = (value.len() as u32).to_be_bytes();
        len_bytes[0] |= 0b1000_0000;
        writer.write_all(&len_bytes)?;
    } else {
        writer.write_all(&(value.len() as u8).to_be_bytes())?;
    }

    Ok(())
}

// FastCGI transmits a name-value pair as the length of the name, followed by the length of the
// value, followed by the name, followed by the value. Lengths of 127 bytes and less can be
// encoded in one byte, while longer lengths are always encoded in four bytes:
pub fn from_record_bytes(bytes: Vec<u8>) -> Result<Pairs, Error> {
    let len = bytes.len();
    let mut cursor = Cursor::new(bytes);
    let mut pairs = BTreeMap::new();

    loop {
        let position = cursor.position() as usize;

        if position == len {
            break;
        }

        let name_len = read_pair_len(&mut cursor)?;
        let value_len = read_pair_len(&mut cursor)?;

        let mut name = vec![0u8; name_len as usize];
        let mut value = vec![0u8; value_len as usize];

        cursor
            .read_exact(&mut name)
            .map_err(|_| Error::MalformedRecordPayload("Params"))?;
        cursor
            .read_exact(&mut value)
            .map_err(|_| Error::MalformedRecordPayload("Params"))?;

        let name = String::from_utf8(name).map_err(|_| Error::InvalidUtf8KeyValuePair)?;
        let value = String::from_utf8(value).map_err(|_| Error::InvalidUtf8KeyValuePair)?;

        pairs.insert(name, value);
    }

    Ok(pairs)
}

pub fn to_record_bytes<W: Write>(pairs: &Pairs, writer: &mut W) -> Result<(), io::Error> {
    for (key, value) in pairs.iter() {
        write_pair_len((key.as_str(), value.as_str()), writer)?;
        write!(writer, "{}{}", key, value)?;
    }

    Ok(())
}
