#![no_std]
#![no_main]

extern crate panic_semihosting;

use stm32_usbd::UsbBus;
use stm32f0xx_hal::{
    adc::Adc,
    gpio,
    gpio::{
        gpioa::*, gpiob::*, gpioc::*, gpiof::*, Alternate, Analog, Output, PushPull, AF0, AF1, AF2,
        AF3, AF4, AF5, AF6, AF7,
    },
    prelude::*,
    rcc::Rcc,
    stm32 as hw,
    stm32::TIM2,
    usb,
    usb::UsbBusType,
};

use usb_device::prelude::*;

use rtfm::app;

mod reporter;

const N: usize = 15;
const M: usize = 10;

pub struct TouchData {
    pub inner: [u16; N * M],
}

impl TouchData {
    fn new() -> TouchData {
        TouchData {
            inner: [0u16; M * N],
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

//TODO: macro to generate match by "iterating" over enum?
//TODO: HAL turns ADC on/off between each read; probably want to use highest speed "continuous scanning" mode from hardware.
type TouchpadInputPins = (
    PA0<Analog>,
    PA1<Analog>,
    PA2<Analog>,
    PA3<Analog>,
    PA4<Analog>,
    PA5<Analog>,
    PA6<Analog>,
    PA7<Analog>,
    PB0<Analog>,
    PB1<Analog>,
);
type TouchpadOutputPins = (
    PB11<Alternate<AF2>>, //tim2ch4
    PB10<Alternate<AF2>>, //tim2ch3
    PA10<Alternate<AF2>>, //tim1ch3
    PA9<Alternate<AF2>>,  //tim1ch2
    PA8<Alternate<AF2>>,  //tim1ch1
    PB15<Alternate<AF2>>, //tim1ch3n
    PB14<Alternate<AF2>>, //tim1ch2n
    PB13<Alternate<AF2>>, //tim1ch1n
    PB3<Alternate<AF2>>,  //tim2ch2
    PB4<Alternate<AF1>>,  //tim3ch1
    PB5<Alternate<AF1>>,  //tim3ch2
    PB6<Alternate<AF2>>,  //tim16ch1n
    PB7<Alternate<AF2>>,  //tim17ch1n
    PB8<Alternate<AF2>>,  //tim16ch1
    PB9<Alternate<AF2>>,  //tim17ch1
);

pub struct Touchpad {
    adc: Adc,
    input_pins: TouchpadInputPins,
    output_pins: TouchpadOutputPins,
}

impl Touchpad {
    fn read(&mut self, idx: usize) -> Option<u16> {
        //rust makes things "easy"
        match idx {
            0 => self.adc.read(&mut self.input_pins.0).ok(),
            1 => self.adc.read(&mut self.input_pins.1).ok(),
            2 => self.adc.read(&mut self.input_pins.2).ok(),
            3 => self.adc.read(&mut self.input_pins.3).ok(),
            4 => self.adc.read(&mut self.input_pins.4).ok(),
            5 => self.adc.read(&mut self.input_pins.5).ok(),
            6 => self.adc.read(&mut self.input_pins.6).ok(),
            7 => self.adc.read(&mut self.input_pins.7).ok(),
            8 => self.adc.read(&mut self.input_pins.8).ok(),
            9 => self.adc.read(&mut self.input_pins.9).ok(),
            _ => None,
        }
    }

    fn read_all(&mut self) -> TouchData {
        let mut d = TouchData::new();
        let col = 0;
        for row in 0..M {
            let idx = row * N + col;
            d.inner[idx] = self.read(row).unwrap();
        }
        d
    }
}

trait PWM {
    fn start(&self, rcc: &mut hw::RCC);
}

macro_rules! impl_pwm {
    ($TIM:ident, $tim:ident, $timXen:ident, $timXrst:ident, $apbenr:ident, $apbrstr:ident) => {
        impl PWM for $TIM {
            fn start(&self, rcc: &mut hw::RCC) {
                //enable
                rcc.$apbenr.modify(|_, w| w.$timXen().set_bit());

                self.ccmr1_output().modify(|_, w| {
                    //pwm mode 1
                    w.oc1m().bits(0b110);
                    w.oc2m().bits(0b110);
                    //preload
                    w.oc1pe().set_bit();
                    w.oc2pe().set_bit();
                    w
                });

                self.ccmr2_output().modify(|_, w| {
                    //pwm mode 1
                    w.oc3m().bits(0b110);
                    w.oc4m().bits(0b110);
                    //preload
                    w.oc3pe().set_bit();
                    w.oc4pe().set_bit();
                    w
                });

                //auto reload preload enabled
                self.cr1.modify(|_, w| w.arpe().set_bit());

                self.ccer.modify(|_, w| {
                    w.cc1e().set_bit();
                    w.cc2e().set_bit();
                    w.cc3e().set_bit();
                    w.cc4e().set_bit();
                    w
                });

                //set frequency
                let ticks = 5_000;
                self.arr.modify(|_, w| w.arr().bits(ticks));

                //set duty cycle
                //TODO: 16 vs 32 bit timer issue here?
                unsafe {
                    self.ccr1.write(|w| w.bits(ticks / 2));
                    self.ccr2.write(|w| w.bits(ticks / 2));
                    self.ccr3.write(|w| w.bits(ticks / 2));
                    self.ccr4.write(|w| w.bits(ticks / 2));
                }

                //"As the preload registers are transferred to the shadow registers only when an update event occurs, before starting the counter, you have to initialize all the registers by setting the UG bit in the TIMx_EGR register."
                self.egr.write(|w| w.ug().update());

                //finally, start timer
                self.cr1.modify(|_, w| w.cen().set_bit());
            }
        }
    };
}

type Led0 = PC14<Output<PushPull>>;
type Led1 = PC13<Output<PushPull>>;

#[app(device = hw)]
const APP: () = {
    struct Resources {
        leds: (Led0, Led1),
        exti: hw::EXTI,
        usb_device: UsbDevice<'static, UsbBusType>,
        reporter: reporter::Reporter<'static, UsbBusType, TouchData>,
        touchpad: Touchpad,
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

            unsafe {
                dp.GPIOA.ospeedr.write(|w| w.bits(0xffff));
                dp.GPIOB.ospeedr.write(|w| w.bits(0xffff));
            }

            let gpioa = dp.GPIOA.split(&mut rcc);
            let gpiob = dp.GPIOB.split(&mut rcc);
            let gpioc = dp.GPIOC.split(&mut rcc);
            let gpiof = dp.GPIOF.split(&mut rcc);
            let syscfg = dp.SYSCFG;
            let exti = dp.EXTI;

            let switch0 = gpioc.pc15.into_pull_up_input(cs);
            let switch1 = gpiof.pf0.into_pull_up_input(cs);
            let switch2 = gpiof.pf1.into_pull_up_input(cs);

            let mut led0 = gpioc.pc14.into_push_pull_output(cs);
            let mut led1 = gpioc.pc13.into_push_pull_output(cs);
            // led0.set_high().unwrap();
            // led1.set_high().unwrap();

            // Enable external interrupt EXTI0 for PA0
            syscfg.exticr1.write(|w| w.exti0().pa0());

            // Set interrupt request mask for line 0
            exti.imr.modify(|_, w| w.mr0().set_bit());

            // Set interrupt falling trigger for line 0
            exti.ftsr.modify(|_, w| w.tr0().set_bit());

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

            //turn on PWM timers
            //tim2ch4
            let mut rcc_raw = unsafe { hw::Peripherals::steal().RCC }; //HAL consumed this...but I want it.
            impl_pwm!(TIM2, tim2, tim2en, tim2rst, apb1enr, apb1rstr);
            dp.TIM2.start(&mut rcc_raw);
            //impl_pwm!(TIM1: (tim1, tim1en, tim1rst, apb2enr, apb2rstr));

            // TIM3: (tim3, tim3en, tim3rst, apb1enr, apb1rstr),
            // TIM14: (tim14, tim14en, tim14rst, apb1enr, apb1rstr),
            // TIM16: (tim16, tim16en, tim16rst, apb2enr, apb2rstr),
            // TIM17: (tim17, tim17en, tim17rst, apb2enr, apb2rstr),

            let touchpad = Touchpad {
                adc: Adc::new(dp.ADC, &mut rcc),
                input_pins: (
                    gpioa.pa0.into_analog(cs),
                    gpioa.pa1.into_analog(cs),
                    gpioa.pa2.into_analog(cs),
                    gpioa.pa3.into_analog(cs),
                    gpioa.pa4.into_analog(cs),
                    gpioa.pa5.into_analog(cs),
                    gpioa.pa6.into_analog(cs),
                    gpioa.pa7.into_analog(cs),
                    gpiob.pb0.into_analog(cs),
                    gpiob.pb1.into_analog(cs),
                ),
                output_pins: (
                    gpiob.pb11.into_alternate_af2(cs),
                    gpiob.pb10.into_alternate_af2(cs),
                    gpioa.pa10.into_alternate_af2(cs),
                    gpioa.pa9.into_alternate_af2(cs),
                    gpioa.pa8.into_alternate_af2(cs),
                    gpiob.pb15.into_alternate_af2(cs),
                    gpiob.pb14.into_alternate_af2(cs),
                    gpiob.pb13.into_alternate_af2(cs),
                    gpiob.pb3.into_alternate_af2(cs),
                    gpiob.pb4.into_alternate_af1(cs),
                    gpiob.pb5.into_alternate_af1(cs),
                    gpiob.pb6.into_alternate_af2(cs),
                    gpiob.pb7.into_alternate_af2(cs),
                    gpiob.pb8.into_alternate_af2(cs),
                    gpiob.pb9.into_alternate_af2(cs),
                ),
            };

            let reporter = reporter::Reporter::new(&usb_bus);

            let usb_device = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
                .manufacturer("Keming Labs")
                .product("Touchtron")
                //.serial_number("TEST")
                .build();

            init::LateResources {
                exti,
                leds: (led0, led1),
                usb_device,
                reporter,
                touchpad,
            }
        })
    }

    #[idle(resources = [leds, usb_device, touchpad, reporter])]
    fn idle(c: idle::Context) -> ! {
        loop {
            c.resources.reporter.queue(c.resources.touchpad.read_all());

            if !c.resources.usb_device.poll(&mut [c.resources.reporter]) {
                continue;
            }
        }
    }
};
