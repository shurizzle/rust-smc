extern crate smc;

use smc::{Result, SMC};

fn main() -> Result<()> {
    let smc = SMC::new()?;
    for fan in smc.fans()? {
        let fan = fan?;
        println!(
            "{}) {} {}%",
            fan.id(),
            fan.name().escape_ascii(),
            fan.percent(&smc)?
        );
    }
    Ok(())
}
