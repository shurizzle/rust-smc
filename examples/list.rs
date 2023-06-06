extern crate smc;

use smc::{FromSMC, Result, SMCError, SMCVal, SMC};

pub enum ValOrErr {
    Val(SMCVal),
    Err(SMCError),
}

impl From<Result<SMCVal>> for ValOrErr {
    fn from(value: Result<SMCVal>) -> Self {
        match value {
            Ok(val) => Self::Val(val),
            Err(err) => Self::Err(err),
        }
    }
}

impl core::fmt::Display for ValOrErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Val(val) => {
                write!(f, "[{}] ", val.r#type.display())?;
                if let Some(i) = i64::from_smc(*val) {
                    write!(f, "{: <11}", i)?;
                } else if let Some(b) = bool::from_smc(*val) {
                    write!(f, "{: <11?}", b)?;
                } else {
                    write!(f, "?          ")?;
                }

                write!(f, " len({: >2}) ", val.len())?;

                for (i, c) in val.data().chunks(2).enumerate() {
                    if i != 0 {
                        write!(f, " ")?;
                    }
                    for c in c {
                        write!(f, "{:0>2x}", c)?;
                    }
                }
                Ok(())
            }
            Self::Err(err) => write!(f, "!!! {:?}", err),
        }
    }
}

fn main() -> Result<()> {
    let smc = SMC::new()?;
    // for f in smc.fan_infos()? {
    //     let f = f?;
    //     println!("{:#?}", f);
    // }
    for key in smc.keys()? {
        let key = key?;
        println!(
            "{} {}",
            key.display(),
            ValOrErr::from(smc.read_key::<SMCVal>(key))
        );
    }
    Ok(())
}
