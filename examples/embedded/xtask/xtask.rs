#![allow(unused_imports, unreachable_code)]
use std::{env, fs, path::{Path, PathBuf}, io::{self, BufReader, BufRead, Read, Write}, time::Duration};

use crossbeam_utils::thread;
use downloader::{Downloader, Download};
use owo_colors::OwoColorize;
use serialport::{ClearBuffer, DataBits, FlowControl, Parity, StopBits, SerialPort, SerialPortInfo, SerialPortType, UsbPortInfo};
use xshell::{cmd, Shell};
use which::which;

#[derive(Default, Debug)]
pub enum Mode {
    #[default]
    Run,
    Bench,
    Flash,
    Test,
    Debug,
}

const BAUD_RATE: u32 = 1_500_000; // TODO: support grabbing from CLI args

// TODO: use `structopt` or something instead...
fn main() -> Result<(), u32> {
    let mut mode = Mode::default();
    let mut bin = None;

    let mut args = std::env::args().skip(1);
    let mut err = false;
    for a in &mut args {
        match &*a {
            "--bench" => {
                mode = Mode::Bench;
            }
            "--flash" => {
                mode = Mode::Flash;
            }
            "--run" => {
                mode = Mode::Run;
            }
            "--test" => {
                mode = Mode::Test;
            }
            "--debug" => {
                mode = Mode::Debug;
            }
            "--" => {
                break;
            }
            other if other.starts_with("--") => {
                err = true;
                eprintln!("Unknown option: {}", other.strip_prefix("--").unwrap());
            }
            other if other.starts_with('-') => {
                err = true;
                eprintln!("Unknown option: {}", other.strip_prefix('-').unwrap());
            }
            other if bin.is_none() => {
                let p = Path::new(other);
                if p.exists() && p.is_file() {
                    bin = Some(PathBuf::from(other));
                } else {
                    err = true;
                    eprintln!("{other} doesn't seem to exist! (expected a binary path)");
                }
            }
            unexpected => {
                err = true;
                eprintln!("Don't know how to handle `{unexpected}`.");
            }
        }
    }

    let mut args = args.peekable();
    if args.peek().is_some() {
        err = true;
        eprintln!(
            "Got passthrough options `{:?}` for mode {mode:?}. \
            Unfortunately, passing args is not supported for \
            embedded devices.",
            args.collect::<Vec<_>>(),
        );
    }

    if err {
        return Err(1);
    }

    let bin = if let Some(b) = bin {
        b
    } else {
        eprintln!("Need a binary path!");
        return Err(2);
    };

    mode.run(bin)
}

// TODO: support explicitly specifying the device! (once we have an alternative
// to `lm4flash`)
fn find_device() -> String {
    let available_ports = serialport::available_ports()
        .expect("couldn't detect available device");
    let found_port = available_ports
        .into_iter()
        .filter(|p| {
            matches!(
                p,
                SerialPortInfo {
                    port_type: SerialPortType::UsbPort(UsbPortInfo {
                        vid: 0x1cbe,
                        pid: 0x00fd,
                        ..
                    }),
                    ..
                }
            )
        })
        .nth(1)
        .expect("couldn't find a USB Serial device that looks like a TM4C...");

    // eprintln!("using USB port: {found_port:#?}");
    found_port.port_name
}

fn find_llvm_objcopy(sh: &Shell) -> PathBuf {
    const HOST_TRIPLE: &'static str = env!("HOST_TRIPLE");
    const RUSTC: &'static str = env!("RUSTC_PATH");

    // $HOST
    // $RUSTC --print sysroot
    // append /lib/rustlib/$HOST/bin/llvm-objcopy
    //
    // should work with nix and rustup

    let sysroot = cmd!(sh, "{RUSTC} --print sysroot").read().unwrap();

    let mut llvm_objcopy_path = PathBuf::from(sysroot);
    llvm_objcopy_path.extend([
        "lib",
        "rustlib",
        HOST_TRIPLE,
        "bin",
        "llvm-objcopy",
    ]);

    // Check that this absolute path exists and is executable:
    match which(llvm_objcopy_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Couldn't find `llvm-objcopy` in the sysroot! (Error: {e})");
            eprintln!("Maybe try `rustup component add llvm-tools-preview`?");
            eprintln!("(this should be handled by the `rust-toolchain.toml` already)");
            eprintln!("\nSearching $PATH instead...");

            let alt = if let Ok(p) = which("llvm-objcopy") {
                p
            } else if let Ok(p) = which("arm-none-eabi-objcopy") {
                p
            } else {
                panic!("Could not find an `objcopy` to use; see above!");
            };

            eprintln!("\nFound `{}`; using in lieu of a sysroot provided `llvm-objcopy`.", alt.display());
            alt
        }
    }
}

fn find_or_get_lm4flash(sh: &Shell) -> PathBuf {
    const XTASK_ARTIFACT_DIR: &'static str = env!("XTASK_ARTIFACT_DIR");

    // check $PATH
    // check our target folder (infer from arg0 or $CARGO_MANIFEST_DIR)
    //   - actually: build script $OUT_DIR + ../../../../ ?
    // download to our target folder

    // First: check $PATH.
    let bin_name = format!("lm4flash{}", env::consts::EXE_SUFFIX);
    if let Ok(p) = which(&bin_name) {
        return p
    }

    // Next: check the artifact dir.
    let bin_path = Path::new(XTASK_ARTIFACT_DIR).join(&bin_name);
    if let Ok(p) = which(&bin_path) {
        return p
    }

    // Last: download it.
    const DOWNLOAD_PREFIX: &'static str = "https://github.com/ut-utp/.github/wiki/assets/binaries/";
    let suffix = match (env::consts::OS, env::consts::ARCH) {
        ("macos" | "ios", "x86_64" | "aarch64") => "macos/lm4flash",
        ("linux" | "freebsd" | "dragonfly" | "netbsd" | "openbsd" | "solaris" | "android", arch @ "x86_64" | arch @ "aarch64") => {
            // I think all of these have some degree of binary compatibility with Linux?
            match arch {
                "x86_64" => "linux-amd64/lm4flash",
                "aarch64" => "linux-arm64/lm4flash",
                _ => unreachable!(),
            }
        },
        ("windows", "x86_64") => "windows/lm4flash.exe",
        (os, arch) => panic!("
            Sorry, we don't have `lm4flash` binaries for '{os}' running on '{arch}'.

            Please either consult your system's package manager, try to build the package
            from source (https://github.com/utzig/lm4tools), or see these instructions
            about building the package with nix: https://github.com/ut-utp/.github/wiki/lm4flash-Binaries
        "),
    };

    fs::create_dir_all(XTASK_ARTIFACT_DIR).unwrap();
    let mut d = Downloader::builder()
        .download_folder(Path::new(XTASK_ARTIFACT_DIR))
        .build()
        .unwrap();
    d.download(&[
        Download::new(&format!("{}/{}", DOWNLOAD_PREFIX, suffix))
            .file_name(Path::new(&bin_name))
    ]).unwrap()[0].as_ref().unwrap();

    // Set permissions:
    if cfg!(unix) {
        use std::os::unix::fs::PermissionsExt;

        let metadata = fs::metadata(&bin_path).unwrap();
        let mut perms = metadata.permissions();

        perms.set_mode(0o755);
        fs::set_permissions(&bin_path, perms).unwrap();
    }

    // If on macOS we'll need to run `codesign` too:
    if cfg!(target_os = "macos") {
        if let Ok(codesign) = which("codesign") {
            cmd!(sh, "{codesign} -s - -f {bin_path}").run().unwrap();
        } else {
            eprintln!("
                Could not find the `codesign` tool in $PATH!! Hopefully this just \
                means your macOS version does not require codesigning.
            ");
        }
    }

    // Finally try to run the binary:
    cmd!(sh, "{bin_path} -V").quiet().run().expect("a working `lm4flash` binary");

    bin_path
}

// todo: try to get the _serial_ to specify to `lm4flash` which
// board to flash..
fn flash_program(sh: &Shell, elf_binary: &Path, _device_port: &str) {
    let lm4flash = find_or_get_lm4flash(sh);
    let objcopy = find_llvm_objcopy(sh);
    let axf_bin_path = {
        let p = elf_binary.parent().unwrap();
        let f = elf_binary.file_stem().unwrap();
        p.join(format!("{:?}.axf", f))
    };

    cmd!(sh, "{objcopy} -O binary {elf_binary} {axf_bin_path}").quiet().run().unwrap();
    cmd!(sh, "{lm4flash} -E -v {axf_bin_path}").quiet().ignore_stdout().run().unwrap()
}

impl Mode {
    fn print(&self, bin: &Path, dev: &str) {
        let verb = match self {
            Mode::Run => "Running",
            Mode::Bench => "Benchmarking",
            Mode::Flash => "Flashing",
            Mode::Test => "Testing",
            Mode::Debug => "Debugging",
        };

        eprintln!("{:>12} {:?} {} {}", verb.green().bold(), bin.file_name().unwrap(), "on".dimmed(), dev.bold());
    }

    fn run(&self, bin: PathBuf) -> Result<(), u32> {
        let dev_path = find_device();
        self.print(&bin, &dev_path);

        let sh = Shell::new().unwrap();

        // This is a bit tricky. We want to only see output from the current
        // execution of the program so we want to only start reading from the
        // serial port _after_ `lm4flash` has started.
        //
        // But we also want to clear the OS's buffer so we don't get old output.
        // We can't do this _after_ `lm4flash` has run because then we'll lose
        // some of the new output and we can't do this _before_ `lm4flash` runs
        // because then we might get some of the old output from between when
        // `lm4flash` runs and when we clear the buffer.
        //
        // So, we clear the buffer _while_ `lm4flash` is running.
        let mut dev = serialport::new(&dev_path, BAUD_RATE)
            .data_bits(DataBits::Eight)
            .flow_control(FlowControl::None)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .timeout(Duration::from_secs(30))
            .open_native().unwrap();

        // Start up `lm4flash`:
        thread::scope(|s| {
            s.spawn(move |_| {
                flash_program(&sh, &bin, &dev_path);
            });

            // wait a little bit:
            std::thread::sleep(Duration::from_millis(80));

            // clear the buffer:
            dev.clear(ClearBuffer::All).unwrap();
        }).unwrap();

        if let Mode::Flash = self {
            return Ok(())
        }
        if let Mode::Debug = self {
            // exec into gdb, etc.
            todo!()
        }

        if let Mode::Run | Mode::Bench = self {
            // Show serial output.
            // let monitor = env::var("SERIAL_MONITOR").unwrap_or("picocom")

            // First try `picocom` if available.
            // TODO
            // print: ctrl + a, ctrl + q to quit

            // Otherwise, warn and drop into this output only
            // facsimile.
            eprintln!("{}", "`picocom` not found, using built-in output-only monitor".yellow());

            let mut out = io::stdout();
            loop {
                match io::copy(&mut dev, &mut out) {
                    Ok(_) => {},
                    Err(err) => eprintln!("error: {err:?}"),
                }
            }
        }

        // let mut dev = BufReader::new(dev);
        // let mut buf = String::new();
        // let mut out = io::stdout();
        // loop {
        //     match io::copy(&mut dev, &mut out) {
        //         Ok(_) => {},
        //         Err(err) => eprintln!("error: {err:?}"),
        //     }
        // }

        // loop {
        //     match dev.read_line(&mut buf) {
        //         Ok(n) => print!("{}", buf),
        //         Err(err) => eprintln!("error: {err:?}"),
        //     }
        //     buf.clear();
        // }

        // fn process(inp: &mut impl Read, sink: &mut impl Write, chk: impl FnMut(&s))

        match self {
            Mode::Bench => {
                // Attach a console but be looking to grab the
                // benchmarking output for post processing if the
                // flags say to do so.
                todo!()
            },
            Mode::Test => {
                // Like `Bench` but different post processing.
                todo!()
            },
            Mode::Run => unreachable!(),
            Mode::Debug => unreachable!(),
            Mode::Flash => unreachable!(),
        }
    }
}
