import numpy as np
import matplotlib.pyplot as plt
import matplotlib.animation as animation
import time
import matplotlib
#matplotlib.use("Qt5Agg")
matplotlib.use("MACOSX")

# import pyqtgraph.examples
# pyqtgraph.examples.run()

n = 15
m = 10
bytes_to_read = (1 + n * m) * 2 #u16 vals

frames_to_read = 1000

####################################

import usb.core
d = usb.core.find(idVendor=0x16c0, idProduct=0x27dd)
d.set_configuration()

address = 0x81 #this is always the address of first endpoint.

def read_touchpad():
    buffer = d.read(address, bytes_to_read)
    data = np.frombuffer(buffer, dtype = np.dtype(np.uint16))
    return data


#########################

fig = plt.figure()
ax = fig.add_subplot()
plot = ax.imshow(np.random.rand(m,n),
                 vmin=0,
                 #vmax=4095, #ADC is actually 12bit only
                 vmax=100,
                 interpolation="nearest",
                 cmap="viridis")


frame_num = 0
frames = np.zeros((frames_to_read, int(bytes_to_read/2)), dtype = np.uint16)

from scipy import signal
from scipy.ndimage import gaussian_filter

def process1(m):
    return gaussian_filter(np.maximum(m - 60, 0), 0.01)

def process1(m):
    return signal.convolve2d(np.maximum(m - 60, 0), np.ones((2,2)) / 4)

    
def animate(i):
    global frame_num
    start = time.time()
    try:
        data = read_touchpad();

        frames[frame_num] = data;
        frame_num += 1

        if frame_num == frames_to_read:
            with open("frames.npy", "wb") as f:
                np.save(f, frames)
            print("Read %s frames, quitting" % frames_to_read)
            exit(0)

        num_samples = data[0]
        # the [::-1] reverses data so we display in same orientation as physical touchpad
        matrix = data[1:][::-1].reshape(m,n) / num_samples
        #print(num_samples, np.amax(matrix))
        plot.set_data(process1(matrix))
        time_taken = time.time() - start
        print(1. / time_taken)
    except Exception as e:
        #yolo
        print(e)
        return None

ani = animation.FuncAnimation(fig, animate, fargs=(), interval=16)

plt.show()


# start = time.time()
# for idx in range(150):
#     read_touchpad();
# print(time.time() - start)
