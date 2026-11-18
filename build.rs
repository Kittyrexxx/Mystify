use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // Only run on Windows targets
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let out_dir = env::var("OUT_DIR").unwrap();
        
        let icon_src = Path::new(&manifest_dir).join("src").join("mystify.ico");
        let icon_dest = Path::new(&out_dir).join("mystify.ico");
        let rc_path = Path::new(&out_dir).join("resource.rc");
        let res_path = Path::new(&out_dir).join("resource.o");

        // does exist
        if !icon_src.exists() {
            eprintln!("Error: mystify.ico not found at {:?}", icon_src);
            std::process::exit(1);
        }

        // Copy
        fs::copy(&icon_src, &icon_dest).expect("Failed to copy icon to OUT_DIR");

        // write resource script
        let rc_content = format!("1 ICON \"{}\"\n", icon_dest.to_str().unwrap().replace('\\', "/"));
        fs::write(&rc_path, rc_content).expect("Failed to write resource.rc");

        let status = Command::new("windres")
            .arg(&rc_path)
            .arg("-O")
            .arg("coff")
            .arg(&res_path)
            .status()
            .expect("Failed to execute windres. Ensure MinGW/windres is in your PATH.");

        if !status.success() {
            eprintln!("windres compilation failed");
            std::process::exit(1);
        }

        println!("cargo:rustc-link-arg={}", res_path.to_str().unwrap());
        println!("cargo:rerun-if-changed=src/mystify.ico");
    }
}