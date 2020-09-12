#![no_std]
#![deny(rust_2018_idioms)]

use core::{mem, ptr};

pub const SDRAM0_TABLE: &[u8; 49280] = include_bytes!(
    "./minerva_tc/mtc_tables/nintendo_switch/sdram0_nx_abca2_0_3_10NoCfgVersion_V9.8.7_V1.6.bin"
);

#[derive(Debug, Copy, Clone)]
pub enum Frequency {
    Freq204,
    Freq800,
    Freq1600,
}

impl Into<i32> for Frequency {
    fn into(self) -> i32 {
        match self {
            Frequency::Freq204 => 204000,
            Frequency::Freq800 => 800000,
            Frequency::Freq1600 => 1600000,
        }
    }
}

pub struct MinervaTrainer {
    cfg: raw::mtc_config_t,
    tables: &'static [raw::emc_table_t; 10],
}

impl MinervaTrainer {
    /// Creates a new memory trainer that will use the given emc table.
    pub fn new(table: &'static [u8; 49280]) -> Self {
        MinervaTrainer {
            tables: transform_table(table),
            cfg: unsafe { mem::zeroed() },
        }
    }

    /// Initializes this `MinervaTrainer`.
    ///
    /// This method **has** to be called before any operation can be done.
    pub unsafe fn init(&mut self, sdram_id: u32) {
        self.cfg.mtc_table = self.tables.as_ptr() as *mut _;
        self.cfg.sdram_id = sdram_id;

        let ram_index = (0..10)
            .find(|idx| read_clk_src_emc() == self.tables[*idx].clk_src_emc)
            .unwrap_or(0);

        self.cfg.rate_from = self.tables[ram_index].rate_khz as i32;
        self.cfg.rate_to = 204000;
        self.cfg.train_mode = raw::train_mode_t::OP_TRAIN.0;
        raw::minerva_main(&mut self.cfg);
        self.cfg.rate_to = 800000;
        raw::minerva_main(&mut self.cfg);
        self.cfg.rate_to = 1600000;
        raw::minerva_main(&mut self.cfg);

        // FSP WAR.
        self.cfg.train_mode = raw::train_mode_t::OP_SWITCH.0;
        self.cfg.rate_to = 800000;
        raw::minerva_main(&mut self.cfg);

        // Switch to max.
        self.cfg.rate_to = 1600000;
        raw::minerva_main(&mut self.cfg);
    }

    /// Changes the frequency of this `MinervaTrainer`.
    pub unsafe fn change_freq(&mut self, freq: Frequency) {
        let freq = freq.into();

        if self.cfg.rate_from != freq {
            self.cfg.rate_to = freq;
            self.cfg.train_mode = raw::train_mode_t::OP_SWITCH.0;
            raw::minerva_main(&mut self.cfg);
        }
    }

    pub unsafe fn periodic_training(&mut self) {
        if self.cfg.rate_from == Frequency::Freq1600.into() {
            self.cfg.train_mode = raw::train_mode_t::OP_PERIODIC_TRAIN.0;
            raw::minerva_main(&mut self.cfg);
        }
    }
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
    #![allow(dead_code)]

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
