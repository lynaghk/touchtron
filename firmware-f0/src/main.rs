#![no_std]
#![no_main]

extern crate panic_semihosting;
use cortex_m::asm::{bkpt, delay, wfi};
use stm32_usbd::UsbBus;
use stm32f0xx_hal::{
    gpio,
    gpio::{
        gpioa::*, gpiob::*, gpioc::*, gpiof::*, Alternate, Analog, Output, PushPull, AF0, AF1, AF2,
        AF3, AF4, AF5, AF6, AF7,
    },
    prelude::*,
    rcc::Rcc,
    stm32 as hw,
    stm32::{TIM1, TIM16, TIM17, TIM2, TIM3},
    usb::UsbBusType,
};

use usb_device::prelude::*;

use rtfm::app;

mod adc;
mod reporter;
use adc::Adc;

const N: usize = 15;
const M: usize = 10;
const TOUCH_DATA_LEN: usize = 1 + M * N; //one extra val at start for the PWM period

const INITIAL_PERIOD: u16 = 4; //Multitouch paper suggests peak SNR at 10 MHz freq

#[derive(Copy, Clone)]
pub struct TouchData {
    pub inner: [u16; TOUCH_DATA_LEN],
}

impl TouchData {
    fn new() -> TouchData {
        TouchData {
            inner: [0u16; TOUCH_DATA_LEN],
        }
    }

    fn clear(&mut self) {
        for idx in 0..TOUCH_DATA_LEN {
            self.inner[idx] = 0;
        }
    }

    fn take(&mut self) -> Self {
        let copy = self.clone();
        self.clear();
        copy
    }

    fn add(&mut self, other: &TouchData) {
        self.inner[0] += 1;
        for idx in 1..TOUCH_DATA_LEN {
            self.inner[idx] += other.inner[idx];
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

macro_rules! PwmOutput {
    ($O: ident, $pin: ty, $TIM: ident, $ccXe: ident) => {
        struct $O($pin);

        impl $O {
            fn on(&self) {
                unsafe { &(*$TIM::ptr()) }
                    .ccer
                    .modify(|_, w| w.$ccXe().set_bit());
            }
            fn off(&self) {
                unsafe { &(*$TIM::ptr()) }
                    .ccer
                    .modify(|_, w| w.$ccXe().clear_bit());
            }
        }
    };
}

//TODO: how much code does this generate?
PwmOutput!(B0, PB11<Alternate<AF2>>, TIM2, cc4e);
PwmOutput!(B1, PB10<Alternate<AF2>>, TIM2, cc3e);
PwmOutput!(B2, PA10<Alternate<AF2>>, TIM1, cc3e);
PwmOutput!(B3, PA9<Alternate<AF2>>, TIM1, cc2e);
PwmOutput!(B4, PA8<Alternate<AF2>>, TIM1, cc1e);
PwmOutput!(B5, PB15<Alternate<AF2>>, TIM1, cc3ne);
PwmOutput!(B6, PB14<Alternate<AF2>>, TIM1, cc2ne);
PwmOutput!(B7, PB13<Alternate<AF2>>, TIM1, cc1ne);
PwmOutput!(B8, PB3<Alternate<AF2>>, TIM2, cc2e);
PwmOutput!(B9, PB4<Alternate<AF1>>, TIM3, cc1e);
PwmOutput!(B10, PB5<Alternate<AF1>>, TIM3, cc2e);
PwmOutput!(B11, PB6<Alternate<AF2>>, TIM16, cc1e);
PwmOutput!(B12, PB7<Alternate<AF2>>, TIM17, cc1e);
PwmOutput!(B13, PB8<Alternate<AF2>>, TIM16, cc1e);
PwmOutput!(B14, PB9<Alternate<AF2>>, TIM17, cc1e);

type TouchpadOutputPins = (
    B0,  //tim2ch4
    B1,  //tim2ch3
    B2,  //tim1ch3
    B3,  //tim1ch2
    B4,  //tim1ch1
    B5,  //tim1ch3n
    B6,  //tim1ch2n
    B7,  //tim1ch1n
    B8,  //tim2ch2
    B9,  //tim3ch1
    B10, //tim3ch2
    B11, //tim16ch1n
    B12, //tim17ch1n
    B13, //tim16ch1
    B14, //tim17ch1
);

pub struct Touchpad {
    adc: Adc,
    timers: (TIM1, TIM2, TIM3, TIM16, TIM17),
    input_pins: TouchpadInputPins,
    output_pins: TouchpadOutputPins,
    period: u16,
    in_progress: TouchData,
    current_row: usize,
    current_col: usize,
}

impl Touchpad {
    fn on(&mut self, idx: usize) {
        match idx {
            0 => self.output_pins.0.on(),
            1 => self.output_pins.1.on(),
            2 => self.output_pins.2.on(),
            3 => self.output_pins.3.on(),
            4 => self.output_pins.4.on(),
            5 => self.output_pins.5.on(),
            6 => self.output_pins.6.on(),
            7 => self.output_pins.7.on(),
            8 => self.output_pins.8.on(),
            9 => self.output_pins.9.on(),
            10 => self.output_pins.10.on(),
            11 => self.output_pins.11.on(),
            12 => self.output_pins.12.on(),
            13 => self.output_pins.13.on(),
            14 => self.output_pins.14.on(),
            _ => {}
        }
    }
    fn off(&mut self, idx: usize) {
        match idx {
            0 => self.output_pins.0.off(),
            1 => self.output_pins.1.off(),
            2 => self.output_pins.2.off(),
            3 => self.output_pins.3.off(),
            4 => self.output_pins.4.off(),
            5 => self.output_pins.5.off(),
            6 => self.output_pins.6.off(),
            7 => self.output_pins.7.off(),
            8 => self.output_pins.8.off(),
            9 => self.output_pins.9.off(),
            10 => self.output_pins.10.off(),
            11 => self.output_pins.11.off(),
            12 => self.output_pins.12.off(),
            13 => self.output_pins.13.off(),
            14 => self.output_pins.14.off(),
            _ => {}
        }
    }

    pub fn adc_interrupt(&mut self) -> Option<TouchData> {
        let status = self.adc.rb.isr.read();
        //self.adc.rb.isr.modify(|_, w| w.eoc().clear());
        // //cortex_m_semihosting::hprintln!("{:#018b}", status.bits()).unwrap();
        if status.ovr().is_overrun() {
            panic!("overrun");
        }
        if status.eoc().is_complete() {
            //reading the register will clear the eoc flag
            self.next_row(self.adc.rb.dr.read().bits() as u16)
        } else {
            None
        }
    }

    fn next_row(&mut self, reading: u16) -> Option<TouchData> {
        self.in_progress.inner[1 + N * self.current_row + self.current_col] = reading;
        self.current_row = (self.current_row + 1) % M;
        //        delay(200);
        self.adc.start(self.current_row as u8);

        if 0 == self.current_row {
            self.next_column()
        } else {
            None
        }
    }

    fn next_column(&mut self) -> Option<TouchData> {
        self.off(self.current_col);
        self.current_col = (self.current_col + 1) % N;
        self.on(self.current_col);

        if 0 == self.current_col {
            Some(self.in_progress.take())
        } else {
            None
        }
    }

    pub fn set_period(&mut self, period: u16) {
        self.period = period;
        self.timers.0.set_period(period);
        self.timers.1.set_period(period);
        self.timers.2.set_period(period);
    }
}

trait PWM {
    fn start(&self, rcc: &mut hw::RCC);
    fn set_period(&self, ticks: u16);
}

macro_rules! impl_pwm {
    ($TIM:ident, $tim:ident, $timXen:ident, $apbenr:ident) => {
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

                self.set_period(INITIAL_PERIOD);

                //finally, start timer
                self.cr1.modify(|_, w| w.cen().set_bit());
            }

            fn set_period(&self, ticks: u16) {
                self.arr.modify(|_, w| w.arr().bits(ticks.into()));

                //set 50% duty cycle
                unsafe {
                    self.ccr1.write(|w| w.bits((ticks / 2).into()));
                    self.ccr2.write(|w| w.bits((ticks / 2).into()));
                    self.ccr3.write(|w| w.bits((ticks / 2).into()));
                    self.ccr4.write(|w| w.bits((ticks / 2).into()));

                    // self.ccr1.write(|w| w.bits(1));
                    // self.ccr2.write(|w| w.bits(1));
                    // self.ccr1.write(|w| w.bits(1));
                    // self.ccr4.write(|w| w.bits(1));
                }

                //"As the preload registers are transferred to the shadow registers only when an update event occurs, before starting the counter, you have to initialize all the registers by setting the UG bit in the TIMx_EGR register."
                self.egr.write(|w| w.ug().update());
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
                //TODO: should this be 48 too?
                .pclk(48.mhz())
                .freeze(&mut dp.FLASH);

            //just make all gpio ports high speed
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

            //TODO: how the hell am I supposed to use this method? it's public, but the ::new constructor on UsbBus doesn't return the Bus, it wraps it in an allocator! There's a freeze method on the allocator that returns the bus, but it's only public within the usb crate. ugh.
            //UsbBusType::force_reenumeration(&dp.USB, || {});

            let usb_bus = unsafe {
                USB_BUS = Some(UsbBus::new(stm32f0xx_hal::usb::Peripheral {
                    usb: dp.USB,
                    pin_dm: gpioa.pa11,
                    pin_dp: gpioa.pa12,
                }));
                USB_BUS.as_ref().unwrap()
            };

            //turn on PWM timers

            impl_pwm!(TIM1, tim1, tim1en, apb2enr);
            impl_pwm!(TIM2, tim2, tim2en, apb1enr);
            impl_pwm!(TIM3, tim3, tim3en, apb1enr);
            // impl_pwm!(TIM16, tim16, tim16en, apb2enr);
            // impl_pwm!(TIM17, tim17, tim17en, apb2enr);

            let mut rcc_raw = unsafe { hw::Peripherals::steal().RCC }; //HAL consumed this...but I want it.

            dp.TIM1.start(&mut rcc_raw);
            //TIM1 has special "main output enable" bit that must be set before it emits signal
            dp.TIM1.bdtr.modify(|_, w| {
                w.moe().set_bit();
                w.ossr().set_bit();
                w
            });

            dp.TIM2.start(&mut rcc_raw);
            dp.TIM3.start(&mut rcc_raw);
            // dp.TIM16.start(&mut rcc_raw);
            // dp.TIM17.start(&mut rcc_raw);

            let mut touchpad = Touchpad {
                current_row: 0,
                current_col: 0,
                period: INITIAL_PERIOD,
                in_progress: TouchData::new(),
                timers: (dp.TIM1, dp.TIM2, dp.TIM3, dp.TIM16, dp.TIM17),
                adc: Adc::new(dp.ADC, &mut rcc_raw),
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
                    B0(gpiob.pb11.into_alternate_af2(cs)),
                    B1(gpiob.pb10.into_alternate_af2(cs)),
                    B2(gpioa.pa10.into_alternate_af2(cs)),
                    B3(gpioa.pa9.into_alternate_af2(cs)),
                    B4(gpioa.pa8.into_alternate_af2(cs)),
                    B5(gpiob.pb15.into_alternate_af2(cs)),
                    B6(gpiob.pb14.into_alternate_af2(cs)),
                    B7(gpiob.pb13.into_alternate_af2(cs)),
                    B8(gpiob.pb3.into_alternate_af2(cs)),
                    B9(gpiob.pb4.into_alternate_af1(cs)),
                    B10(gpiob.pb5.into_alternate_af1(cs)),
                    B11(gpiob.pb6.into_alternate_af2(cs)),
                    B12(gpiob.pb7.into_alternate_af2(cs)),
                    B13(gpiob.pb8.into_alternate_af2(cs)),
                    B14(gpiob.pb9.into_alternate_af2(cs)),
                ),
            };

            //options: change pin mode at runtime, but this causes issues with the tuple types --- need to overwrite enum in place or erase typestate or something
            //could also impl my own trait on each af pin to enable/disable associated timer channel

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

    #[idle(resources = [touchpad])]
    fn idle(mut c: idle::Context) -> ! {
        c.resources.touchpad.lock(|t| {
            t.adc.start(0);
        });

        loop {
            wfi();
        }
    }

    #[task(binds = ADC_COMP, priority = 3, resources = [touchpad, reporter])]
    fn handle_adc(mut c: handle_adc::Context) {
        if let Some(latest_reading) = c.resources.touchpad.adc_interrupt() {
            match c.resources.reporter.queued {
                Some(ref mut data) => {
                    data.add(&latest_reading);
                }
                None => {
                    c.resources.reporter.queue(latest_reading);
                }
            }
        }
    }

    #[task(binds = USB, priority = 3, resources = [usb_device, reporter])]
    fn usb(c: usb::Context) {
        // let usb = unsafe { &(*hw::USB::ptr()) };
        // let status = usb.istr.read();
        // if status.susp().is_suspend() {}
        // cortex_m_semihosting::hprintln!("usb interrupt: {:#018b}", status.bits()).unwrap();
        //0b0000 1001 0000 0000
        //cortex_m_semihosting::hprintln!("usb");

        c.resources.usb_device.poll(&mut [c.resources.reporter]);
        //TODO: not clear from the docs whether I need to call poll myself here: https://docs.rs/usb-device/0.2.5/usb_device/device/index.html
        //if I do, then usb only works for a few heatmap frames --- maybe I get stuck in the interrupt handler due to the reporter's recursion?
        // use usb_device::class_prelude::*;
        // c.resources.reporter.poll();
    }
};
