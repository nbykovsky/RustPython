use std::ops::{Bound, RangeBounds};

pub fn try_get_chars(s: &str, range: impl RangeBounds<usize>) -> Option<&str> {
    let mut chars = s.chars();
    let start = match range.start_bound() {
        Bound::Included(&i) => i,
        Bound::Excluded(&i) => i + 1,
        Bound::Unbounded => 0,
    };
    for _ in 0..start {
        chars.next()?;
    }
    let s = chars.as_str();
    let range_len = match range.end_bound() {
        Bound::Included(&i) => i + 1 - start,
        Bound::Excluded(&i) => i - start,
        Bound::Unbounded => return Some(s),
    };
    char_range_end(s, range_len).map(|end| &s[..end])
}

pub fn get_chars(s: &str, range: impl RangeBounds<usize>) -> &str {
    try_get_chars(s, range).unwrap()
}

#[inline]
pub fn char_range_end(s: &str, nchars: usize) -> Option<usize> {
    let i = match nchars.checked_sub(1) {
        Some(last_char_index) => {
            let (index, c) = s.char_indices().nth(last_char_index)?;
            index + c.len_utf8()
        }
        None => 0,
    };
    Some(i)
}

pub fn zfill(bytes: &[u8], width: usize) -> Vec<u8> {
    if width <= bytes.len() {
        bytes.to_vec()
    } else {
        let (sign, s) = match bytes.first() {
            Some(_sign @ b'+') | Some(_sign @ b'-') => {
                (unsafe { bytes.get_unchecked(..1) }, &bytes[1..])
            }
            _ => (&b""[..], bytes),
        };
        let mut filled = Vec::new();
        filled.extend_from_slice(sign);
        filled.extend(std::iter::repeat(b'0').take(width - bytes.len()));
        filled.extend_from_slice(s);
        filled
    }
}

/// Convert a string to ascii compatible, escaping unicodes into escape
/// sequences.
pub fn to_ascii(value: &str) -> String {
    let mut ascii = String::new();
    for c in value.chars() {
        if c.is_ascii() {
            ascii.push(c)
        } else {
            let c = c as i64;
            let hex = if c < 0x100 {
                format!("\\x{:02x}", c)
            } else if c < 0x10000 {
                format!("\\u{:04x}", c)
            } else {
                format!("\\U{:08x}", c)
            };
            ascii.push_str(&hex)
        }
    }
    ascii
}

/// Get a python `repr()` of the string value. Returns None for OverflowError.
pub fn repr(s: &str) -> Option<String> {
    let in_len = s.len();
    let mut out_len = 0usize;
    // let mut max = 127;
    let mut squote = 0;
    let mut dquote = 0;

    for ch in s.chars() {
        let incr = match ch {
            '\'' => {
                squote += 1;
                1
            }
            '"' => {
                dquote += 1;
                1
            }
            '\\' | '\t' | '\r' | '\n' => 2,
            ch if ch < ' ' || ch as u32 == 0x7f => 4, // \xHH
            ch if ch.is_ascii() => 1,
            ch if crate::char::is_printable(ch) => {
                // max = std::cmp::max(ch, max);
                ch.len_utf8()
            }
            ch if (ch as u32) < 0x100 => 4,   // \xHH
            ch if (ch as u32) < 0x10000 => 6, // \uHHHH
            _ => 10,                          // \uHHHHHHHH
        };
        if out_len > (std::isize::MAX as usize) - incr {
            return None;
        }
        out_len += incr;
    }

    let (quote, num_escaped_quotes) = choose_quotes_for_repr(squote, dquote);
    // we'll be adding backslashes in front of the existing inner quotes
    out_len += num_escaped_quotes;

    // if we don't need to escape anything we can just copy
    let unchanged = out_len == in_len;

    // start and ending quotes
    out_len += 2;

    let mut repr = String::with_capacity(out_len);
    repr.push(quote);
    if unchanged {
        repr.push_str(s);
    } else {
        for ch in s.chars() {
            use std::fmt::Write;
            match ch {
                '\n' => repr.push_str("\\n"),
                '\t' => repr.push_str("\\t"),
                '\r' => repr.push_str("\\r"),
                // these 2 branches *would* be handled below, but we shouldn't have to do a
                // unicodedata lookup just for ascii characters
                '\x20'..='\x7e' => {
                    // printable ascii range
                    if ch == quote || ch == '\\' {
                        repr.push('\\');
                    }
                    repr.push(ch);
                }
                ch if ch.is_ascii() => {
                    write!(repr, "\\x{:02x}", ch as u8).unwrap();
                }
                ch if crate::char::is_printable(ch) => {
                    repr.push(ch);
                }
                '\0'..='\u{ff}' => {
                    write!(repr, "\\x{:02x}", ch as u32).unwrap();
                }
                '\0'..='\u{ffff}' => {
                    write!(repr, "\\u{:04x}", ch as u32).unwrap();
                }
                _ => {
                    write!(repr, "\\U{:08x}", ch as u32).unwrap();
                }
            }
        }
    }
    repr.push(quote);

    Some(repr)
}

/// returns the outer quotes to use and the number of quotes that need to be escaped
pub(crate) fn choose_quotes_for_repr(num_squotes: usize, num_dquotes: usize) -> (char, usize) {
    // always use squote unless we have squotes but no dquotes
    let use_dquote = num_squotes > 0 && num_dquotes == 0;
    if use_dquote {
        ('"', num_dquotes)
    } else {
        ('\'', num_squotes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_chars() {
        let s = "0123456789";
        assert_eq!(get_chars(s, 3..7), "3456");
        assert_eq!(get_chars(s, 3..7), &s[3..7]);

        let s = "0유니코드 문자열9";
        assert_eq!(get_chars(s, 3..7), "코드 문");

        let s = "0😀😃😄😁😆😅😂🤣9";
        assert_eq!(get_chars(s, 3..7), "😄😁😆😅");
    }
}
