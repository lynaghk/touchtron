use crate::hw;

pub struct Adc {
    pub rb: hw::ADC,
}

//Pin=> ADC channel
// gpioa::PA0<Analog> => 0_u8,
// gpioa::PA1<Analog> => 1_u8,
// gpioa::PA2<Analog> => 2_u8,
// gpioa::PA3<Analog> => 3_u8,
// gpioa::PA4<Analog> => 4_u8,
// gpioa::PA5<Analog> => 5_u8,
// gpioa::PA6<Analog> => 6_u8,
// gpioa::PA7<Analog> => 7_u8,
// gpiob::PB0<Analog> => 8_u8,
// gpiob::PB1<Analog> => 9_u8,

//Mostly copy/pasted from f0 hal.
impl Adc {
    pub fn new(adc: hw::ADC, rcc: &mut hw::RCC) -> Self {
        let mut s = Self { rb: adc };
        s.select_clock(rcc);
        s.calibrate();

        s.rb.smpr.modify(|_, w| w.smp().cycles239_5());

        s.rb.cfgr1.modify(|_, w| {
            w.res().twelve_bit();
            w.align().right();
            //w.cont().continuous();
            w
        });

        //enable interrupt on end of conversion
        s.rb.ier.modify(|_, w| w.eocie().set_bit());

        //use all 10 adc channels. TODO: this should really come from the types of the input pins.
        //s.rb.chselr.write(|w| unsafe { w.bits(0b11_1111_1111) });

        s.power_up();

        s
    }

    pub fn start(&mut self, channel: u8) {
        self.rb
            .chselr
            .write(|w| unsafe { w.bits(1_u32 << channel) });
        self.rb.cr.modify(|_, w| w.adstart().start_conversion());
    }

    fn calibrate(&mut self) {
        /* Ensure that ADEN = 0 */
        if self.rb.cr.read().aden().is_enabled() {
            /* Clear ADEN by setting ADDIS */
            self.rb.cr.modify(|_, w| w.addis().disable());
        }
        while self.rb.cr.read().aden().is_enabled() {}

        /* Clear DMAEN */
        self.rb.cfgr1.modify(|_, w| w.dmaen().disabled());

        /* Start calibration by setting ADCAL */
        self.rb.cr.modify(|_, w| w.adcal().start_calibration());

        /* Wait until calibration is finished and ADCAL = 0 */
        while self.rb.cr.read().adcal().is_calibrating() {}
    }

    fn select_clock(&mut self, rcc: &mut hw::RCC) {
        rcc.apb2enr.modify(|_, w| w.adcen().enabled());
        rcc.cr2.modify(|_, w| w.hsi14on().on());
        while rcc.cr2.read().hsi14rdy().is_not_ready() {}
    }

    fn power_up(&mut self) {
        if self.rb.isr.read().adrdy().is_ready() {
            self.rb.isr.modify(|_, w| w.adrdy().clear());
        }
        self.rb.cr.modify(|_, w| w.aden().enabled());
        while self.rb.isr.read().adrdy().is_not_ready() {}
    }

    fn power_down(&mut self) {
        self.rb.cr.modify(|_, w| w.adstp().stop_conversion());
        while self.rb.cr.read().adstp().is_stopping() {}
        self.rb.cr.modify(|_, w| w.addis().disable());
        while self.rb.cr.read().aden().is_enabled() {}
    }
}
