use anyhow::Result;
use hidapi::{HidApi, HidDevice};

use crate::utils::parse_string_from;

const PFU_VENDOR_ID: u16 = 1278;
const HHKB_PRODUCT_ID: u16 = 33;
const CONTROL_INTERFACE_NUMBER: i32 = 2;
const WRITE_HEADER: u8 = 0xAA;
const READ_HEADER: u8 = 0x55;
const GET_KEYMAP: u8 = 0x87;

pub struct Hhkb(HidDevice);

#[repr(u8)]
pub enum Layer {
    Base = 0,
    Fn = 1,
}

impl From<u8> for Layer {
    fn from(value: u8) -> Self {
        match value {
            0 => Layer::Base,
            1 => Layer::Fn,
            _ => panic!("Invalid Layer!"),
        }
    }
}

impl Hhkb {
    pub fn new() -> Result<Self> {
        let api = HidApi::new()?;
        let device_info = api
            .device_list()
            .find(|device| {
                device.vendor_id() == PFU_VENDOR_ID
                    && device.product_id() == HHKB_PRODUCT_ID
                    && device.interface_number() == CONTROL_INTERFACE_NUMBER
            })
            .ok_or_else(|| {
                anyhow::anyhow!("HHKB programming interface not found! Is the device plugged in?")
            })?;

        let handle = api.open_path(device_info.path())?;
        Ok(Self(handle))
    }

    pub fn get_info(&self) -> Result<HhkbInfo> {
        let resp = self.send_command(2, None)?;

        let type_number = parse_string_from(&resp[6..26]);
        let revision = parse_string_from(&resp[26..30]);
        let serial = parse_string_from(&resp[30..46]);

        let app_firm_version = format!("{:X}{}.{}{}", resp[46], resp[47], resp[48], resp[49]);
        let boot_firm_version = format!("{:X}{}.{}{}", resp[54], resp[55], resp[56], resp[57]);

        let running_firmware = resp[62];

        Ok(HhkbInfo {
            type_number,
            revision,
            serial,
            app_firm_version,
            boot_firm_version,
            running_firmware,
        })
    }

    pub fn get_mode(&self) -> Result<u8> {
        let resp = self.send_command(6, None)?;
        Ok(resp[6])
    }

    pub fn get_dip_state(&self) -> Result<[bool; 6]> {
        let resp = self.send_command(5, None)?;
        let mut dip = [false; 6];
        for i in 0..6 {
            dip[i] = resp[6 + i] != 0;
        }
        Ok(dip)
    }

    pub fn base_layer(&self, mode: u8) -> Result<[u8; 128]> {
        self.keymap(mode, Layer::Base)
    }

    pub fn fn_layer(&self, mode: u8) -> Result<[u8; 128]> {
        self.keymap(mode, Layer::Fn)
    }

    fn keymap(&self, mode: u8, layer: Layer) -> Result<[u8; 128]> {
        let layout_profile = mode << 4;
        let mut write_buf = [0u8; 65];
        write_buf[0] = 0;
        write_buf[1] = WRITE_HEADER;
        write_buf[2] = WRITE_HEADER;
        write_buf[3] = GET_KEYMAP;
        write_buf[4] = 0;
        write_buf[5] = 2;
        write_buf[6] = layout_profile;
        write_buf[7] = layer as u8;

        self.0.write(&write_buf)?;

        let mut keymap = [0u8; 128];
        let mut read_buf = [0u8; 64];

        self.0.read(&mut read_buf)?;
        Self::verify_keymap_block(&read_buf, 0x41, 58)?;
        keymap[0..58].copy_from_slice(&read_buf[6..64]);

        self.0.read(&mut read_buf)?;
        Self::verify_keymap_block(&read_buf, 0x82, 58)?;
        keymap[58..116].copy_from_slice(&read_buf[6..64]);

        self.0.read(&mut read_buf)?;
        Self::verify_keymap_block(&read_buf, 0xc3, 12)?;
        keymap[116..128].copy_from_slice(&read_buf[6..18]);

        Ok(keymap)
    }

    fn verify_keymap_block(buf: &[u8; 64], expexted_block_id: u8, expected_len: u8) -> Result<()> {
        if buf[0] != READ_HEADER || buf[1] != READ_HEADER {
            return Err(anyhow::anyhow!("Invalid response header"));
        }

        if buf[2] != GET_KEYMAP {
            return Err(anyhow::anyhow!(
                "Response command ID mismatch: expected 0x87"
            ));
        }

        if buf[4] != expexted_block_id {
            return Err(anyhow::anyhow!(
                "Mismatched block ID: expected {:02x}, got {:02x}",
                expexted_block_id,
                buf[4]
            ));
        }

        if buf[5] != expected_len {
            return Err(anyhow::anyhow!(
                "Mismatched block data length: expected {}, got {}",
                expected_len,
                buf[5]
            ));
        }

        Ok(())
    }

    fn send_command(&self, cmd: u8, payload: Option<&[u8; 61]>) -> Result<[u8; 64]> {
        let mut write_buf = [0u8; 65];
        write_buf[0] = 0;
        write_buf[1] = WRITE_HEADER;
        write_buf[2] = WRITE_HEADER;
        write_buf[3] = cmd;

        if let Some(payload) = payload {
            write_buf[4..4 + payload.len()].copy_from_slice(payload);
        }

        self.0.write(&write_buf)?;

        let mut read_buf = [0u8; 64];
        let bytes_read = self.0.read(&mut read_buf)?;

        if bytes_read < 4 {
            return Err(anyhow::anyhow!("Received truncated packet from keyboard"));
        }

        if read_buf[0] != READ_HEADER || read_buf[1] != READ_HEADER {
            return Err(anyhow::anyhow!(
                "Got bad response header: {:02x} {:02x}",
                read_buf[0],
                read_buf[1]
            ));
        }

        if read_buf[2] != cmd {
            return Err(anyhow::anyhow!(
                "Got mismatched command ID: expected {}, got {}",
                cmd,
                read_buf[2]
            ));
        }

        Ok(read_buf)
    }
}

impl AsRef<HidDevice> for Hhkb {
    fn as_ref(&self) -> &HidDevice {
        &self.0
    }
}

#[derive(Debug)]
pub struct HhkbInfo {
    pub type_number: String,
    pub revision: String,
    pub serial: String,
    pub app_firm_version: String,
    pub boot_firm_version: String,
    pub running_firmware: u8,
}
