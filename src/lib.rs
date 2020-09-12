//! Rust bindings to the [minerva_tc] C library for Tegra X1 DRAM training.
//!
//! Supports DDR2/3 und LPDDR3/4 memory training.
//!
//! # Example
//!
//! ```no_run
//! use libtegra::fuse;
//! use minerva_rs::{Frequency, MinervaTrainer};
//!
//! // Create and initialize a new memory trainer. Switch to the maximum supported frequency.
//! let mut trainer = MinervaTrainer::new(fuse::read_sdram_id())
//!     .expect("Failed to create a memory trainer for an unknown DRAM type.");
//! trainer.init();
//!
//! // Switch to a DRAM frequency of 800MHz.
//! trainer.change_frequency(Frequency::Freq800);
//! ```
//!
//! [minerva_tc]: https://github.com/CTCaer/minerva_tc

#![no_std]
#![deny(rust_2018_idioms)]

use core::{mem, ptr};

/// The frequencies to train DRAM to.
#[derive(Debug, Copy, Clone)]
pub enum Frequency {
    /// A DRAM frequency of 204MHz.
    Freq204,
    /// A DRAM frequency of 800MHz.
    Freq800,
    /// A DRAM frequency of 1600MHz.
    Freq1600,
}

impl Into<i32> for Frequency {
    fn into(self) -> i32 {
        match self {
            Frequency::Freq204 => 204_000,
            Frequency::Freq800 => 800_000,
            Frequency::Freq1600 => 1_600_000,
        }
    }
}

/// The Minerva memory trainer for Tegra X1 SoCs.
///
/// It is responsible for training the Tegra X1 DRAM with pre-defined profiles based on the SDRAM
/// ID that can be obtained from the fuses.
///
/// Instances of it should be created over the [`MinervaTrainer::new`] method. They need
/// to be initialized with [`MinervaTrainer::init`] afterwards before it can be used freely.
pub struct MinervaTrainer {
    cfg: raw::mtc_config_t,
    tables: &'static [raw::emc_table_t; 10],
}

impl MinervaTrainer {
    /// Creates a new DRAM trainer that will use the table that selects the correct DRAM
    /// profile based on the supplied SDRAM ID.
    ///
    /// Returns `None` if the SDRAM ID is invalid.
    pub fn new(sdram_id: u32) -> Option<Self> {
        let profile = dram_profile::get_by_sdram_id(sdram_id)?;

        let mut cfg = unsafe { mem::zeroed::<raw::mtc_config_t>() };
        cfg.sdram_id = sdram_id;

        Some(MinervaTrainer {
            tables: transform_table(profile),
            cfg,
        })
    }

    /// Initializes this DRAM trainer.
    ///
    /// This method **has** to be called in advance before any DRAM training can be done.
    pub fn init(&mut self) {
        self.cfg.mtc_table = self.tables.as_ptr() as *mut _;

        let ram_index = (0..10)
            .find(|idx| read_clk_src_emc() == self.tables[*idx].clk_src_emc)
            .unwrap_or(0);

        self.cfg.rate_from = self.tables[ram_index].rate_khz as i32;
        self.cfg.rate_to = Frequency::Freq204.into();
        self.cfg.train_mode = raw::train_mode_t::OP_TRAIN.0;
        unsafe { raw::minerva_main(&mut self.cfg) };
        self.cfg.rate_to = Frequency::Freq800.into();
        unsafe { raw::minerva_main(&mut self.cfg) };
        self.cfg.rate_to = Frequency::Freq1600.into();
        unsafe { raw::minerva_main(&mut self.cfg) };

        // FSP WAR.
        self.cfg.train_mode = raw::train_mode_t::OP_SWITCH.0;
        self.cfg.rate_to = Frequency::Freq800.into();
        unsafe { raw::minerva_main(&mut self.cfg) };

        // Switch to highest frequency of 1600MHz.
        self.cfg.rate_to = Frequency::Freq1600.into();
        unsafe { raw::minerva_main(&mut self.cfg) };
    }

    /// Changes the DRAM frequency of this DRAM trainer.
    pub fn change_frequency(&mut self, freq: Frequency) {
        let freq = freq.into();

        if self.cfg.rate_from != freq {
            self.cfg.rate_to = freq;
            self.cfg.train_mode = raw::train_mode_t::OP_SWITCH.0;
            unsafe { raw::minerva_main(&mut self.cfg) };
        }
    }

    /// Performs periodic memory training compensation on the DRAM with the profile
    /// selected by this DRAM trainer.
    pub fn periodic_training(&mut self) {
        if self.cfg.rate_from == Frequency::Freq1600.into() {
            self.cfg.train_mode = raw::train_mode_t::OP_PERIODIC_TRAIN.0;
            unsafe { raw::minerva_main(&mut self.cfg) };
        }
    }
}

fn read_clk_src_emc() -> u32 {
    let addr = raw::CLOCK_BASE + raw::CLK_RST_CONTROLLER_CLK_SOURCE_EMC;
    unsafe { ptr::read_volatile(addr as *const u32) }
}

fn transform_table(table: &'static [u8; 49280]) -> &'static [raw::emc_table_t; 10] {
    use core::convert::TryInto;

    // SAFETY: The size of `raw::emc_table_t` is equal to the length of the given table
    //         divided by 10.
    let slice = unsafe { core::slice::from_raw_parts(table.as_ptr() as *const _, 10) };
    slice.try_into().unwrap()
}

/// DRAM profiles for Minerva memory training.
pub mod dram_profile {
    /// The SDRAM ID for the Samsung K4F6E304HB-MGCH LPDDR4 DRAM block.
    pub const DRAM_4GB_SAMSUNG_K4F6E304HB_MGCH: u32 = 0;

    /// The SDRAM ID for the SK Hynix H9HCNNNBPUMLHR-HLN LPDDR4 DRAM block.
    pub const DRAM_4GB_HYNIX_H9HCNNNBPUMLHR_NLN: u32 = 1;

    /// The SDRAM ID for the Mouser Micron MT53B512M32D2NP-062-WT:C LPDDR4 DRAM block.
    pub const DRAM_4GB_MICRON_MT53B512M32D2NP_062_WT: u32 = 2;

    /// The SDRAM ID for the Samsung Copper LPDDR4 DRAM block.
    pub const DRAM_4GB_COPPER_SAMSUNG: u32 = 3;

    /// The SDRAM ID for the Samsung K4FHE3D4HM-MFCH LPDDR4 DRAM block.
    pub const DRAM_6GB_SAMSUNG_K4FHE3D4HM_MFCH: u32 = 4;

    /// The SDRAM ID for the SK Hynix Copper LPDDR4 DRAM block.
    pub const DRAM_4GB_COPPER_HYNIX: u32 = 5;

    /// The SDRAM ID for the Mouser Micron Copper LPDDR4 DRAM block.
    pub const DRAM_4GB_COPPER_MICRON: u32 = 6;

    /// A Nintendo Switch DRAM profile to be used for training with [`MinervaTrainer`].
    ///
    /// This should be used for devices with SDRAM IDs of `0`, `2`, `3` and `4`.
    ///
    /// [`MinervaTrainer`]: struct.MinervaTrainer.html
    pub const SDRAM0_NX_ABCA2_0_3: &[u8; 49280] = include_bytes!(
    "./minerva_tc/mtc_tables/nintendo_switch/sdram0_nx_abca2_0_3_10NoCfgVersion_V9.8.7_V1.6.bin"
    );

    /// A Nintendo Switch DRAM profile to be used for training with [`MinervaTrainer`].
    ///
    /// This should be used for devices with an SDRAM ID of `1`.
    ///
    /// [`MinervaTrainer`]: struct.MinervaTrainer.html
    pub const SDRAM1_NX_ABCA2_2_0: &[u8; 49280] = include_bytes!(
    "./minerva_tc/mtc_tables/nintendo_switch/sdram1_nx_abca2_2_0_10NoCfgVersion_V9.8.7_V1.6.bin"
    );

    /// A Nintendo Switch DRAM profile to be used for training with [`MinervaTrainer`].
    ///
    /// [`MinervaTrainer`]: struct.MinervaTrainer.html
    pub const SDRAM2_NX_ABCA2_0_3: &[u8; 49280] = include_bytes!(
    "./minerva_tc/mtc_tables/nintendo_switch/sdram2_nx_abca2_0_3_10NoCfgVersion_V9.8.7_V1.6.bin"
    );

    /// A Nintendo Switch DRAM profile to be used for training with [`MinervaTrainer`].
    ///
    /// [`MinervaTrainer`]: struct.MinervaTrainer.html
    pub const SDRAM3_NX_ABCA2_0_3: &[u8; 49280] = include_bytes!(
    "./minerva_tc/mtc_tables/nintendo_switch/sdram3_nx_abca2_0_3_10NoCfgVersion_V9.8.7_V1.6.bin"
    );

    /// A Nintendo Switch DRAM profile to be used for training with [`MinervaTrainer`].
    ///
    /// [`MinervaTrainer`]: struct.MinervaTrainer.html
    pub const SDRAM4_NX_ABCA2_1_0: &[u8; 49280] = include_bytes!(
    "./minerva_tc/mtc_tables/nintendo_switch/sdram4_nx_abca2_1_0_10NoCfgVersion_V9.8.7_V1.6.bin"
    );

    pub fn get_by_sdram_id(id: u32) -> Option<&'static [u8; 49280]> {
        match id {
            DRAM_4GB_SAMSUNG_K4F6E304HB_MGCH => Some(SDRAM0_NX_ABCA2_0_3),
            DRAM_4GB_HYNIX_H9HCNNNBPUMLHR_NLN => Some(SDRAM1_NX_ABCA2_2_0),
            DRAM_4GB_MICRON_MT53B512M32D2NP_062_WT => Some(SDRAM0_NX_ABCA2_0_3),
            DRAM_4GB_COPPER_SAMSUNG => Some(SDRAM0_NX_ABCA2_0_3),
            DRAM_6GB_SAMSUNG_K4FHE3D4HM_MFCH => Some(SDRAM0_NX_ABCA2_0_3),
            _ => None,
        }
    }
}

mod raw {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
