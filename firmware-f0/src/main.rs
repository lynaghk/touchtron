#![no_std]
#![no_main]

extern crate panic_semihosting;

use stm32_usbd::UsbBus;
use stm32f0xx_hal::stm32 as hw;
use stm32f0xx_hal::{gpio, prelude::*, usb, usb::UsbBusType};
use usb_device::prelude::*;

use rtfm::app;

mod reporter;

const N: usize = 12;
const M: usize = 8;

pub struct TouchData {
    pub inner: [u16; N * M],
}

impl TouchData {
    fn new() -> TouchData {
        TouchData {
            inner: [1u16; M * N],
        }
    }
}

//TODO: is this the best way to get array of u16 into a slice of u8?
impl AsRef<[u8]> for TouchData {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts_mut(self.inner.as_ptr() as *mut u8, self.inner.len() * 2)
        }
    }
}

#[app(device = hw)]
const APP: () = {
    struct Resources {
        led: gpio::gpioc::PC8<gpio::Output<gpio::PushPull>>,
        exti: hw::EXTI,
        usb_device: UsbDevice<'static, UsbBusType>,
        reporter: reporter::Reporter<'static, UsbBusType, TouchData>,
    }

    #[init]
    fn init(_cx: init::Context) -> init::LateResources {
        // init() is already run with interrupts disabled,
        // but we need a CriticalSection to pass to some stm32f0xx_hal methods
        cortex_m::interrupt::free(move |cs| {
            let mut dp = hw::Peripherals::take().unwrap();

            // enable SYSCFG clock
            dp.RCC.apb2enr.modify(|_, w| w.syscfgen().enabled());

            let mut rcc = dp
                .RCC
                .configure()
                .hsi48()
                .enable_crs(dp.CRS)
                .sysclk(48.mhz())
                .pclk(24.mhz())
                .freeze(&mut dp.FLASH);

            let gpioa = dp.GPIOA.split(&mut rcc);
            let gpioc = dp.GPIOC.split(&mut rcc);
            let syscfg = dp.SYSCFG;
            let exti = dp.EXTI;

            // user button at PA0
            let _ = gpioa.pa0.into_pull_down_input(cs);

            // Enable external interrupt EXTI0 for PA0
            syscfg.exticr1.write(|w| w.exti0().pa0());

            // Set interrupt request mask for line 0
            exti.imr.modify(|_, w| w.mr0().set_bit());

            // Set interrupt falling trigger for line 0
            exti.ftsr.modify(|_, w| w.tr0().set_bit());

            let mut led = gpioc.pc8.into_push_pull_output(cs);
            led.set_low().unwrap();

            // UsbDevice take refs to usb_bus but outlive init()
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

            let reporter = reporter::Reporter::new(&usb_bus);

            let usb_device = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
                .manufacturer("Fake company")
                .product("Serial port")
                .serial_number("TEST")
                .build();

            init::LateResources {
                exti,
                led,
                usb_device,
                reporter,
            }
        })
    }

    #[idle(resources = [led, usb_device, reporter])]
    fn idle(c: idle::Context) -> ! {
        loop {
            c.resources.reporter.queue(TouchData::new());

            if !c.resources.usb_device.poll(&mut [c.resources.reporter]) {
                continue;
            }
        }
    }
};
