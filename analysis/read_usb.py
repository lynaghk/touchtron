import usb.core
#https://github.com/pyusb/pyusb/blob/master/docs/tutorial.rst

#touchtron = usb.core.find(idVendor=0x16c0, idProduct=0x27dd)
#touchtron.set_configuration()


# devices = usb.core.find(find_all=True)
# for d in devices:
#     print(d)

address = 0x81
d = usb.core.find(idVendor=0x16c0, idProduct=0x27dd)

for _ in range(1, 10):
    ret = d.read(address, 16, 100)
    print(ret)

