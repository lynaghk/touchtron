#![no_std]
#![no_main]

extern crate panic_semihosting;

mod counter;
mod reporter;

use embedded_hal::digital::v2::{InputPin, OutputPin};
use stm32_usbd::UsbBus;
use stm32f1xx_hal::stm32 as hw;
use stm32f1xx_hal::{gpio, prelude::*, usb, usb::UsbBusType};
use usb_device::prelude::*;

use rtfm::app;

const N: usize = 12;
const M: usize = 2;
const MAX_PACKET_SIZE: u16 = (M * N * 2) as u16;

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
        exti: hw::EXTI,
        counter: counter::Counter<'static, UsbBusType>,
        reporter: reporter::Reporter<'static, UsbBusType, TouchData>,
        midi: usbd_midi::midi_device::MidiClass<'static, UsbBusType>,
        usb_device: UsbDevice<'static, UsbBusType>,
        led: gpio::gpioc::PC13<gpio::Output<gpio::PushPull>>,
        input: gpio::gpiob::PB12<gpio::Input<gpio::PullUp>>,
    }

    // Interrupt handlers used to dispatch software tasks
    extern "C" {
        fn ADC1_2();
    }

    #[init]
    fn init(_cx: init::Context) -> init::LateResources {
        let dp = hw::Peripherals::take().unwrap();
        //Configure button interrupt
        // Enable the alternate function I/O clock (for external interrupts)
        dp.RCC.apb2enr.write(|w| w.afioen().enabled());

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
        led.set_low().ok();

        ////////////////
        //Configure button interrupt
        let mut gpiob = dp.GPIOB.split(&mut rcc.apb2);
        let input = gpiob.pb12.into_pull_up_input(&mut gpiob.crh);

        // Set EXTI12 multiplexers to use port B
        dp.AFIO.exticr4.write(|w| unsafe { w.exti12().bits(0x01) });
        // Enable interrupt on EXTI12
        dp.EXTI.imr.write(|w| w.mr12().set_bit());
        // Set falling and rising trigger selection for EXTI12
        dp.EXTI.ftsr.write(|w| w.tr12().set_bit());
        dp.EXTI.rtsr.write(|w| w.tr12().set_bit());

        ////////////////
        //Configure USB
        static mut USB_BUS: Option<usb_device::bus::UsbBusAllocator<UsbBusType>> = None;

        let gpioa = dp.GPIOA.split(&mut rcc.apb2);
        unsafe {
            USB_BUS = Some(UsbBus::new(usb::Peripheral {
                usb: dp.USB,
                pin_dm: gpioa.pa11,
                pin_dp: gpioa.pa12,
            }));

            //this is a hack to workaround rtfm lifetime issues
            let usb_bus = USB_BUS.as_ref().unwrap();

            let counter = counter::Counter::new(usb_bus, MAX_PACKET_SIZE);
            let reporter = reporter::Reporter::new(usb_bus, MAX_PACKET_SIZE, TouchData::new());
            let midi = usbd_midi::midi_device::MidiClass::new(usb_bus);

            let usb_device = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x16c0, 0x27dd))
                .manufacturer("Fake company")
                .product("Foo")
                .serial_number("TEST")
                //use usbd_midi::data::usb::constants::USB_CLASS_NONE;
                //        .device_class(USB_CLASS_NONE)
                .build();

            init::LateResources {
                exti: dp.EXTI,
                usb_device,
                counter,
                reporter,
                midi,
                led,
                input,
            }
        }
    }

    #[idle(resources = [usb_device, counter, reporter, midi])]
    fn idle(c: idle::Context) -> ! {
        loop {
            //generate random touch data
            //c.resources.reporter.data;

            if !c.resources.usb_device.poll(&mut [
                c.resources.counter,
                c.resources.reporter,
                c.resources.midi,
            ]) {
                continue;
            }
        }
    }

    #[task(priority = 2, binds = EXTI15_10, resources = [exti, led, input])]
    fn exti15_10(mut c: exti15_10::Context) {
        //clear interrupt
        c.resources.exti.pr.modify(|_, w| w.pr12().set_bit());

        let led = &mut c.resources.led;
        if c.resources.input.is_high().unwrap() {
            led.set_high().unwrap();
        } else {
            led.set_low().unwrap();
        }
    }
};
