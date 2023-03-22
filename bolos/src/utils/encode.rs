/*******************************************************************************
*   (c) 2022 Zondax AG
*
*  Licensed under the Apache License, Version 2.0 (the "License");
*  you may not use this file except in compliance with the License.
*  You may obtain a copy of the License at
*
*      http://www.apache.org/licenses/LICENSE-2.0
*
*  Unless required by applicable law or agreed to in writing, software
*  distributed under the License is distributed on an "AS IS" BASIS,
*  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
*  See the License for the specific language governing permissions and
*  limitations under the License.
********************************************************************************/
use crate::{LedgerUnwrap, PIC};

pub mod bech32;

mod convert_der_to_rs;
pub use convert_der_to_rs::{convert_der_to_rs, ConvertError as ConvertDERtoRSError};

pub enum ConvertBitsError {
    /// `FROM` or `TO` bit size are either 0 or greater than 8
    InvalidConversion {
        from: u8,
        to: u8,
    },
    /// Input value `elem` exceeds `FROM` bit size
    InvalidData {
        idx: usize,
        elem: u8,
    },
    InvalidPadding,
    /// `out` size was too small for the conversion
    OutputBufferTooSmall {
        expected: usize,
    },
}

/// General algorithm to convert a byte sequence from one base to another
pub fn convert_bits<const FROM: u8, const TO: u8>(
    input: &[u8],
    out: &mut [u8],
    pad: bool,
) -> Result<(), ConvertBitsError> {
    if FROM > 8 || TO > 8 || FROM == 0 || TO == 0 {
        return Err(ConvertBitsError::InvalidConversion { from: FROM, to: TO });
    }

    let bits_num = input.len() * FROM as usize;
    let expected_out_size = if bits_num % TO as usize == 0 {
        bits_num / TO as usize
    } else {
        bits_num / TO as usize + 1
    };
    if out.len() < expected_out_size {
        return Err(ConvertBitsError::OutputBufferTooSmall {
            expected: expected_out_size,
        });
    }

    let mut out_idx = 0;

    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    let maxv: u32 = (1 << TO) - 1;
    for (i, value) in input.iter().enumerate() {
        let v = *value as u32;
        if (v >> FROM) != 0 {
            // Input value exceeds `from` bit size
            return Err(ConvertBitsError::InvalidData {
                idx: i,
                elem: v as u8,
            });
        }
        acc = (acc << FROM) | v;
        bits += FROM as u32;
        while bits >= TO as u32 {
            bits -= TO as u32;

            out[out_idx] = ((acc >> bits) & maxv) as u8;
            out_idx += 1;
        }
    }
    if pad {
        if bits > 0 {
            out[out_idx] = ((acc << (TO as u32 - bits)) & maxv) as u8;
        }
    } else if bits >= FROM as u32 || ((acc << (TO as u32 - bits)) & maxv) != 0 {
        return Err(ConvertBitsError::InvalidPadding);
    }

    Ok(())
}

/// Simple error indicating that the output slice was too small for the given input
pub struct OutputBufferTooSmall;

/// Attempt to convert the input byte slice into a hex string
///
/// The hex string will be written to `output`, with the number of bytes written returned
pub fn hex_encode(
    input: impl AsRef<[u8]>,
    output: &mut [u8],
) -> Result<usize, OutputBufferTooSmall> {
    let input = input.as_ref();

    if input.len() * 2 > output.len() {
        return Err(OutputBufferTooSmall);
    }

    const HEX_CHARS_LOWER: &[u8; 16] = b"0123456789abcdef";

    let table = PIC::new(HEX_CHARS_LOWER).into_inner();
    for (byte, out) in input.iter().zip(output.chunks_exact_mut(2)) {
        let high = *table.get(((byte & 0xf0) >> 4) as usize).ledger_unwrap();
        let low = *table.get((byte & 0xf) as usize).ledger_unwrap();

        //number of items guaranteed
        // as we checked the size beforehand so
        // output will always be at least the right length
        // to encode input
        out[0] = high;
        out[1] = low;
    }

    Ok(input.len() * 2)
}

/// Attempt to convert the input byte slice into a base58 encoded string,
/// written in `output`.
///
/// Will return the number of bytes written.
pub fn bs58_encode(
    input: impl AsRef<[u8]>,
    output: &mut [u8],
) -> Result<usize, OutputBufferTooSmall> {
    const ALPHABET_ENCODE: &[u8; 58] =
        b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let table = PIC::new(ALPHABET_ENCODE).into_inner();

    let input = input.as_ref();
    let mut index = 0;

    for &val in input.iter() {
        let mut carry = val as usize;
        for byte in output.get_mut(..index).ledger_unwrap() {
            carry += (*byte as usize) << 8;
            *byte = (carry % 58) as u8;
            carry /= 58;
        }
        while carry > 0 {
            let output = output.get_mut(index).ok_or(OutputBufferTooSmall)?;
            *output = (carry % 58) as u8;
            index += 1;
            carry /= 58;
        }
    }

    for _ in input.iter().take_while(|v| **v == 0) {
        let output = output.get_mut(index).ok_or(OutputBufferTooSmall)?;

        *output = 0;
        index += 1;
    }

    for val in output.get_mut(..index).ledger_unwrap() {
        *val = *table.get(*val as usize).ledger_unwrap();
    }

    output.get_mut(..index).ledger_unwrap().reverse();
    Ok(index)
}
