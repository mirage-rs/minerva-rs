use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let dst = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let build = dst.join("build");

    let mut cc = cc::Build::new();
    cc.warnings(false).out_dir(&build);

    cc.static_flag(true).shared_flag(false).cargo_metadata(true);

    cc.file("src/minerva_tc/mtc/mtc.c")
        .include("src/minerva_tc/mtc")
        .compile("minerva");
}
