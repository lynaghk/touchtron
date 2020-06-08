import usb.core
#https://github.com/pyusb/pyusb/blob/master/docs/tutorial.rst

# devices = usb.core.find(find_all=True)
# for d in devices:
#     print(d)
#exit


d = usb.core.find(idVendor=0x16c0, idProduct=0x27dd)
d.set_configuration()

address = 0x81 #this is always the address of first endpoint.

n = 12
m = 8
bytes_to_read = n*m*2 #u16 vals

for idx in range(1, 10):
    ret = d.read(address, bytes_to_read)
    print(ret)
