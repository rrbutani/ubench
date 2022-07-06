#![no_std]
#![no_main]

#[cfg(not(target_arch = "arm"))]
compile_error!("please pass --target thumbv7em-none-eabihf or use the aliases!");

use panic_halt as _;

use cortex_m_rt::entry;
use tm4c123x_hal::{self as hal, prelude::*};

use ubench_embedded_example_tm4c::*;
use ubench::{*, metrics::*, reporters::*};

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
    let mut uart = hal::serial::Serial::uart0(
        p.UART0,
        porta
            .pa1
            .into_af_push_pull::<hal::gpio::AF1>(&mut porta.control),
        porta
            .pa0
            .into_af_push_pull::<hal::gpio::AF1>(&mut porta.control),
        (),
        (),
        115200_u32.bps(),
        hal::serial::NewlineMode::SwapLFtoCRLF,
        &clocks,
        &sc.power_control,
    );

    let mut core_p = hal::CorePeripherals::take().unwrap();
    let mut m = CortexMCycleCount::new(&mut core_p.DWT, &mut core_p.DCB).unwrap();
    let mut r = BasicReporter::new_with_serial::<u8, _, _>(&mut uart);

    BenchmarkRunner::new()
        .set_iterations(20)
        .add(
            suite("fibonacci comparison", (0..36).step_by(5))
            .add("recursive", Recursive)
            .add("iterative", Iterative)
            .add("closed form", ClosedForm)
        )
        .run(&mut m, &mut r);

    loop { }
}
