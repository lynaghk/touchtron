#![no_std]
#![no_main]

//CDC-ACM serial port example using polling in a busy loop.
//copied from https://github.com/stm32-rs/stm32-usbd-examples/blob/master/example-stm32f072rb/src/main.rs

extern crate panic_semihosting;

use stm32_usbd::UsbBus;
use stm32f0xx_hal::stm32 as hw;
use stm32f0xx_hal::{gpio, prelude::*, usb, usb::UsbBusType};
use usb_device::prelude::*;
use usbd_serial::{SerialPort, USB_CLASS_CDC};

use rtfm::app;

#[app(device = hw)]
const APP: () = {
    struct Resources {
        led: gpio::gpioc::PC8<gpio::Output<gpio::PushPull>>,
        usb_device: UsbDevice<'static, UsbBusType>,
        serial: SerialPort<'static, UsbBusType>,
    }

    #[init]
    fn init(_cx: init::Context) -> init::LateResources {
        let mut dp = hw::Peripherals::take().unwrap();

        let mut rcc = dp
            .RCC
            .configure()
            .hsi48()
            .enable_crs(dp.CRS)
            .sysclk(48.mhz())
            .pclk(24.mhz())
            .freeze(&mut dp.FLASH);

        let gpioc = dp.GPIOC.split(&mut rcc);
        let mut led = cortex_m::interrupt::free(|cs| gpioc.pc8.into_push_pull_output(cs));
        led.set_high().unwrap();

        let gpioa = dp.GPIOA.split(&mut rcc);

        // SerialPort and UsbDevice take refs to usb_bus but outlive init()
        // so usb_bus must be owned with static lifetime
        // see https://github.com/Rahix/shared-bus/issues/4#issuecomment-503512441
        static mut USB_BUS: Option<usb_device::bus::UsbBusAllocator<UsbBusType>> = None;

        let usb_bus = unsafe {
            USB_BUS = Some(UsbBus::new(usb::Peripheral {
                usb: dp.USB,
                pin_dm: gpioa.pa11,
                pin_dp: gpioa.pa12,
            }));
            USB_BUS.as_ref().unwrap()
        };

        let serial = SerialPort::new(&usb_bus);

        let usb_device = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("Fake company")
            .product("Serial port")
            .serial_number("TEST")
            .device_class(USB_CLASS_CDC)
            .build();

        init::LateResources {
            led: led,
            usb_device: usb_device,
            serial: serial,
        }
    }

    #[idle(resources = [led, usb_device, serial])]
    fn idle(cx: idle::Context) -> ! {
        loop {
            if !cx.resources.usb_device.poll(&mut [cx.resources.serial]) {
                continue;
            }

            let mut buf = [0u8; 64];

            match cx.resources.serial.read(&mut buf) {
                Ok(count) if count > 0 => {
                    // Echo back in upper case
                    for c in buf[0..count].iter_mut() {
                        if 0x61 <= *c && *c <= 0x7a {
                            *c &= !0x20;
                        }
                    }

                    let mut write_offset = 0;
                    while write_offset < count {
                        match cx.resources.serial.write(&buf[write_offset..count]) {
                            Ok(len) if len > 0 => {
                                write_offset += len;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }
};
