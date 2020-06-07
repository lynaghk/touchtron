use core::marker::PhantomData;
use usb_device::class::UsbClass;
use usb_device::class_prelude::*;

pub struct Counter<'a, B>
where
    B: UsbBus,
{
    idx: i64,
    interface: InterfaceNumber,
    write_ep: EndpointIn<'a, B>,
    _marker: PhantomData<B>,
}

const INTERVAL: u8 = 1; //Frame count.
const MAX_PACKET_SIZE: u16 = 64;

impl<B> Counter<'_, B>
where
    B: UsbBus,
{
    pub fn new(alloc: &UsbBusAllocator<B>) -> Counter<'_, B> {
        Counter {
            idx: 0,
            interface: alloc.interface(),
            write_ep: alloc.interrupt(MAX_PACKET_SIZE, INTERVAL),
            _marker: PhantomData,
        }
    }
}

impl<B> UsbClass<B> for Counter<'_, B>
where
    B: UsbBus,
{
    fn get_configuration_descriptors(&self, writer: &mut DescriptorWriter) -> Result<(), UsbError> {
        let vendor_class = 0xff;
        let no_detail = 0;
        writer.interface(self.interface, vendor_class, no_detail, no_detail)?;
        writer.endpoint(&self.write_ep)?;

        Ok(())
    }

    fn poll(&mut self) {
        self.idx += 1;
        self.write_ep.write(&self.idx.to_le_bytes()).ok();
    }
}
