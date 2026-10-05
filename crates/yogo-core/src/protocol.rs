pub const VID: u16 = 0x373b;
pub const PID: u16 = 0x11ff;
pub const USB_PID: u16 = 4507;
pub const USAGE_PAGE: u16 = 0xff60;
pub const USAGE: u16 = 0x61;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Connection {
    Usb,
    Receiver,
}
impl Connection {
    pub fn detect(vid: u16, pid: u16, page: u16, usage: u16) -> Option<Self> {
        if vid != VID || page != USAGE_PAGE || usage != USAGE {
            return None;
        }
        match pid {
            USB_PID => Some(Self::Usb),
            PID => Some(Self::Receiver),
            _ => None,
        }
    }
    pub fn report_size(self) -> usize {
        match self {
            Self::Usb => 64,
            Self::Receiver => 32,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Usb => "USB 有线",
            Self::Receiver => "2.4 GHz 接收器",
        }
    }
    pub fn backup_file(self) -> &'static str {
        match self {
            Self::Usb => "device-backup-usb.json",
            Self::Receiver => "device-backup.json",
        }
    }
}
pub fn recovery_pending(dir: &std::path::Path) -> bool {
    [Connection::Usb, Connection::Receiver]
        .iter()
        .any(|c| dir.join(c.backup_file()).exists())
}

pub fn packet(
    command: u8,
    offset: u16,
    length: u8,
    data: &[u8],
    sequence: u16,
) -> Result<[u8; 32], String> {
    transport_packet(command, offset, length, data, sequence, 32)?
        .try_into()
        .map_err(|_| "无效 HID 数据包".into())
}
pub fn transport_packet(
    command: u8,
    offset: u16,
    length: u8,
    data: &[u8],
    sequence: u16,
    report_size: usize,
) -> Result<Vec<u8>, String> {
    if ![32, 64].contains(&report_size)
        || usize::from(length) > report_size - 8
        || data.len() > report_size - 8
        || sequence == 0
    {
        return Err("无效 HID 数据包".into());
    }
    let mut out = vec![0; report_size];
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
