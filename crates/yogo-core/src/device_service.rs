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
    backup_path: PathBuf,
}
impl DeviceService {
    pub fn open(dir: PathBuf) -> Result<Self, String> {
        if dir.join("server.lock").exists() {
            return Err("旧版网页服务仍在运行，请先关闭旧服务".into());
        }
        let api = HidApi::new().map_err(|e| e.to_string())?;
        let devices = api
            .device_list()
            .filter(|d| {
                d.vendor_id() == VID
                    && d.product_id() == PID
                    && d.usage_page() == USAGE_PAGE
                    && d.usage() == USAGE
            })
            .collect::<Vec<_>>();
        if devices.len() != 1 {
            return Err(if devices.is_empty() {
                "未找到 YOGO 75 PRO 2.4G 接收器"
            } else {
                "请只连接一个 YOGO 75 PRO 接收器"
            }
            .into());
        }
        let info = devices[0];
        let handle = info.open_device(&api).map_err(|e| e.to_string())?;
        Ok(Self {
            handle,
            sequence: (crate::now_ms() % 65534 + 1) as u16,
            last_start: None,
            model: info.product_string().unwrap_or("YOGO 75 PRO").into(),
            serial: info.serial_number().unwrap_or("").into(),
            backup_path: dir.join("device-backup.json"),
        })
    }
    fn next_sequence(&mut self) -> u16 {
        self.sequence = self.sequence.wrapping_add(1).max(1);
        self.sequence
    }
    fn write(&self, p: &[u8; 32]) -> Result<(), String> {
        let mut report = [0; 33];
        report[1..].copy_from_slice(p);
        let n = self.handle.write(&report).map_err(|e| e.to_string())?;
        if n != 33 {
            return Err(format!("HID 写入不完整：{n}/33"));
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
        self.write(&packet(command, offset, length, data, seq)?)?;
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
            return Ok(r[8..].to_vec());
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
        self.request(0x14, 24, 24, &[])?
            .try_into()
            .map_err(|_| "点阵配置长度不正确".into())
    }
    fn write_dot(&mut self, block: &[u8; 24]) -> Result<(), String> {
        let first = self.request(0x14, 0, 24, &[])?;
        let last = self.request(0x14, 48, 16, &[])?;
        if first.len() < 24 || last.len() < 16 {
            return Err("设备配置长度不正确".into());
        }
        // All three segments are mandatory: the final segment commits the staged config.
        self.request(0x15, 0, 24, &first[..24])?;
        self.request(0x15, 24, 24, block)?;
        self.request(0x15, 48, 16, &last[..16])?;
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
        if original[6] == 6 {
            return Err("当前已是自定义图案，无法备份。请在 ATK 先选择一个预设灯效".into());
        }
        let backup = Backup {
            vendor_id: VID,
            product_id: PID,
            serial_number: self.serial.clone(),
            block: original,
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
        for offset in (0..108).step_by(24) {
            let seq = self.next_sequence();
            self.write(&packet(
                0x3b,
                offset as u16,
                24,
                &rgb[offset..(offset + 24).min(108)],
                seq,
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
            || backup.product_id != PID
            || backup.serial_number != self.serial
        {
            return Err("恢复备份属于另一台设备，已保留备份".into());
        }
        let mut current = self.read_dot()?;
        restore_dot(&mut current, &backup.block);
        self.request(0x3d, 0, 0, &[])?;
        self.write_dot(&current)?;
        fs::remove_file(&self.backup_path).map_err(|e| e.to_string())?;
        Ok(())
    }
}
