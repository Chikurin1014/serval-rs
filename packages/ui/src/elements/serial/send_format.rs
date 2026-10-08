#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendFormat {
    Text,
    /// e.g. `17fff` for `01 7f ff`
    Hex,
    /// e.g. `100000001` for `01 01`
    Bin,
    /// e.g. `256` for `01 00`
    Dec,
}

impl SendFormat {
    pub const ALL: [SendFormat; 4] = [Self::Text, Self::Hex, Self::Bin, Self::Dec];

    pub fn name(self) -> &'static str {
        match self {
            Self::Text => "TXT",
            Self::Hex => "HEX",
            Self::Bin => "BIN",
            Self::Dec => "DEC",
        }
    }

    pub fn prefix(self) -> Option<&'static str> {
        match self {
            Self::Hex => Some("0x"),
            Self::Bin => Some("0b"),
            _ => None,
        }
    }

    pub fn placeholder(self) -> &'static str {
        match self {
            Self::Text => "Type text to send to the active port",
            Self::Hex => "A number to send, e.g. 17fff",
            Self::Bin => "A number to send, e.g. 1111_0000",
            Self::Dec => "A number to send, e.g. 1024",
        }
    }

    /// The bytes `input` stands for (big-endian), or why there are none.
    pub fn parse(self, input: &str) -> Result<Vec<u8>, String> {
        let (radix, digits_per_byte) = match self {
            Self::Text => return Ok(input.as_bytes().to_vec()),
            Self::Hex => (16, Some(2)),
            Self::Bin => (2, Some(8)),
            Self::Dec => (10, None),
        };
        let digits = digits(input, self.prefix().unwrap_or_default(), radix)?;
        Ok(match digits_per_byte {
            Some(width) => grouped_bytes(&digits, radix, width),
            None => decimal_bytes(&digits),
        })
    }
}

/// The digits of `input`, without `prefix`, spaces and `_`.
fn digits(input: &str, prefix: &str, radix: u32) -> Result<String, String> {
    let input = input.trim();
    let number = match prefix {
        "" => input,
        prefix => input.strip_prefix(prefix).unwrap_or(input),
    };
    let digits = number
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_')
        .collect::<String>();
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return Err(format!("'{input}' is not a number in base {radix}"));
    }
    Ok(digits)
}

/// `digits` in `radix`, `width` digits a byte.
fn grouped_bytes(digits: &str, radix: u32, width: usize) -> Vec<u8> {
    let padding = (width - digits.len() % width) % width;
    let padded = format!("{}{digits}", "0".repeat(padding));
    padded
        .as_bytes()
        .chunks(width)
        .map(|byte| {
            let byte = std::str::from_utf8(byte).expect("ASCII digits");
            u8::from_str_radix(byte, radix).expect("checked digits")
        })
        .collect()
}

fn decimal_bytes(digits: &str) -> Vec<u8> {
    let mut bytes = vec![0u8];
    for digit in digits.bytes().map(|digit| u32::from(digit - b'0')) {
        let mut carry = digit;
        for byte in bytes.iter_mut().rev() {
            let value = u32::from(*byte) * 10 + carry;
            *byte = value as u8;
            carry = value >> 8;
        }
        while carry > 0 {
            bytes.insert(0, carry as u8);
            carry >>= 8;
        }
    }
    let leading_zeros = bytes.iter().take_while(|&&byte| byte == 0).count();
    bytes.split_off(leading_zeros.min(bytes.len() - 1))
}

#[cfg(test)]
mod tests {
    use super::SendFormat;

    #[test]
    fn text_is_its_utf8_bytes() {
        assert_eq!(
            SendFormat::Text.parse("a 温"),
            Ok("a 温".as_bytes().to_vec())
        );
    }

    #[test]
    fn hex_is_two_digits_a_byte() {
        assert_eq!(SendFormat::Hex.parse("17fff"), Ok(vec![0x01, 0x7f, 0xff]));
        assert_eq!(
            SendFormat::Hex.parse("0x01 7F_FF"),
            Ok(vec![0x01, 0x7f, 0xff])
        );
        assert_eq!(SendFormat::Hex.parse("0001"), Ok(vec![0x00, 0x01]));
        assert!(SendFormat::Hex.parse("0g").is_err());
        assert!(SendFormat::Hex.parse(" ").is_err());
    }

    #[test]
    fn bin_is_eight_digits_a_byte() {
        assert_eq!(SendFormat::Bin.parse("100000001"), Ok(vec![0x01, 0x01]));
        assert_eq!(SendFormat::Bin.parse("1111_0000"), Ok(vec![0xf0]));
        assert!(SendFormat::Bin.parse("12").is_err());
    }

    #[test]
    fn dec_is_a_number_in_as_few_bytes_as_it_takes() {
        assert_eq!(SendFormat::Dec.parse("0"), Ok(vec![0]));
        assert_eq!(SendFormat::Dec.parse("255"), Ok(vec![0xff]));
        assert_eq!(SendFormat::Dec.parse("256"), Ok(vec![0x01, 0x00]));
        assert_eq!(SendFormat::Dec.parse("0065536"), Ok(vec![0x01, 0x00, 0x00]));
        assert_eq!(
            SendFormat::Dec.parse("18446744073709551616"),
            Ok(vec![1, 0, 0, 0, 0, 0, 0, 0, 0])
        );
        assert!(SendFormat::Dec.parse("-1").is_err());
        assert!(SendFormat::Dec.parse("1.5").is_err());
    }
}
