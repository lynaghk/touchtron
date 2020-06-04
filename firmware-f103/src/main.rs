#![no_std]
#![no_main]

//CDC-ACM serial port example using polling in a busy loop.
//copied from https://github.com/stm32-rs/stm32-usbd-examples/blob/master/example-stm32f072rb/src/main.rs

extern crate panic_semihosting;

mod counter;

use embedded_hal::digital::v2::OutputPin;
use stm32_usbd::UsbBus;
use stm32f1xx_hal::stm32 as hw;
use stm32f1xx_hal::{gpio, prelude::*, usb, usb::UsbBusType};
use usb_device::prelude::*;

use rtfm::app;

#[app(device = hw)]
const APP: () = {
    struct Resources {
        exti: hw::EXTI,
        counter: counter::Counter<'static, UsbBusType>,
        midi: usbd_midi::midi_device::MidiClass<'static, UsbBusType>,
        usb_device: UsbDevice<'static, UsbBusType>,
        led: gpio::gpioc::PC13<gpio::Output<gpio::PushPull>>,
    }

    // Interrupt handlers used to dispatch software tasks
    extern "C" {
        fn ADC1_2();
    }

    #[init]
    fn init(mut cx: init::Context) -> init::LateResources {
        let mut dp = hw::Peripherals::take().unwrap();

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
        gpiob.pb11.into_pull_up_input(&mut gpiob.crh);

        // Set EXTI11 multiplexers to use port B
        dp.AFIO.exticr3.write(|w| unsafe { w.exti11().bits(0x01) });
        // Enable interrupt on EXTI11
        dp.EXTI.imr.write(|w| w.mr11().set_bit());
        // Set falling trigger selection for EXTI11
        dp.EXTI.ftsr.write(|w| w.tr11().set_bit());

        ////////////////
        //Configure USB
        static mut USB_BUS: Option<usb_device::bus::UsbBusAllocator<UsbBusType>> = None;

        let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);
        unsafe {
            USB_BUS = Some(UsbBus::new(usb::Peripheral {
                usb: dp.USB,
                pin_dm: gpioa.pa11,
                pin_dp: gpioa.pa12,
            }));

            //this is a hack to workaround rtfm lifetime issues
            let usb_bus = USB_BUS.as_ref().unwrap();

            let max_packet_size = 16;
            let mut counter = counter::Counter::new(usb_bus, max_packet_size);
            let mut midi = usbd_midi::midi_device::MidiClass::new(usb_bus);

            let mut usb_device = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x16c0, 0x27dd))
                .manufacturer("Fake company")
                .product("Foo")
                .serial_number("TEST")
                //use usbd_midi::data::usb::constants::USB_CLASS_NONE;
                //        .device_class(USB_CLASS_NONE)
                .build();

            init::LateResources {
                exti: dp.EXTI,
                usb_device: usb_device,
                counter: counter,
                midi: midi,
                led: led,
            }
        }
    }

    #[idle(resources = [usb_device, counter, midi])]
    fn idle(c: idle::Context) -> ! {
        loop {
            if !c
                .resources
                .usb_device
                .poll(&mut [c.resources.counter, c.resources.midi])
            {
                continue;
            }
        }
    }

    #[task(priority = 2, binds = EXTI15_10, resources = [exti, led])]
    fn exti15_10(mut c: exti15_10::Context) {
        //clear interrupt
        c.resources.exti.pr.modify(|_, w| w.pr11().set_bit());

        let led = &mut c.resources.led;

        if led.is_set_high().unwrap() {
            led.set_low().unwrap();
        } else {
            led.set_high().unwrap();
        }
    }
};
