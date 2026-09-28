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
