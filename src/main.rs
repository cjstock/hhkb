use anyhow::Result;
use hhkb::Hhkb;

fn main() -> Result<()> {
    let hhkb = Hhkb::new()?;
    let info = hhkb.get_info()?;
    println!("{:?}", info);
    let dips = hhkb.get_dip_state()?;
    println!("{:?}", dips);
    let mode = hhkb.get_mode()?;
    println!("{:?}", mode);

    let base_layer = hhkb.base_layer(mode)?;
    println!("{:?}", base_layer);
    Ok(())
}
