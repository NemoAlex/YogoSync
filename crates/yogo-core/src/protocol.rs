pub const VID: u16 = 0x373b;
pub const PID: u16 = 0x11ff;
pub const USAGE_PAGE: u16 = 0xff60;
pub const USAGE: u16 = 0x61;

pub fn packet(
    command: u8,
    offset: u16,
    length: u8,
    data: &[u8],
    sequence: u16,
) -> Result<[u8; 32], String> {
    if length > 24 || data.len() > 24 || sequence == 0 {
        return Err("无效 HID 数据包".into());
    }
    let mut out = [0; 32];
    out[0] = 0xaa;
    out[1] = command;
    out[2..4].copy_from_slice(&offset.to_le_bytes());
    out[4] = length;
    out[5..7].copy_from_slice(&sequence.to_le_bytes());
    out[8..8 + data.len()].copy_from_slice(data);
    Ok(out)
}
pub fn normalize_reply(data: &[u8]) -> &[u8] {
    if data.len() > 1 && data[0] != 0xaa && data[1] == 0xaa {
        &data[1..]
    } else {
        data
    }
}
pub fn restore_dot(current: &mut [u8; 24], original: &[u8; 24]) {
    current[5..14].copy_from_slice(&original[5..14]);
}

// Custom pixels cannot be read back. Persist a safe built-in restore target instead.
// ATK HUB's dot preset list starts with mode 0 (star).
pub fn recovery_target(original: &[u8; 24]) -> [u8; 24] {
    let mut target = *original;
    if target[6] == 6 {
        target[5] = 0;
        target[6] = 0;
        if target[7] == 0 {
            target[7] = 50;
        }
        if target[11..14] == [0, 0, 0] {
            target[11..14].copy_from_slice(&[255, 255, 255]);
        }
    }
    target
}
