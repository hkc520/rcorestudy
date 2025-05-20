use std::{env, fs, path::PathBuf};

#[allow(unused_macros)]
macro_rules! display {
    ($fmt:expr) => (println!("cargo:warning={}", format!($fmt)));
    ($fmt:expr, $($arg:tt)*) => (println!(concat!("cargo:warning=", $fmt), $($arg)*));
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("can't find manifest dir"));

    // 生成空的 ramdisk 段，避免符号未定义
    fs::write(
        out_dir.join("inc.S"),
        ".section .data\n\
         .global ramdisk_start\n\
         .global ramdisk_end\n\
         .p2align 12\n\
         ramdisk_start:\n\
         .byte 0\n\
         ramdisk_end:",
    )
    .expect("can't write ram file to out_dir");

    println!("cargo:rerun-if-changed=build.rs");
}
