#![no_std]
#![no_main]

#[cfg(not(target_arch = "arm"))]
compile_error!("please pass --target thumbv7em-none-eabihf or use the aliases!");

use core::fmt::Write;

use cortex_m_rt::entry;
use tm4c123x_hal::{self as hal, prelude::*};
use panic_write::PanicHandler;

use ubench::{metrics::*, reporters::*, *};
use ubench_embedded_example_tm4c::*;

const PANIC_DELIM: &str = "++++++++++";
const END_DELIM: &str = "==========";

#[entry]
fn main() -> ! {
    let p = hal::Peripherals::take().unwrap();
    let mut sc = p.SYSCTL.constrain();
    sc.clock_setup.oscillator = hal::sysctl::Oscillator::Main(
        hal::sysctl::CrystalFrequency::_16mhz,
        hal::sysctl::SystemClock::UsePll(hal::sysctl::PllOutputFrequency::_80_00mhz),
    );
    let clocks = sc.clock_setup.freeze();

    // Activate UART
    let mut porta = p.GPIO_PORTA.split(&sc.power_control);
    let uart = hal::serial::Serial::uart0(
        p.UART0,
        porta
            .pa1
            .into_af_push_pull::<hal::gpio::AF1>(&mut porta.control),
        porta
            .pa0
            .into_af_push_pull::<hal::gpio::AF1>(&mut porta.control),
        (),
        (),
        1_500_000_u32.bps(),
        hal::serial::NewlineMode::SwapLFtoCRLF,
        &clocks,
        &sc.power_control,
    );

    // PanicHandler:
    let mut uart = PanicHandler::new_with_hook(
        uart,
        |w, panic_info| {
            writeln!(w, "\n{}", PANIC_DELIM).unwrap();
            writeln!(w, "{panic_info}").unwrap();
            writeln!(w, "{}", PANIC_DELIM).unwrap();
        }
    );

    let mut core_p = hal::CorePeripherals::take().unwrap();
    let mut m = CortexMCycleCount::new(&mut core_p.DWT, &mut core_p.DCB).unwrap();
    let mut r = BasicReporter::new_with_serial::<u8, _, _>(&mut *uart);

    BenchmarkRunner::new()
        .set_iterations(20)
        .add(
            suite("fibonacci comparison", (0..29).step_by(5))
                .add("recursive", Recursive)
                .add("iterative", Iterative)
                .add("closed form", ClosedForm),
        )
        .run(&mut m, &mut r);

    writeln!(uart, "\n{}", END_DELIM).unwrap();

    loop {}
}
