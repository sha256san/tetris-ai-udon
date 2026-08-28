use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/hip_kernel.cpp");
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let has_hipcc = Command::new("hipcc").arg("--version").output().is_ok();

    if has_hipcc {
        let obj_path = format!("{}/hip_kernel.o", out_dir);
        let lib_path = format!("{}/libtetris_hip.a", out_dir);

        let status = Command::new("hipcc")
            .args(&[
                "-c",
                "--offload-arch=gfx1200",
                "-O3",
                "-fPIC",
                "-Wno-unused-result",
                "src/hip_kernel.cpp",
                "-o",
                &obj_path,
            ])
            .status();

        if let Ok(s) = status {
            if s.success() {
                let ar_status = Command::new("ar")
                    .args(&["rcs", &lib_path, &obj_path])
                    .status();
                if let Ok(ars) = ar_status {
                    if ars.success() {
                        println!("cargo:rustc-link-search=native={}", out_dir);
                        println!("cargo:rustc-link-lib=static=tetris_hip");
                        println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu");
                        println!("cargo:rustc-link-lib=dylib=amdhip64");
                        println!("cargo:rustc-cfg=has_rocm");
                        return;
                    }
                }
            }
        }
    }
}
