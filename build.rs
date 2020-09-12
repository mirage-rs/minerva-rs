use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let dst = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let build = dst.join("build");

    let mut cc = cc::Build::new();
    cc.warnings(false).extra_warnings(false).out_dir(&build);

    cc.static_flag(true).shared_flag(false).cargo_metadata(true);

    cc.file("src/minerva_tc/mtc/mtc.c")
        .include("src/minerva_tc/mtc")
        .compile("minerva");

    let bindings = bindgen::Builder::default()
        .use_core()
        .ctypes_prefix("cty")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .header("src/minerva_tc/mtc/mtc.h")
        .header("src/minerva_tc/mtc/mtc_mc_emc_regs.h")
        .whitelist_type("mtc_config_t")
        .whitelist_type("train_mode_t")
        .whitelist_function("minerva_main")
        .whitelist_var("CLOCK_BASE")
        .whitelist_var("CLK_RST_CONTROLLER_CLK_SOURCE_EMC")
        .newtype_enum("train_mode_t")
        .generate()
        .expect("failed to generate rust bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
