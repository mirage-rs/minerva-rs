#![no_std]
#![deny(rust_2018_idioms)]

use core::{mem, ptr};

pub const SDRAM0_TABLE: &[u8; 49280] = include_bytes!(
    "./minerva_tc/mtc_tables/nintendo_switch/sdram0_nx_abca2_0_3_10NoCfgVersion_V9.8.7_V1.6.bin"
);

pub struct MinervaTrainer {
    cfg: raw::mtc_config_t,
    table: &'static [raw::emc_table_t; 10],
}

impl MinervaTrainer {
    /// Creates a new memory trainer that will use the given emc table.
    pub fn new(table: &'static [u8; 49280]) -> Self {
        let table = transform_table(table);
        MinervaTrainer {
            table,
            cfg: unsafe { mem::zeroed() },
        }
    }

    /// Initializes this `MinervaTrainer`.
    ///
    /// This method **has** to be called before any operation can be done.
    pub fn init(&mut self) {}
}

fn read_clk_src_emc() -> u32 {
    let addr = raw::CLOCK_BASE + raw::CLK_RST_CONTROLLER_CLK_SOURCE_EMC;
    unsafe { ptr::read_volatile(addr as *const u32) }
}

/// Transforms the raw table representation of bytes to an slice of emc tables.
///
/// The returned slice will always have a length of 10.
fn transform_table(table: &'static [u8; 49280]) -> &'static [raw::emc_table_t; 10] {
    use core::convert::TryInto;

    // SAFETY:
    //
    // The size of `raw::emc_table_t` is equal to the length of the given table
    // divided by 10.
    let slice = unsafe { core::slice::from_raw_parts(table.as_ptr() as *const _, 10) };
    slice.try_into().unwrap()
}

mod raw {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
