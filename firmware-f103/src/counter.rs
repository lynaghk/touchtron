use core::marker::PhantomData;
use usb_device::class::UsbClass;
use usb_device::class_prelude::*;
//use usbd_serial
pub struct Counter<'a, B>
where
    B: UsbBus,
{
    idx: i64,
    my_interface: InterfaceNumber,
    write_ep: EndpointIn<'a, B>,
    _marker: PhantomData<B>,
}

const INTERVAL: u8 = 1; //Frame count.

impl<B> Counter<'_, B>
where
    B: UsbBus,
{
    pub fn new(alloc: &UsbBusAllocator<B>, max_packet_size: u16) -> Counter<'_, B> {
        Counter {
            idx: 0,
            my_interface: alloc.interface(),
            write_ep: alloc.interrupt(max_packet_size, INTERVAL),
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
        writer.interface(self.my_interface, vendor_class, no_detail, no_detail)?;
        //         writer.write(
        //             CS_INTERFACE,
        //             &[
        // ...
        //             ])?;

        writer.endpoint(&self.write_ep)?;

        Ok(())
    }

    fn poll(&mut self) {
        self.idx += 1;
        self.write_ep.write(&self.idx.to_le_bytes()).ok();
    }
}
