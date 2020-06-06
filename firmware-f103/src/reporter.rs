use core::marker::PhantomData;
use usb_device::class::UsbClass;
use usb_device::class_prelude::*;

pub struct Reporter<'a, B, D>
where
    B: UsbBus,
    D: AsRef<[u8]>,
{
    pub data: D,
    interface: InterfaceNumber,
    write_ep: EndpointIn<'a, B>,
    _marker: PhantomData<B>,
}

const INTERVAL: u8 = 1; //Frame count.

impl<B, D> Reporter<'_, B, D>
where
    B: UsbBus,
    D: AsRef<[u8]>,
{
    pub fn new(alloc: &UsbBusAllocator<B>, max_packet_size: u16, data: D) -> Reporter<'_, B, D> {
        Reporter {
            data,
            interface: alloc.interface(),
            write_ep: alloc.interrupt(max_packet_size, INTERVAL),
            _marker: PhantomData,
        }
    }
}

impl<B, D> UsbClass<B> for Reporter<'_, B, D>
where
    B: UsbBus,
    D: AsRef<[u8]>,
{
    fn get_configuration_descriptors(&self, writer: &mut DescriptorWriter) -> Result<(), UsbError> {
        let vendor_class = 0xff;
        let no_detail = 0;
        writer.interface(self.interface, vendor_class, no_detail, no_detail)?;
        writer.endpoint(&self.write_ep)?;

        Ok(())
    }

    fn poll(&mut self) {
        self.write_ep.write(self.data.as_ref()).ok();
    }
}
