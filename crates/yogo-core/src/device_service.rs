use crate::{art::Frame, events::atomic_write, protocol::*};
use hidapi::{HidApi, HidDevice};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Backup {
    vendor_id: u16,
    product_id: u16,
    serial_number: String,
    block: [u8; 24],
}
pub struct DeviceService {
    handle: HidDevice,
    sequence: u16,
    last_start: Option<Instant>,
    pub model: String,
    serial: String,
    product_id: u16,
    pub connection: Connection,
    backup_path: PathBuf,
}
impl DeviceService {
    pub fn open(dir: PathBuf) -> Result<Self, String> {
        if dir.join("server.lock").exists() {
            return Err("旧版网页服务仍在运行，请先关闭旧服务".into());
        }
        let api = HidApi::new().map_err(|e| e.to_string())?;
        let mut devices = api
            .device_list()
            .filter_map(|d| {
                Connection::detect(d.vendor_id(), d.product_id(), d.usage_page(), d.usage())
                    .map(|connection| (d, connection))
            })
            .collect::<Vec<_>>();
        devices.sort_by_key(|(_, c)| *c == Connection::Receiver);
        // Two transports may belong to one keyboard. Never guess between two
        // keyboards using the same transport, including a nonresponsive one.
        for connection in [Connection::Usb, Connection::Receiver] {
            if devices.iter().filter(|(_, c)| *c == connection).count() > 1 {
                return Err("检测到多个同类 YOGO 75 PRO 设备，请只保留一台键盘".into());
            }
        }
        let mut errors = Vec::new();
        for (info, connection) in devices {
            let handle = match info.open_device(&api) {
                Ok(handle) => handle,
                Err(e) => {
                    errors.push(e.to_string());
                    continue;
                }
            };
            let mut device = Self {
                handle,
                sequence: (crate::now_ms() % 65534 + 1) as u16,
                last_start: None,
                model: info.product_string().unwrap_or("YOGO 75 PRO").into(),
                serial: info.serial_number().unwrap_or("").into(),
                product_id: info.product_id(),
                connection,
                backup_path: dir.join(connection.backup_file()),
            };
            // Enumeration only proves a dongle is plugged in. Probe the actual
            // keyboard before selecting a transport, without changing settings.
            match device.read_dot() {
                Ok(_) => return Ok(device),
                Err(e) => errors.push(e),
            }
        }
        Err(errors.pop().unwrap_or_else(|| {
            "未找到 YOGO 75 PRO，请连接 USB 数据线并切到有线模式，或连接 2.4G 接收器".into()
        }))
    }
    pub fn check_connection(&mut self) -> Result<(), String> {
        self.read_dot().map(|_| ())
    }
    fn next_sequence(&mut self) -> u16 {
        self.sequence = self.sequence.wrapping_add(1).max(1);
        self.sequence
    }
    fn write(&self, p: &[u8]) -> Result<(), String> {
        let mut report = vec![0; p.len() + 1];
        report[1..].copy_from_slice(p);
        let n = self.handle.write(&report).map_err(|e| e.to_string())?;
        if n != report.len() {
            return Err(format!("HID 写入不完整：{n}/{}", report.len()));
        }
        Ok(())
    }
    fn request(
        &mut self,
        command: u8,
        offset: u16,
        length: u8,
        data: &[u8],
    ) -> Result<Vec<u8>, String> {
        let seq = self.next_sequence();
        self.write(&transport_packet(
            command,
            offset,
            length,
            data,
            seq,
            self.connection.report_size(),
        )?)?;
        let start = Instant::now();
        let mut buf = [0; 65];
        while start.elapsed() < Duration::from_secs(3) {
            let n = self
                .handle
                .read_timeout(&mut buf, 300)
                .map_err(|e| e.to_string())?;
            let r = normalize_reply(&buf[..n]);
            if r.len() < 8
                || r[0] != 0xaa
                || r[1] != command
                || u16::from_le_bytes([r[5], r[6]]) != seq
            {
                continue;
            }
            if r[2] == 0xff {
                return Err(format!("设备拒绝命令 0x{command:02X}"));
            }
            return Ok(r[8..r.len().min(self.connection.report_size())].to_vec());
        }
        Err(format!(
            "设备响应超时 0x{command:02X}；请关闭 ATK 设置页后重试"
        ))
    }
    fn start(&mut self) -> Result<(), String> {
        if self
            .last_start
            .is_none_or(|t| t.elapsed() > Duration::from_secs(3))
        {
            self.request(0x10, 0, 0, &[])?;
            self.last_start = Some(Instant::now())
        }
        Ok(())
    }
    fn read_dot(&mut self) -> Result<[u8; 24], String> {
        self.start()?;
        let data = if self.connection == Connection::Usb {
            let data = self.request(0x14, 0, 56, &[])?;
            data.get(24..48).ok_or("点阵配置长度不正确")?.to_vec()
        } else {
            self.request(0x14, 24, 24, &[])?
        };
        data.try_into().map_err(|_| "点阵配置长度不正确".into())
    }
    fn write_dot(&mut self, block: &[u8; 24]) -> Result<(), String> {
        if self.connection == Connection::Usb {
            let mut first = self.request(0x14, 0, 56, &[])?;
            let last = self.request(0x14, 56, 8, &[])?;
            if first.len() < 56 || last.len() < 8 {
                return Err("设备配置长度不正确".into());
            }
            first[24..48].copy_from_slice(block);
            self.request(0x15, 0, 56, &first[..56])?;
            self.request(0x15, 56, 8, &last[..8])?;
        } else {
            let first = self.request(0x14, 0, 24, &[])?;
            let last = self.request(0x14, 48, 16, &[])?;
            if first.len() < 24 || last.len() < 16 {
                return Err("设备配置长度不正确".into());
            }
            // The final segment commits the staged config on both transports.
            self.request(0x15, 0, 24, &first[..24])?;
            self.request(0x15, 24, 24, block)?;
            self.request(0x15, 48, 16, &last[..16])?;
        }
        std::thread::sleep(Duration::from_millis(250));
        if &self.read_dot()? != block {
            return Err("点阵配置写入校验失败".into());
        }
        Ok(())
    }
    pub fn begin(&mut self) -> Result<(), String> {
        if self.backup_path.exists() {
            self.restore()?;
        }
        let original = self.read_dot()?;
        let backup = Backup {
            vendor_id: VID,
            product_id: self.product_id,
            serial_number: self.serial.clone(),
            block: recovery_target(&original),
        };
        atomic_write(
            &self.backup_path,
            &serde_json::to_vec(&backup).map_err(|e| e.to_string())?,
        )?;
        let mut changed = original;
        changed[5] = 0;
        changed[6] = 6;
        self.request(0x3d, 0, 0, &[])?;
        self.write_dot(&changed)
    }
    pub fn write_frame(&mut self, frame: &Frame) -> Result<(), String> {
        self.start()?;
        let rgb = frame
            .iter()
            .flatten()
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        let size = self.connection.report_size() - 8;
        for offset in (0..108).step_by(size) {
            let seq = self.next_sequence();
            self.write(&transport_packet(
                0x3b,
                offset as u16,
                size as u8,
                &rgb[offset..(offset + size).min(108)],
                seq,
                self.connection.report_size(),
            )?)?;
        }
        Ok(())
    }
    pub fn restore(&mut self) -> Result<(), String> {
        if !self.backup_path.exists() {
            return Ok(());
        }
        let backup: Backup =
            serde_json::from_slice(&fs::read(&self.backup_path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("恢复备份无效：{e}"))?;
        if backup.vendor_id != VID
            || backup.product_id != self.product_id
            || backup.serial_number != self.serial
        {
            return Err("恢复备份属于另一台设备，已保留备份".into());
        }
        let mut current = self.read_dot()?;
        restore_dot(&mut current, &recovery_target(&backup.block));
        self.request(0x3d, 0, 0, &[])?;
        self.write_dot(&current)?;
        fs::remove_file(&self.backup_path).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Explicit opt-in only: changes the physical matrix briefly, then restores it.
    #[test]
    #[ignore = "requires a connected keyboard and YOGOSYNC_HARDWARE_TEST_DIR"]
    fn hardware_display_and_restore() {
        let dir = PathBuf::from(
            std::env::var("YOGOSYNC_HARDWARE_TEST_DIR").expect("test backup directory required"),
        );
        fs::create_dir_all(&dir).unwrap();
        let mut device = DeviceService::open(dir).unwrap();
        println!("Selected {} ({})", device.connection.label(), device.model);
        let before = device.read_dot().unwrap();
        let result = (|| -> Result<(), String> {
            device.begin()?;
            device.write_frame(&crate::art::frame_for(crate::task_states::PetState::Done))?;
            device.check_connection()?;
            Ok(())
        })();
        // Attempt recovery even if display/health checks fail.
        let restored = device.restore();
        result.unwrap();
        restored.unwrap();
        assert_eq!(device.read_dot().unwrap(), recovery_target(&before));
    }
}
