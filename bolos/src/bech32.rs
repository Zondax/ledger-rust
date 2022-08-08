/*******************************************************************************
*   (c) 2021 Zondax GmbH
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

pub struct OutputBufferTooSmall;

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

/// Perform bits conversion
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

/// Integer in the range `0..32`
#[derive(PartialEq, Eq, Debug, Copy, Clone, Default, PartialOrd, Ord, Hash)]
#[allow(non_camel_case_types)]
struct u5(u8);

impl u5 {
    /// Encoding character set. Maps data value -> char
    const CHARSET: &'static [char; 32] = &[
        'q', 'p', 'z', 'r', 'y', '9', 'x', '8', //  +0
        'g', 'f', '2', 't', 'v', 'd', 'w', '0', //  +8
        's', '3', 'j', 'n', '5', '4', 'k', 'h', // +16
        'c', 'e', '6', 'm', 'u', 'a', '7', 'l', // +24
    ];

    /// Convert a `u8` to `u5` if in range, return `Error` otherwise
    #[allow(dead_code)]
    pub fn try_from_u8(value: u8) -> Result<u5, ()> {
        if value > 31 {
            Err(())
        } else {
            Ok(u5(value))
        }
    }

    /// Returns a copy of the underlying `u8` value
    #[inline(always)]
    pub fn to_u8(self) -> u8 {
        self.0
    }

    #[inline(always)]
    pub fn charset() -> &'static [char; 32] {
        crate::PIC::new(Self::CHARSET).into_inner()
    }
    /// Get char representing this 5 bit value as defined in BIP173
    pub fn to_char(self) -> char {
        Self::charset()[self.to_u8() as usize]
    }
}

impl From<u5> for u8 {
    fn from(num: u5) -> Self {
        num.0
    }
}

impl AsRef<u8> for u5 {
    fn as_ref(&self) -> &u8 {
        &self.0
    }
}

pub struct Bech32Writer<'b> {
    out: &'b mut [u8],
    bytes_written: usize,
    chk: u32,
    variant: Variant,
}

#[derive(Debug)]
pub enum Bech32WriterError {
    OutputBufferTooSmall,
}

impl<'b> Bech32Writer<'b> {
    /// Generator coefficients
    const GEN: &'static [u32; 5] = &[
        0x3b6a_57b2,
        0x2650_8e6d,
        0x1ea1_19fa,
        0x3d42_33dd,
        0x2a14_62b3,
    ];

    /// Creatw a new bech32 writer
    pub fn new(hrp: &str, out: &'b mut [u8], variant: Variant) -> Result<Self, Bech32WriterError> {
        let mut this = Self {
            chk: 1,
            out,
            bytes_written: 0,
            variant,
        };

        let hrp = hrp.as_bytes();
        this.check_rem_out(hrp.len() + 1)?;
        this.out[0..hrp.len()].copy_from_slice(hrp);
        this.out[hrp.len()] = b'1'; //hrp1data (separator)
        this.bytes_written = hrp.len() + 1;

        // expand HRP
        for b in hrp {
            this.polymod_step(u5(b >> 5));
        }
        this.polymod_step(u5(0));
        for b in hrp {
            this.polymod_step(u5(b & 0x1f));
        }

        Ok(this)
    }

    fn polymod_step(&mut self, v: u5) {
        let b = (self.chk >> 25) as u8;
        self.chk = (self.chk & 0x01ff_ffff) << 5 ^ (u32::from(*v.as_ref()));

        for (i, item) in Self::gen().iter().enumerate() {
            if (b >> i) & 1 == 1 {
                self.chk ^= item;
            }
        }
    }

    //verify that out has enough space for the operation
    fn check_rem_out(&self, needed: usize) -> Result<(), Bech32WriterError> {
        let avail = self.out.len() - self.bytes_written;
        if avail < needed {
            Err(Bech32WriterError::OutputBufferTooSmall)
        } else {
            Ok(())
        }
    }

    ///PIC'ed generator coefficients
    fn gen() -> &'static [u32; 5] {
        crate::PIC::new(Self::GEN).into_inner()
    }

    /// Writes a single 5 bit value of the data part
    fn write_u5(&mut self, data: u5) -> Result<(), Bech32WriterError> {
        self.check_rem_out(1)?;
        self.polymod_step(data);
        self.out[self.bytes_written] = data.to_char() as u8;
        self.bytes_written += 1;

        Ok(())
    }

    /// Write a chunck of data
    //TODO: calling write subsequently with the same data split
    // should yield the same result as not splitting the data
    pub fn write(&mut self, data: impl AsRef<[u8]>) -> Result<(), Bech32WriterError> {
        // Amount of bits left over from last round, stored in buffer.
        let mut buffer_bits = 0u32;
        // Holds all unwritten bits left over from last round. The bits are stored beginning from
        // the most significant bit. E.g. if buffer_bits=3, then the byte with bits a, b and c will
        // look as follows: [a, b, c, 0, 0, 0, 0, 0]
        let mut buffer: u8 = 0;

        for b in data.as_ref() {
            // Write first u5 if we have to write two u5s this round. That only happens if the
            // buffer holds too many bits, so we don't have to combine buffer bits with new bits
            // from this rounds byte.
            if buffer_bits >= 5 {
                self.write_u5(u5((buffer & 0b1111_1000) >> 3))?;
                buffer <<= 5;
                buffer_bits -= 5;
            }

            // Combine all bits from buffer with enough bits from this rounds byte so that they fill
            // a u5. Save reamining bits from byte to buffer.
            let from_buffer = buffer >> 3;
            let from_byte = b >> (3 + buffer_bits); // buffer_bits <= 4

            self.write_u5(u5(from_buffer | from_byte))?;
            buffer = b << (5 - buffer_bits);
            buffer_bits += 3;
        }

        // There can be at most two u5s left in the buffer after processing all bytes, write them.
        if buffer_bits >= 5 {
            self.write_u5(u5((buffer & 0b1111_1000) >> 3))?;
            buffer <<= 5;
            buffer_bits -= 5;
        }

        if buffer_bits != 0 {
            self.write_u5(u5(buffer >> 3))?;
        }

        Ok(())
    }

    fn inner_finalize(&mut self) -> Result<usize, Bech32WriterError> {
        // Pad with 6 zeros
        for _ in 0..6 {
            self.polymod_step(u5(0))
        }

        let plm: u32 = self.chk ^ self.variant.constant();

        self.check_rem_out(6)?;
        for p in 0..6 {
            self.out[self.bytes_written] =
                u5(((plm >> (5 * (5 - p))) & 0x1f) as u8).to_char() as u8;
            self.bytes_written += 1;
        }

        Ok(self.bytes_written)
    }

    pub const fn estimate_size(hrp_len: usize, data_len: usize) -> usize {
        let bits_num = data_len * 8;
        //verify if there's a need to add an extra bite for padding
        let base = if bits_num % 5 == 0 {
            bits_num / 5
        } else {
            bits_num / 5 + 1
        };

        hrp_len + 1 + base + 6
    }

    /// Finalize the writer
    pub fn finalize(mut self) -> Result<usize, Bech32WriterError> {
        self.inner_finalize()
    }
}

/// Encode `data` with the given `hrp` into the given `out` with `Bech32`
pub fn encode(
    hrp: &str,
    data: impl AsRef<[u8]>,
    out: &mut [u8],
    variant: Variant,
) -> Result<usize, Bech32WriterError> {
    let mut encoder = Bech32Writer::new(hrp, out, variant)?;
    encoder.write(data)?;
    encoder.finalize()
}

pub const fn estimate_size(hrp_len: usize, data_len: usize) -> usize {
    Bech32Writer::estimate_size(hrp_len, data_len)
}

/// Used for encoding operations for the two variants of Bech32
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Variant {
    /// The original Bech32 described in [BIP-0173](https://github.com/bitcoin/bips/blob/master/bip-0173.mediawiki)
    Bech32,
    /// The improved Bech32m variant described in [BIP-0350](https://github.com/bitcoin/bips/blob/master/bip-0350.mediawiki)
    Bech32m,
}

const BECH32_CONST: u32 = 1;
const BECH32M_CONST: u32 = 0x2bc8_30a3;

impl Variant {
    const fn constant(self) -> u32 {
        match self {
            Variant::Bech32 => BECH32_CONST,
            Variant::Bech32m => BECH32M_CONST,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPECTED: &str = "email1dpjkcmr0gpax7mnyv9uzucmggydtp0";
    const EXPECTEDM: &str = "email1dpjkcmr0gpax7mnyv9uzucmgaca8yd";
    const HRP: &str = "email";
    const INPUT: [u8; 15] = *b"hello@zondax.ch";

    #[test]
    fn estimate() {
        assert_eq!(
            EXPECTED.as_bytes().len(),
            Bech32Writer::estimate_size(HRP.len(), INPUT.len())
        );

        assert_eq!(
            1 + 6 + 1 + HRP.len() + 1,
            Bech32Writer::estimate_size(HRP.len(), 1)
        );
    }

    #[test]
    fn estimatem() {
        assert_eq!(
            EXPECTEDM.as_bytes().len(),
            Bech32Writer::estimate_size(HRP.len(), INPUT.len())
        );

        assert_eq!(
            1 + 6 + 1 + HRP.len() + 1,
            Bech32Writer::estimate_size(HRP.len(), 1)
        );
    }

    #[test]
    fn encode_with_writer() {
        let mut out = [0; Bech32Writer::estimate_size(HRP.len(), INPUT.len())];

        let mut encoder =
            Bech32Writer::new(HRP, &mut out, Variant::Bech32).expect("unable to write HRP");
        encoder.write(&INPUT).expect("unable to write data");
        let written = encoder.finalize().expect("unable to finalize");

        let encoded = std::str::from_utf8(&out).expect("invalid utf8 bytes");

        assert_eq!(&encoded[..written], EXPECTED, "encoding difference");
    }

    #[test]
    #[ignore]
    fn encode_with_writer_pieces() {
        let mut out = [0; Bech32Writer::estimate_size(HRP.len(), INPUT.len())];

        let mut encoder =
            Bech32Writer::new(HRP, &mut out, Variant::Bech32).expect("unable to write HRP");
        encoder.write(&INPUT[..3]).expect("unable to write data");
        encoder.write(&INPUT[3..]).expect("unable to write data");
        let written = encoder.finalize().expect("unable to finalize");

        let encoded = std::str::from_utf8(&out).expect("invalid utf8 bytes");

        assert_eq!(&encoded[..written], EXPECTED, "encoding difference");
    }

    #[test]
    fn encodem_with_writer() {
        let mut out = [0; Bech32Writer::estimate_size(HRP.len(), INPUT.len())];

        let mut encoder =
            Bech32Writer::new(HRP, &mut out, Variant::Bech32m).expect("unable to write HRP");
        encoder.write(&INPUT).expect("unable to write data");
        let written = encoder.finalize().expect("unable to finalize");

        let encoded = std::str::from_utf8(&out).expect("invalid utf8 bytes");

        assert_eq!(&encoded[..written], EXPECTEDM, "encoding difference");
    }

    #[test]
    #[ignore]
    fn encodem_with_writer_pieces() {
        let mut out = [0; Bech32Writer::estimate_size(HRP.len(), INPUT.len())];

        let mut encoder =
            Bech32Writer::new(HRP, &mut out, Variant::Bech32m).expect("unable to write HRP");
        encoder.write(&INPUT[..3]).expect("unable to write data");
        encoder.write(&INPUT[3..]).expect("unable to write data");
        let written = encoder.finalize().expect("unable to finalize");

        let encoded = std::str::from_utf8(&out).expect("invalid utf8 bytes");

        assert_eq!(&encoded[..written], EXPECTEDM, "encoding difference");
    }

    #[test]
    fn encode_with_fn() {
        let mut out = [0; estimate_size(HRP.len(), INPUT.len())];
        let written =
            encode(HRP, &INPUT, &mut out, Variant::Bech32).expect("unable to encode bech32");

        assert_eq!(&out[..written], EXPECTED.as_bytes());
    }

    const HRPS: [&str; 5] = [
        "a",
        "an83characterlonghumanreadablepartthatcontainsthenumber1andtheexcludedcharactersbio",
        "abcdef",
        "1",
        "split",
    ];

    const BECH32_ENCODINGS: [&str;5] = ["A12UEL5L",
"an83characterlonghumanreadablepartthatcontainsthenumber1andtheexcludedcharactersbio1tt5tgs",
"abcdef1w3jhxarfdenjqmn4d43x2u3qxyetjagk",
"11fysxxctwyp3k7atwwss82upqw3hjqarhv4h8g7fqw3mk7grpdejzqarfv5sx67fqd3skxetnyp6hqfqnfpy",
"split1wd5xz6m9ypkhjgrgv9hxggr4wpehgun9v9kjqar0yp4x76twyp6xsefqvdex2actnr6mw"];

    /*
    const BECH32_INPUTS: [&[u8];5] = [& *b"",
        & *b"",
        & [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,
        16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31],
        & [0;82],
        & *b""];
     */

    #[test]
    fn valid_checksum_bech32_0() {
        const INPUT: [u8; 0] = *b"";
        let mut out = [0; estimate_size(HRPS[0].len(), INPUT.len())];
        encode(HRPS[0], &INPUT, &mut out, Variant::Bech32).expect("unable to encode bech32");
        let encoded = std::str::from_utf8(&out).expect("invalid utf8 bytes");

        assert_eq!(BECH32_ENCODINGS[0].to_lowercase(), encoded.to_lowercase());
    }

    #[test]
    fn valid_checksum_bech32_1() {
        const INPUT: [u8; 0] = *b"";
        let mut out = [0; estimate_size(HRPS[1].len(), INPUT.len())];
        encode(HRPS[1], &INPUT, &mut out, Variant::Bech32).expect("unable to encode bech32");
        let encoded = std::str::from_utf8(&out).expect("invalid utf8 bytes");

        assert_eq!(BECH32_ENCODINGS[1].to_lowercase(), encoded.to_lowercase());
    }

    #[test]
    fn valid_checksum_bech32_2() {
        const INPUT: [u8; 16] = *b"testing number 1";
        let mut out = [0; estimate_size(HRPS[2].len(), INPUT.len())];
        let written =
            encode(HRPS[2], &INPUT, &mut out, Variant::Bech32).expect("unable to encode bech32");
        let encoded = std::str::from_utf8(&out[..written]).expect("invalid utf8 bytes");

        assert_eq!(BECH32_ENCODINGS[2].to_lowercase(), encoded.to_lowercase());
    }

    #[test]
    fn valid_checksum_bech32_3() {
        const INPUT: [u8; 48] = *b"I can count up to twenty two and tie my laces up";
        let mut out = [0; estimate_size(HRPS[3].len(), INPUT.len())];
        let written =
            encode(HRPS[3], &INPUT, &mut out, Variant::Bech32).expect("unable to encode bech32");
        let encoded = std::str::from_utf8(&out[..written]).expect("invalid utf8 bytes");

        assert_eq!(BECH32_ENCODINGS[3].to_lowercase(), encoded.to_lowercase());
    }

    #[test]
    fn valid_checksum_bech32_4() {
        const INPUT: [u8; 39] = *b"shake my hand upstream to join the crew";
        let mut out = [0; estimate_size(HRPS[4].len(), INPUT.len())];
        encode(HRPS[4], &INPUT, &mut out, Variant::Bech32).expect("unable to encode bech32");
        let encoded = std::str::from_utf8(&out).expect("invalid utf8 bytes");

        assert_eq!(BECH32_ENCODINGS[4].to_lowercase(), encoded.to_lowercase());
    }

    const BECH32M_HRPS: [&str; 7] = [
        "a",
        "a",
        "an83characterlonghumanreadablepartthatcontainsthetheexcludedcharactersbioandnumber1",
        "abcdef",
        "1",
        "split",
        "?",
    ];
    const BECH32M_ENCODINGS: [&str;7] =["A1LQFN3A", "a1lqfn3a",
        "an83characterlonghumanreadablepartthatcontainsthetheexcludedcharactersbioandnumber11sg7hg6",
        "abcdef1fysxxctwyp3k7atwwss82upqw3hjqarhv4h8g7fqw3mk7grpdejzqarfv5sx67fqd3skxetnyp6hq2jj757",
        "11wd5xz6m9ypkhjgrgv9hxggr4wpehgun9v9kjqar0yp4x76twyp6xsefqvdex2acwrq0w6",
        "split1wd5xz6m9ypkhjgrgv9hxggr4wpehgun9v9kjqar0yp4x76twyp6xsefqvdex2ac70nk7v",
        "?1t9hsrndf9p"];

    #[test]
    fn valid_checksum_bech32m_0() {
        const INPUT: [u8; 0] = *b"";
        let mut out = [0; estimate_size(BECH32M_HRPS[0].len(), INPUT.len())];
        encode(BECH32M_HRPS[0], &INPUT, &mut out, Variant::Bech32m)
            .expect("unable to encode bech32m");
        let encoded = std::str::from_utf8(&out).expect("invalid utf8 bytes");

        assert_eq!(BECH32M_ENCODINGS[0].to_lowercase(), encoded.to_lowercase());
    }

    #[test]
    fn valid_checksum_bech32m_1() {
        const INPUT: [u8; 0] = *b"";
        let mut out = [0; estimate_size(BECH32M_HRPS[1].len(), INPUT.len())];
        encode(BECH32M_HRPS[1], &INPUT, &mut out, Variant::Bech32m)
            .expect("unable to encode bech32m");
        let encoded = std::str::from_utf8(&out).expect("invalid utf8 bytes");

        assert_eq!(BECH32M_ENCODINGS[1].to_lowercase(), encoded.to_lowercase());
    }

    #[test]
    fn valid_checksum_bech32m_2() {
        const INPUT: [u8; 0] = *b"";
        let mut out = [0; estimate_size(BECH32M_HRPS[2].len(), INPUT.len())];
        encode(BECH32M_HRPS[2], &INPUT, &mut out, Variant::Bech32m)
            .expect("unable to encode bech32m");
        let encoded = std::str::from_utf8(&out).expect("invalid utf8 bytes");

        assert_eq!(BECH32M_ENCODINGS[2].to_lowercase(), encoded.to_lowercase());
    }

    #[test]
    fn valid_checksum_bech32m_3() {
        const INPUT: [u8; 48] = *b"I can count up to twenty two and tie my laces up";
        let mut out = [0; estimate_size(BECH32M_HRPS[3].len(), INPUT.len())];
        encode(BECH32M_HRPS[3], &INPUT, &mut out, Variant::Bech32m)
            .expect("unable to encode bech32m");
        let encoded = std::str::from_utf8(&out).expect("invalid utf8 bytes");

        assert_eq!(BECH32M_ENCODINGS[3].to_lowercase(), encoded.to_lowercase());
    }

    #[test]
    fn valid_checksum_bech32m_4() {
        const INPUT: [u8; 39] = *b"shake my hand upstream to join the crew";
        let mut out = [0; estimate_size(BECH32M_HRPS[4].len(), INPUT.len())];
        encode(BECH32M_HRPS[4], &INPUT, &mut out, Variant::Bech32m)
            .expect("unable to encode bech32m");
        let encoded = std::str::from_utf8(&out).expect("invalid utf8 bytes");

        assert_eq!(BECH32M_ENCODINGS[4].to_lowercase(), encoded.to_lowercase());
    }

    #[test]
    fn valid_checksum_bech32m_5() {
        const INPUT: [u8; 39] = *b"shake my hand upstream to join the crew";
        let mut out = [0; estimate_size(BECH32M_HRPS[5].len(), INPUT.len())];
        let written = encode(BECH32M_HRPS[5], &INPUT, &mut out, Variant::Bech32m)
            .expect("unable to encode bech32m");
        let encoded = std::str::from_utf8(&out[..written]).expect("invalid utf8 bytes");

        assert_eq!(BECH32M_ENCODINGS[5].to_lowercase(), encoded.to_lowercase());
    }

    #[test]
    fn valid_checksum_bech32m_6() {
        const INPUT: [u8; 2] = *b"Yo";
        let mut out = [0; estimate_size(BECH32M_HRPS[6].len(), INPUT.len())];
        encode(BECH32M_HRPS[6], &INPUT, &mut out, Variant::Bech32m)
            .expect("unable to encode bech32m");
        let encoded = std::str::from_utf8(&out).expect("invalid utf8 bytes");

        assert_eq!(BECH32M_ENCODINGS[6].to_lowercase(), encoded.to_lowercase());
    }
}
