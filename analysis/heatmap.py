import numpy as np
import matplotlib.pyplot as plt
import matplotlib.animation as animation
import time

n = 10
m = 15

####################################

import usb.core
d = usb.core.find(idVendor=0x16c0, idProduct=0x27dd)
d.set_configuration()

address = 0x81 #this is always the address of first endpoint.
bytes_to_read = n * m * 2 #u16 vals
def read_touchpad():
    buffer = d.read(address, bytes_to_read)
    matrix = np.frombuffer(buffer, np.dtype(np.uint16)).reshape(n,m)
    return matrix


#########################

fig = plt.figure()
ax = fig.add_subplot()
plot = ax.imshow(np.random.rand(n, m),
                 vmin=0,
                 vmax=4095, #ADC is actually 12bit only
                 interpolation="nearest",
                 cmap="viridis")




def animate(i):
    start = time.time()
    plot.set_data(read_touchpad())
    time_taken = time.time() - start
    print(1. / time_taken)

ani = animation.FuncAnimation(fig, animate, fargs=(), interval=16)
plt.show()
