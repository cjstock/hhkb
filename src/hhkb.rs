use std::sync::Mutex;
use std::time::Duration;

use anyhow::{Context, Result, ensure};
use hidapi::{HidApi, HidDevice};

use crate::config::{CurrentKeymaps, Keymap, ModifierKeymaps};
use crate::utils::parse_string_from;

const SELECTORS: [[u8; 2]; 4] = [[0, 0], [0x10, 0], [0, 1], [0x10, 1]];
const TIMEOUT_MS: i32 = 5_000;

// The lock covers entire sessions, including multi-report reads and readback.
// Direct use of AsRef<HidDevice> bypasses this lock and must not overlap these APIs.
pub struct Hhkb(HidDevice, Mutex<()>);

#[repr(u8)]
pub enum Layer {
    Base = 0,
    Fn = 1,
}

impl From<u8> for Layer {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Base,
            1 => Self::Fn,
            _ => panic!("Invalid Layer!"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HhkbInfo {
    pub type_number: String,
    pub revision: String,
    pub serial: String,
    pub app_firm_version: String,
    pub boot_firm_version: String,
    pub running_firmware: u8,
}

/// Full snapshot: base keys, base modifiers, Fn keys, Fn modifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeymapSnapshot {
    pub info: HhkbInfo,
    pub mode: u8,
    pub sleep_minutes: u8,
    pub maps: [Keymap; 4],
}

impl KeymapSnapshot {
    pub fn current_keymaps(&self) -> CurrentKeymaps {
        CurrentKeymaps {
            schema_version: 1,
            model: self.info.type_number.clone(),
            serial: self.info.serial.clone(),
            mode: self.mode,
            base: self.maps[0].clone(),
            fn_layer: self.maps[2].clone(),
            modifiers: Some(ModifierKeymaps {
                base: self.maps[1].clone(),
                fn_layer: self.maps[3].clone(),
            }),
        }
    }
}

trait Transport {
    fn write(&self, report: &[u8; 65]) -> Result<usize>;
    fn read(&self, report: &mut [u8; 64]) -> Result<usize>;
    fn pause(&self) {
        std::thread::sleep(Duration::from_secs(1));
    }
}

impl Transport for HidDevice {
    fn write(&self, report: &[u8; 65]) -> Result<usize> {
        Ok(HidDevice::write(self, report)?)
    }
    fn read(&self, report: &mut [u8; 64]) -> Result<usize> {
        Ok(self.read_timeout(report, TIMEOUT_MS)?)
    }
}

struct Protocol<'a, T>(&'a T);

fn packet(cmd: u8, payload: &[u8]) -> [u8; 65] {
    assert!(payload.len() <= 61);
    let mut report = [0; 65];
    report[1..4].copy_from_slice(&[0xaa, 0xaa, cmd]);
    report[4..4 + payload.len()].copy_from_slice(payload);
    report
}

fn write_blocks(selector: [u8; 2], map: &Keymap) -> [[u8; 65]; 3] {
    let mut data = [0; 130];
    data[..2].copy_from_slice(&selector);
    data[2..].copy_from_slice(&map.0);
    let mut reports = [[0; 65]; 3];
    for ((report, chunk), id) in reports
        .iter_mut()
        .zip(data.chunks(59))
        .zip([0x41, 0x82, 0xc3])
    {
        let mut payload = vec![id, chunk.len() as u8];
        payload.extend_from_slice(chunk);
        *report = packet(0x86, &payload);
    }
    reports
}

impl<T: Transport> Protocol<'_, T> {
    fn send(&self, report: &[u8; 65]) -> Result<()> {
        let count = self.0.write(report)?;
        ensure!(
            count == 65,
            "Short HID write: expected 65 bytes, got {count}"
        );
        Ok(())
    }

    fn reply(&self, cmd: u8) -> Result<[u8; 64]> {
        let mut report = [0; 64];
        let count = self.0.read(&mut report)?;
        ensure!(
            count != 0,
            "Response timeout after {TIMEOUT_MS} ms for command {cmd:02x}"
        );
        ensure!(
            count == 64,
            "Short HID response: expected 64 bytes, got {count}"
        );
        ensure!(report[..2] == [0x55, 0x55], "Invalid response header");
        ensure!(
            report[2] == cmd,
            "Response command ID mismatch: expected {cmd:02x}, got {:02x}",
            report[2]
        );
        ensure!(
            report[3] == 0,
            "Unexpected response byte 3: {:02x} for command {cmd:02x}",
            report[3]
        );
        Ok(report)
    }

    fn command(&self, cmd: u8, payload: &[u8]) -> Result<[u8; 64]> {
        self.send(&packet(cmd, payload))?;
        self.reply(cmd)
    }

    fn session<R>(&self, body: impl FnOnce() -> Result<R>) -> Result<R> {
        // Even a failed opening reply may mean the device entered a session.
        let result = self.command(1, &[0, 1, 0]).and_then(|_| body());
        let close = self.command(1, &[0, 1, 1]);
        match (result, close) {
            (Ok(value), Ok(_)) => Ok(value),
            (Ok(_), Err(err)) => Err(err.context("Session closure failed")),
            (Err(err), Ok(_)) => Err(err),
            (Err(err), Err(close)) => {
                Err(err.context(format!("Session closure also failed: {close:#}")))
            }
        }
    }

    fn info(&self) -> Result<HhkbInfo> {
        let r = self.command(2, &[])?;
        ensure!(r[5] == 57, "Invalid device-info length");
        Ok(HhkbInfo {
            type_number: parse_string_from(&r[6..26]),
            revision: parse_string_from(&r[26..30]),
            serial: parse_string_from(&r[30..46]),
            app_firm_version: format!("{:X}{}.{}{}", r[46], r[47], r[48], r[49]),
            boot_firm_version: format!("{:X}{}.{}{}", r[54], r[55], r[56], r[57]),
            running_firmware: r[62],
        })
    }

    fn mode(&self) -> Result<u8> {
        let r = self.command(6, &[])?;
        ensure!(r[5] == 1, "Invalid mode response length");
        Ok(r[6])
    }

    fn dips(&self) -> Result<[bool; 6]> {
        let r = self.command(5, &[])?;
        ensure!(r[5] == 12, "Invalid DIP response length");
        Ok(std::array::from_fn(|i| r[6 + i] != 0))
    }

    fn keymap(&self, selector: [u8; 2]) -> Result<Keymap> {
        self.send(&packet(0x87, &[0, 2, selector[0], selector[1]]))?;
        let mut map = [0; 128];
        for ((offset, len), id) in [(0, 58), (58, 58), (116, 12)]
            .into_iter()
            .zip([0x41, 0x82, 0xc3])
        {
            let r = self.reply(0x87)?;
            ensure!(
                r[4] == id,
                "Invalid read block ID: expected {id:02x}, got {:02x}",
                r[4]
            );
            ensure!(
                r[5] as usize == len,
                "Invalid read block length: expected {len}, got {}",
                r[5]
            );
            map[offset..offset + len].copy_from_slice(&r[6..6 + len]);
        }
        Ok(Keymap(map))
    }

    fn snapshot(&self) -> Result<KeymapSnapshot> {
        self.session(|| {
            let info = self.info()?;
            self.dips()?;
            let sleep = self.command(9, &[])?;
            ensure!(sleep[5] == 1, "Invalid sleep response length");
            let mut maps = Vec::new();
            for selector in SELECTORS {
                maps.push(self.keymap(selector)?);
            }
            let mode = self.mode()?;
            ensure!(
                mode == 0,
                "Unsupported mode {mode}; only mode 0 is verified"
            );
            Ok(KeymapSnapshot {
                info,
                mode,
                sleep_minutes: sleep[6],
                maps: maps.try_into().unwrap(),
            })
        })
    }

    fn write_current(&self, config: &CurrentKeymaps) -> Result<()> {
        config.validate()?;
        // Preparation is a complete, closed read session. No reads are inserted
        // into the captured write exchange below.
        let before = self.snapshot()?;
        validate_identity(config, &before)?;
        let mut expected = before.clone();
        expected.maps[0] = config.base.clone();
        expected.maps[2] = config.fn_layer.clone();
        if let Some(modifiers) = &config.modifiers {
            expected.maps[1] = modifiers.base.clone();
            expected.maps[3] = modifiers.fn_layer.clone();
        }
        self.session(|| {
            let info = self.info()?;
            ensure!(
                info == before.info,
                "Device identity or firmware changed before write"
            );
            self.dips()?;
            ensure!(self.mode()? == before.mode, "Mode changed before write");
            self.command(8, &[0, 1, before.sleep_minutes])?;
            self.0.pause();
            for (selector, map) in SELECTORS.into_iter().zip(&expected.maps) {
                for report in write_blocks(selector, map) {
                    self.send(&report)?;
                    self.reply(0x86)?;
                }
            }
            self.command(4, &[])?;
            self.command(7, &[0, 1, before.mode])?;
            Ok(())
        })
        .context("Write exchange failed; device may be partially changed; no rollback attempted")?;
        let actual = self
            .snapshot()
            .context("Write completed but fresh readback failed")?;
        ensure!(
            actual.info == expected.info,
            "Readback mismatch: device identity or firmware"
        );
        ensure!(actual.mode == expected.mode, "Readback mismatch: mode");
        ensure!(
            actual.sleep_minutes == expected.sleep_minutes,
            "Readback mismatch: sleep time"
        );
        for (i, selector) in SELECTORS.iter().enumerate() {
            if let Some(index) = actual.maps[i]
                .0
                .iter()
                .zip(expected.maps[i].0)
                .position(|(a, b)| *a != b)
            {
                anyhow::bail!(
                    "Readback mismatch: selector {:02x}/{:02x}, byte {index}",
                    selector[0],
                    selector[1]
                );
            }
        }
        Ok(())
    }
}

pub fn validate_identity(config: &CurrentKeymaps, snapshot: &KeymapSnapshot) -> Result<()> {
    config.validate()?;
    ensure!(
        config.model == snapshot.info.type_number,
        "Device model mismatch"
    );
    ensure!(
        config.serial == snapshot.info.serial,
        "Device serial mismatch"
    );
    ensure!(config.mode == snapshot.mode, "Device mode mismatch");
    if let Some(modifiers) = &config.modifiers
        && (modifiers.base != snapshot.maps[1] || modifiers.fn_layer != snapshot.maps[3])
    {
        let supported = snapshot
            .info
            .app_firm_version
            .strip_prefix("A0.")
            .and_then(|minor| minor.parse::<u16>().ok())
            .is_some_and(|minor| minor >= 48);
        ensure!(
            supported && snapshot.info.running_firmware == 0,
            "Shortcut modifier edits require running HYBRID application firmware A0.48 or later; found {} (running firmware {})",
            snapshot.info.app_firm_version,
            snapshot.info.running_firmware
        );
    }
    Ok(())
}

impl Hhkb {
    pub fn new() -> Result<Self> {
        let api = HidApi::new()?;
        let info = api
            .device_list()
            .find(|d| {
                d.vendor_id() == 0x04fe && d.product_id() == 0x0021 && d.interface_number() == 2
            })
            .context("HHKB programming interface 04fe:0021/2 not found; check USB access")?;
        Ok(Self(api.open_path(info.path())?, Mutex::new(())))
    }

    fn locked<R>(&self, f: impl FnOnce(Protocol<'_, HidDevice>) -> Result<R>) -> Result<R> {
        let _guard = self
            .1
            .lock()
            .map_err(|_| anyhow::anyhow!("HHKB session lock poisoned"))?;
        f(Protocol(&self.0))
    }

    pub fn get_info(&self) -> Result<HhkbInfo> {
        self.locked(|p| p.session(|| p.info()))
    }
    pub fn get_mode(&self) -> Result<u8> {
        self.locked(|p| p.session(|| p.mode()))
    }
    pub fn get_dip_state(&self) -> Result<[bool; 6]> {
        self.locked(|p| p.session(|| p.dips()))
    }
    pub fn base_layer(&self, mode: u8) -> Result<[u8; 128]> {
        self.keymap(mode, Layer::Base)
    }
    pub fn fn_layer(&self, mode: u8) -> Result<[u8; 128]> {
        self.keymap(mode, Layer::Fn)
    }
    fn keymap(&self, mode: u8, layer: Layer) -> Result<[u8; 128]> {
        ensure!(
            mode == 0,
            "Unsupported mode {mode}; only mode 0 is verified"
        );
        self.locked(|p| {
            p.session(|| {
                ensure!(
                    p.mode()? == mode,
                    "Requested mode differs from current keyboard mode"
                );
                Ok(p.keymap([0, layer as u8])?.0)
            })
        })
    }
    pub fn read_snapshot(&self) -> Result<KeymapSnapshot> {
        self.locked(|p| p.snapshot())
    }
    pub fn read_current_keymaps(&self) -> Result<CurrentKeymaps> {
        Ok(self.read_snapshot()?.current_keymaps())
    }
    pub fn write_current_keymaps(&self, config: &CurrentKeymaps) -> Result<()> {
        self.locked(|p| p.write_current(config))
    }
}

impl AsRef<HidDevice> for Hhkb {
    fn as_ref(&self) -> &HidDevice {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    fn fixture(text: &str) -> Vec<[u8; 64]> {
        text.lines()
            .map(|line| {
                line.split_whitespace()
                    .map(|s| u8::from_str_radix(s, 16).unwrap())
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap()
            })
            .collect()
    }
    fn startup() -> Vec<[u8; 64]> {
        fixture(include_str!("../tests/fixtures/startup.hex"))
    }
    fn baseline_write() -> Vec<[u8; 64]> {
        fixture(include_str!("../tests/fixtures/baseline-write.hex"))
    }
    struct Mock {
        script: RefCell<VecDeque<[u8; 64]>>,
        writes: RefCell<Vec<[u8; 65]>>,
        fault: RefCell<Option<(usize, usize)>>,
        read_count: RefCell<usize>,
        short_write: RefCell<Option<usize>>,
        pauses: RefCell<usize>,
    }
    impl Mock {
        fn new(script: Vec<[u8; 64]>) -> Self {
            Self {
                script: RefCell::new(script.into()),
                writes: RefCell::new(vec![]),
                fault: RefCell::new(None),
                read_count: RefCell::new(0),
                short_write: RefCell::new(None),
                pauses: RefCell::new(0),
            }
        }
        fn complete(&self) {
            assert!(self.script.borrow().is_empty(), "Unconsumed reports");
        }
    }
    impl Transport for Mock {
        fn write(&self, report: &[u8; 65]) -> Result<usize> {
            self.writes.borrow_mut().push(*report);
            if let Some(count) = self.short_write.borrow_mut().take() {
                return Ok(count);
            }
            let expected = self
                .script
                .borrow_mut()
                .pop_front()
                .expect("Unexpected write");
            ensure!(
                report[0] == 0 && report[1..] == expected,
                "Outgoing packet mismatch for command {:02x}",
                report[3]
            );
            Ok(65)
        }
        fn read(&self, report: &mut [u8; 64]) -> Result<usize> {
            let expected = self
                .script
                .borrow_mut()
                .pop_front()
                .expect("Unexpected read");
            *report = expected;
            let index = *self.read_count.borrow();
            *self.read_count.borrow_mut() += 1;
            if let Some((at, count)) = *self.fault.borrow()
                && at == index
            {
                return Ok(count);
            }
            Ok(64)
        }
        fn pause(&self) {
            *self.pauses.borrow_mut() += 1;
        }
    }
    fn snapshot() -> KeymapSnapshot {
        let mock = Mock::new(startup());
        let snapshot = Protocol(&mock).snapshot().unwrap();
        mock.complete();
        snapshot
    }
    fn close_reports() -> Vec<[u8; 64]> {
        startup()[26..].to_vec()
    }
    fn write_run(
        write: Vec<[u8; 64]>,
        readback: Vec<[u8; 64]>,
        config: &CurrentKeymaps,
    ) -> (Mock, Result<()>) {
        let mock = Mock::new([startup(), write, readback].concat());
        let result = Protocol(&mock).write_current(config);
        (mock, result)
    }
    #[test]
    fn captured_baseline_reassembles_and_round_trips_exact_packets() {
        let before = snapshot();
        assert_eq!(before.mode, 0);
        assert_eq!(before.sleep_minutes, 30);
        assert_eq!(before.maps[0].0[31], 0xe0);
        assert_eq!(before.maps[0].0[60], 0x29);
        let (mock, result) = write_run(baseline_write(), startup(), &before.current_keymaps());
        result.unwrap();
        mock.complete();
        assert_eq!(*mock.pauses.borrow(), 1);
    }
    #[test]
    fn captured_ctrl_escape_swap_and_manual_fn_restoration() {
        let original = snapshot();
        let swapped = fixture(include_str!("../tests/fixtures/swap-write.hex"));
        let base_restore = fixture(include_str!("../tests/fixtures/base-restore-write.hex"));
        let restored = fixture(include_str!("../tests/fixtures/fn-restored.hex"));
        fn maps(reports: &[[u8; 64]]) -> Vec<Keymap> {
            let chunks: Vec<_> = reports
                .iter()
                .filter(|r| r[..3] == [0xaa, 0xaa, 0x86])
                .collect();
            chunks
                .chunks(3)
                .map(|group| {
                    let data: Vec<_> = group
                        .iter()
                        .flat_map(|r| r[5..5 + r[4] as usize].to_vec())
                        .collect();
                    Keymap(data[2..].try_into().unwrap())
                })
                .collect()
        }
        let swap = maps(&swapped);
        let differences: Vec<_> = original
            .maps
            .iter()
            .zip(&swap)
            .enumerate()
            .flat_map(|(layer, (a, b))| {
                a.0.iter()
                    .zip(b.0)
                    .enumerate()
                    .filter_map(move |(index, (x, y))| (*x != y).then_some((layer, index, *x, y)))
            })
            .collect();
        assert_eq!(
            differences,
            [(0, 31, 0xe0, 0x29), (0, 60, 0x29, 0xe0), (2, 60, 0x29, 0)]
        );
        let partial = maps(&base_restore);
        assert_eq!(partial[0], original.maps[0]);
        assert_eq!(partial[2].0[60], 0);
        assert_eq!(maps(&restored), original.maps);
        let readback_start = restored
            .iter()
            .rposition(|r| r[..6] == [0xaa, 0xaa, 1, 0, 1, 0])
            .unwrap();
        let mock = Mock::new(restored[readback_start..].to_vec());
        assert_eq!(Protocol(&mock).snapshot().unwrap(), original);
        mock.complete();
    }
    #[test]
    fn writes_shortcut_keys_and_masks_and_checks_modifier_readback() {
        let mut config = snapshot().current_keymaps();
        config.base.0[30] = 0x06; // Physical A -> Ctrl+Shift+C.
        config.fn_layer.0[40] = 0x4c; // Physical T -> RCtrl+RAlt+Delete.
        let modifiers = config.modifiers.as_mut().unwrap();
        modifiers.base.0[30] = 0x03;
        modifiers.fn_layer.0[40] = 0x50;
        let mut write = baseline_write();
        // Independent expected bytes in the captured first-block payloads;
        // these expectations do not use the production packet encoder.
        for report in &mut write {
            if report[..4] == [0xaa, 0xaa, 0x86, 0x41] {
                match (report[5], report[6]) {
                    (0, 0) => report[7 + 30] = 0x06,
                    (0x10, 0) => report[7 + 30] = 0x03,
                    (0, 1) => report[7 + 40] = 0x4c,
                    (0x10, 1) => report[7 + 40] = 0x50,
                    _ => panic!("Unexpected selectors"),
                }
            }
        }
        let mut readback = startup();
        for (block, report) in readback
            .iter_mut()
            .filter(|r| r[..3] == [0x55, 0x55, 0x87])
            .enumerate()
        {
            match block {
                0 => report[6 + 30] = 0x06,
                3 => report[6 + 30] = 0x03,
                6 => report[6 + 40] = 0x4c,
                9 => report[6 + 40] = 0x50,
                _ => {}
            }
        }
        let (mock, result) = write_run(write.clone(), readback.clone(), &config);
        result.unwrap();
        mock.complete();
        let reply = readback
            .iter_mut()
            .filter(|r| r[..3] == [0x55, 0x55, 0x87])
            .nth(9)
            .unwrap();
        reply[6 + 40] = 0;
        let (mock, result) = write_run(write, readback, &config);
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("selector 10/01, byte 40")
        );
        mock.complete();
    }

    #[test]
    fn legacy_imports_preserve_nonzero_modifier_maps() {
        let mut initial = startup();
        initial
            .iter_mut()
            .filter(|r| r[..3] == [0x55, 0x55, 0x87])
            .nth(3)
            .unwrap()[6 + 30] = 0x03;
        let mock = Mock::new(initial.clone());
        let before = Protocol(&mock).snapshot().unwrap();
        mock.complete();
        let mut config = before.current_keymaps();
        config.modifiers = None;
        let mut write = baseline_write();
        write
            .iter_mut()
            .find(|r| r[..7] == [0xaa, 0xaa, 0x86, 0x41, 59, 0x10, 0])
            .unwrap()[7 + 30] = 0x03;
        let mock = Mock::new([initial.clone(), write, initial].concat());
        Protocol(&mock).write_current(&config).unwrap();
        mock.complete();
    }

    #[test]
    fn rejects_shortcut_edits_on_old_or_boot_firmware_before_writing() {
        for (minor, running) in [(47, 0), (48, 1)] {
            let mut initial = startup();
            initial[3][48] = minor / 10;
            initial[3][49] = minor % 10;
            initial[3][62] = running;
            let mut config = snapshot().current_keymaps();
            config.modifiers.as_mut().unwrap().base.0[30] = 1;
            let mock = Mock::new(initial);
            let error = Protocol(&mock)
                .write_current(&config)
                .unwrap_err()
                .to_string();
            assert!(error.contains("A0.48"), "{error}");
            assert!(
                mock.writes
                    .borrow()
                    .iter()
                    .all(|r| ![8, 0x86, 4, 7].contains(&r[3]))
            );
            mock.complete();
        }
    }

    #[test]
    fn independently_edits_layers_and_preserves_companions() {
        let before = snapshot();
        for index in [0, 2] {
            let mut after = before.clone();
            after.maps[index].0[30] = 5;
            let mut write = baseline_write();
            let mut map_number = 0;
            for r in &mut write {
                if r[..3] == [0xaa, 0xaa, 0x86] {
                    let block = map_number % 3;
                    *r = write_blocks(SELECTORS[map_number / 3], &after.maps[map_number / 3])
                        [block][1..]
                        .try_into()
                        .unwrap();
                    map_number += 1;
                }
            }
            let mut readback = startup();
            let mut read_number = 0;
            for r in &mut readback {
                if r[..3] == [0x55, 0x55, 0x87] {
                    let block = read_number % 3;
                    let offset = [0, 58, 116][block];
                    let len = [58, 58, 12][block];
                    r[6..6 + len]
                        .copy_from_slice(&after.maps[read_number / 3].0[offset..offset + len]);
                    read_number += 1;
                }
            }
            let (mock, result) = write_run(write, readback, &after.current_keymaps());
            result.unwrap();
            mock.complete();
        }
    }
    #[test]
    fn rejects_short_transfers_timeouts_and_malformed_replies_and_closes() {
        // Faults in the opening reply, info reply, and first map block.
        for read_index in [0, 1, 4] {
            for length in [0, 3, 63] {
                let full = startup();
                let end = full
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r[..2] == [0x55, 0x55])
                    .nth(read_index)
                    .unwrap()
                    .0;
                let mock = Mock::new([full[..=end].to_vec(), close_reports()].concat());
                *mock.fault.borrow_mut() = Some((read_index, length));
                assert!(Protocol(&mock).snapshot().is_err());
                mock.complete();
            }
        }
        for (position, value) in [(0, 0), (2, 0), (3, 1), (4, 0x82), (5, 57)] {
            let mut script = startup()[..10].to_vec();
            script[9][position] = value;
            script.extend(close_reports());
            let mock = Mock::new(script);
            assert!(Protocol(&mock).snapshot().is_err());
            mock.complete();
        }
        let mock = Mock::new(close_reports());
        *mock.short_write.borrow_mut() = Some(64);
        assert!(Protocol(&mock).snapshot().is_err());
        mock.complete();
    }

    #[test]
    fn write_block_boundaries_preserve_every_byte_and_report_id() {
        let map = Keymap(std::array::from_fn(|i| i as u8));
        let blocks = write_blocks([0x10, 1], &map);
        let mut data = Vec::new();
        for ((r, id), len) in blocks.iter().zip([0x41, 0x82, 0xc3]).zip([59, 59, 12]) {
            assert_eq!(r[..6], [0, 0xaa, 0xaa, 0x86, id, len]);
            data.extend_from_slice(&r[6..6 + len as usize]);
            assert!(r[6 + len as usize..].iter().all(|byte| *byte == 0));
        }
        assert_eq!(&data[..2], &[0x10, 1]);
        assert_eq!(&data[2..], map.0);
    }

    #[test]
    fn reports_cleanup_failure_when_unread_blocks_remain() {
        let full = startup();
        let mut script = full[..10].to_vec();
        script[9][4] = 0;
        script.push(full[26]); // Best-effort close, with a pending map reply.
        script.push(full[10]);
        let mock = Mock::new(script);
        let error = format!("{:#}", Protocol(&mock).snapshot().unwrap_err());
        assert!(error.contains("Invalid read block ID"));
        assert!(error.contains("Session closure also failed"));
        assert!(error.contains("command ID mismatch"));
        mock.complete();
    }
    #[test]
    fn stops_after_failed_write_reply_without_commit_or_retry() {
        let before = snapshot();
        let mut write = baseline_write()[..12].to_vec();
        write[11][3] = 1;
        write.extend(close_reports());
        let mock = Mock::new([startup(), write].concat());
        assert!(
            Protocol(&mock)
                .write_current(&before.current_keymaps())
                .is_err()
        );
        mock.complete();
        assert_eq!(
            mock.writes.borrow().iter().filter(|r| r[3] == 0x86).count(),
            1
        );
        assert!(!mock.writes.borrow().iter().any(|r| r[3] == 4));
    }
    #[test]
    fn identifies_readback_mismatches() {
        let config = snapshot().current_keymaps();
        for (report_index, byte, value, expected) in [
            (10, 6, 9, "00/00"),
            (14, 6, 9, "10/00"),
            (18, 6, 9, "00/01"),
            (22, 6, 9, "10/01"),
            (7, 6, 20, "sleep"),
            (25, 6, 1, "Unsupported mode"),
        ] {
            let mut readback = startup();
            readback[report_index][byte] = value;
            let (mock, result) = write_run(baseline_write(), readback, &config);
            let error = format!("{:#}", result.unwrap_err());
            assert!(error.contains(expected), "{error}");
            mock.complete();
        }
    }
    #[test]
    fn invalid_identity_and_schema_never_write_configuration() {
        let mut config = snapshot().current_keymaps();
        config.serial = "OTHER".into();
        let mock = Mock::new(startup());
        assert!(
            Protocol(&mock)
                .write_current(&config)
                .unwrap_err()
                .to_string()
                .contains("serial")
        );
        mock.complete();
        config.schema_version = 2;
        let mock = Mock::new(vec![]);
        assert!(Protocol(&mock).write_current(&config).is_err());
        assert!(mock.writes.borrow().is_empty());
    }
    #[test]
    fn closure_failure_is_reported_alongside_original_error() {
        let mut script = startup()[..4].to_vec();
        script[3][3] = 1;
        script.extend(close_reports());
        script[5][3] = 1;
        let mock = Mock::new(script);
        let error = format!("{:#}", Protocol(&mock).snapshot().unwrap_err());
        assert!(error.contains("closure also failed"));
        assert!(error.contains("command 02"));
        mock.complete();
    }
}
