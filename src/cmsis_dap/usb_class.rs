// USB Device support
use usb_device::class_prelude::*;
use usb_device::device::DEFAULT_ALTERNATE_SETTING;

pub struct CmsisDap<'a, B: UsbBus, const MAX_PACKET_SIZE: usize> {
    interface: InterfaceNumber,
    serial_string: StringIndex,
    out_ep: EndpointOut<'a, B>,
    in_ep: EndpointIn<'a, B>,
}

impl<B: UsbBus, const MAX_PACKET_SIZE: usize> CmsisDap<'_, B, MAX_PACKET_SIZE> {
    pub fn new(allocator: &UsbBusAllocator<B>) -> CmsisDap<'_, B, MAX_PACKET_SIZE> {
        CmsisDap {
            interface: allocator.interface(),
            serial_string: allocator.string(),
            out_ep: allocator.bulk(64),
            in_ep: allocator.bulk(64),
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, UsbError> {
        self.out_ep.read(buf)
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize, UsbError> {
        self.in_ep.write(buf)
    }
}

const USB_IF_CLASS_VENDOR: u8 = 0xff;
const USB_IF_SUBCLASS_VENDOR: u8 = 0x00;
const USB_IF_PROTOCOL_NONE: u8 = 0x00;

impl<B: UsbBus, const MAX_PACKET_SIZE: usize> usb_device::class::UsbClass<B>
    for CmsisDap<'_, B, MAX_PACKET_SIZE>
{
    fn get_configuration_descriptors(
        &self,
        writer: &mut usb_device::descriptor::DescriptorWriter,
    ) -> usb_device::Result<()> {
        writer.interface_alt(
            self.interface,
            DEFAULT_ALTERNATE_SETTING,
            USB_IF_CLASS_VENDOR,
            USB_IF_SUBCLASS_VENDOR,
            USB_IF_PROTOCOL_NONE,
            Some(self.serial_string),
        )?;
        writer.endpoint(&self.out_ep)?;
        writer.endpoint(&self.in_ep)?;
        Ok(())
    }

    fn get_string(&self, index: StringIndex, lang_id: u16) -> Option<&str> {
        let _ = lang_id;
        if index == self.serial_string {
            Some("CMSIS-DAP interface")
        } else {
            None
        }
    }
}
