//! The 1994 elevator settings a `.TDT` import brings with a shaft (#305): the
//! 56-byte schedule block of the save's elevator record, kept as it came so
//! an export back to 1994 writes it again. The simulation never reads it;
//! this game's elevators run by their own schedule (`schedule.rs`). The
//! layout, as far as the game's saves show it, is in
//! `docs/tdt/tdt-byte-map.md` section 6.1.

use serde_json::Value;

/// The save version a tower that keeps a block is written at: an older build
/// would drop the block, and an export from it would lose the shafts' 1994
/// settings without a word, so it refuses the save instead.
pub const LEGACY_SCHEDULE_SAVE_VERSION: i64 = 13;

/// The block's size in the elevator record.
pub const LEGACY_SCHEDULE_LEN: usize = 56;

/// The block as the save names it.
pub type LegacySchedule = Box<[u8; LEGACY_SCHEDULE_LEN]>;

/// The block from its saved form, a string of 112 hex digits. Anything else
/// is no block, and so is an all-zero one: the 1994 game runs no cars on a
/// shaft with it, so a save never keeps one for an export to write.
pub fn legacy_schedule_from_save(raw: Option<&Value>) -> Option<LegacySchedule> {
    let hex = raw?.as_str()?.as_bytes();
    if hex.len() != LEGACY_SCHEDULE_LEN * 2 {
        return None;
    }
    let mut out = [0u8; LEGACY_SCHEDULE_LEN];
    for (byte, pair) in out.iter_mut().zip(hex.chunks_exact(2)) {
        *byte = (hex_digit(pair[0])? << 4) | hex_digit(pair[1])?;
    }
    out.iter().any(|&b| b != 0).then(|| Box::new(out))
}

/// The block in its saved form: lowercase hex, two digits a byte.
pub fn legacy_schedule_to_save(block: &[u8; LEGACY_SCHEDULE_LEN]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(LEGACY_SCHEDULE_LEN * 2);
    for &b in block {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 0x0f) as usize] as char);
    }
    s
}

fn hex_digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn block() -> [u8; LEGACY_SCHEDULE_LEN] {
        let mut b = [0u8; LEGACY_SCHEDULE_LEN];
        b[..14].fill(0x01);
        b[14..28].fill(0x05);
        b[18] = 0x0a;
        b[28..34].copy_from_slice(&[1, 2, 1, 0, 2, 0]);
        b
    }

    #[test]
    fn a_block_survives_its_saved_form() {
        let b = block();
        let saved = legacy_schedule_to_save(&b);
        assert_eq!(saved.len(), 112);
        assert!(saved.starts_with("0101"));
        assert_eq!(
            legacy_schedule_from_save(Some(&json!(saved))).as_deref(),
            Some(&b)
        );
        let upper = json!(saved.to_uppercase());
        assert_eq!(legacy_schedule_from_save(Some(&upper)).as_deref(), Some(&b));
    }

    #[test]
    fn a_damaged_or_dead_block_is_no_block() {
        let saved = legacy_schedule_to_save(&block());
        for bad in [
            json!(saved[..110].to_string()),
            json!(format!("{saved}00")),
            json!(saved.replacen('0', "g", 1)),
            json!(saved.replacen('0', "é", 1)),
            json!("00".repeat(LEGACY_SCHEDULE_LEN)),
            json!(56),
            json!([1, 5, 0]),
            Value::Null,
        ] {
            assert_eq!(legacy_schedule_from_save(Some(&bad)), None, "{bad}");
        }
        assert_eq!(legacy_schedule_from_save(None), None);
    }
}
