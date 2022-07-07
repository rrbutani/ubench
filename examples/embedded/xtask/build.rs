use std::{env, fs, ffi, path::{Path, PathBuf}};

fn var(name: impl AsRef<ffi::OsStr>) -> Result<String, env::VarError> {
    println!("cargo:rerun-if-env-changed=RUSTC");
    env::var(name)
}

fn which(query: impl AsRef<ffi::OsStr>) -> which::Result<PathBuf> {
    println!("cargo:rerun-if-env-changed=PATH");
    which::which(query)
}

fn main() {
    // Tell `xtask` where rustc is so it can find the sysroot:
    let rustc = var("RUSTC").expect("cargo sets $RUSTC for build scripts");
    let rustc = Path::new(&rustc);

    let rustc = if rustc.exists() && rustc.is_absolute() {
        rustc.to_path_buf()
    } else {
        // Assume it's a command that we need to get the path of with `which`:
        //
        // (this will work with relative paths too; we'll declare false
        // dependence on `$PATH` in these cases but that's fine)
        which(rustc).unwrap()
    };
    println!("cargo:rerun-if-changed={}", rustc.display());
    println!("cargo:rustc-env=RUSTC_PATH={}", rustc.display());


    // Tell `xtask` where it can stick its artifacts:
    let out_dir = var("OUT_DIR").expect("cargo sets $OUT_DIR for build scripts");
    let out_dir = Path::new(&out_dir);

    // We seem to get paths like `target/debug/build/xtask-55dae522e1d36b80/out`
    let potential_target_dir = {
        let mut path = Some(out_dir);

        // Try to go 4 directories up!
        for _ in 0..4 {
            if let Some(p) = path {
                path = p.parent();
            }
        }

        path
    };
    let target_dir = if let Some(p) = potential_target_dir.filter(|p| p.file_name().unwrap() == "target") {
        p
    } else {
        out_dir
    };
    let artifact_dir = target_dir.join("xtask");
    fs::create_dir_all(&artifact_dir).unwrap();
    println!("cargo:rustc-env=XTASK_ARTIFACT_DIR={}", artifact_dir.display());

    // Forward the host target triple:
    println!("cargo:rustc-env=HOST_TRIPLE={}", var("HOST").unwrap());
}
