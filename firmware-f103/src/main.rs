#![no_std]
#![no_main]

//CDC-ACM serial port example using polling in a busy loop.
//copied from https://github.com/stm32-rs/stm32-usbd-examples/blob/master/example-stm32f072rb/src/main.rs

extern crate panic_semihosting;

mod counter;

use cortex_m_rt::entry;
use embedded_hal::digital::v2::OutputPin;
use stm32_usbd::UsbBus;
use stm32f1xx_hal::{prelude::*, stm32, usb};
use usb_device::prelude::*;

#[entry]
fn main() -> ! {
    let mut dp = stm32::Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();

    let clocks = rcc
        .cfgr
        .use_hse(8.mhz())
        .sysclk(48.mhz())
        .pclk1(24.mhz())
        .freeze(&mut flash.acr);

    assert!(clocks.usbclk_valid());

    // Configure the on-board LED (PC13, green)
    let mut gpioc = dp.GPIOC.split(&mut rcc.apb2);
    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
    led.set_high().ok();

    let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);

    let usb_bus = UsbBus::new(usb::Peripheral {
        usb: dp.USB,
        pin_dm: gpioa.pa11,
        pin_dp: gpioa.pa12,
    });

    let max_packet_size = 16;
    let mut counter = counter::Counter::new(&usb_bus, max_packet_size);

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .manufacturer("Fake company")
        .product("Foo")
        .serial_number("TEST")
        .build();

    loop {
        if !usb_dev.poll(&mut [&mut counter]) {
            continue;
        }
    }
}
