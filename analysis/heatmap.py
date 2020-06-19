import numpy as np
import matplotlib.pyplot as plt
import matplotlib.animation as animation
import time

n = 15
m = 10
bytes_to_read = (1 + n * m) * 2 #u16 vals

frames_to_read = 100

####################################

import usb.core
d = usb.core.find(idVendor=0x16c0, idProduct=0x27dd)
d.set_configuration()

address = 0x81 #this is always the address of first endpoint.

def read_touchpad():
    buffer = d.read(address, bytes_to_read)
    matrix = np.frombuffer(buffer, np.dtype(np.uint16)).reshape(m,n)
    return matrix


#########################

fig = plt.figure()
ax = fig.add_subplot()
plot = ax.imshow(np.random.rand(m,n),
                 vmin=0,
                 vmax=4095, #ADC is actually 12bit only
                 interpolation="nearest",
                 cmap="viridis")


frame_num = 0
frames = np.zeros(frames_to_read * bytes_to_read/2, np.dtype(np.uint16)).reshape(frames_to_read, -1)

def animate(i):
    global frame_num
    start = time.time()
    data = read_touchpad();
    frames[frame_num] = data;
    frame_num += 1

    if frame_num == frames_to_read:
        with open("frames.npy", "wb") as f:
            np.save(f, frames)
        exit(0)

    print(data[0]) #PWM period in ticks
    plot.set_data(data[1, :])
    time_taken = time.time() - start
    #print(1. / time_taken)

ani = animation.FuncAnimation(fig, animate, fargs=(), interval=16)
plt.show()
