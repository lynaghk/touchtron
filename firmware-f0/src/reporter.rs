use usb_device::class::UsbClass;
use usb_device::class_prelude::*;

const INTERVAL: u8 = 1; //Frame count.
const MAX_PACKET_SIZE: u16 = 64; // USB full speed max packet size is 64
type Offset = usize;
pub struct Reporter<'a, B, D>
where
    B: UsbBus,
    D: AsRef<[u8]>,
{
    transmitting: Option<(Offset, D)>,
    queued: Option<D>,
    interface: InterfaceNumber,
    write_ep: EndpointIn<'a, B>,
}

impl<B, D> Reporter<'_, B, D>
where
    B: UsbBus,
    D: AsRef<[u8]>,
{
    pub fn new(alloc: &UsbBusAllocator<B>) -> Reporter<'_, B, D> {
        Reporter {
            transmitting: None,
            queued: None,
            interface: alloc.interface(),
            write_ep: alloc.interrupt(MAX_PACKET_SIZE, INTERVAL),
        }
    }

    pub fn queue(&mut self, new_data: D) {
        self.queued = Some(new_data);
    }
}

impl<'a, B, D: 'a> UsbClass<B> for Reporter<'_, B, D>
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
    //TODO, safe to rely on this method getting called? https://github.com/mvirkkunen/usb-device/issues/32
    fn poll(&mut self) {
        match &mut self.transmitting {
            Some((offset, msg)) => {
                let n = msg.as_ref().len();
                let end = core::cmp::min(*offset + MAX_PACKET_SIZE as usize, n);
                let packet = &msg.as_ref()[*offset..end];
                self.write_ep.write(packet).ok();
                *offset += packet.len();
                if n == end {
                    self.transmitting = None;
                }
            }

            None => {
                if let Some(new_data) = self.queued.take() {
                    self.transmitting = Some((0, new_data));
                    self.poll(); //recur so first chunk of new data is sent
                }
            }
        }
    }
}
