import numpy as np
import matplotlib.pyplot as plt
import matplotlib.animation as animation
import time

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
                 vmax=1000,
                 interpolation="nearest",
                 cmap="viridis")


frame_num = 0
frames = np.zeros((frames_to_read, int(bytes_to_read/2)), dtype = np.uint16)

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

        pwm_period = data[0]
        #print(pwm_period)
        # the [::-1] reverses data so we display in same orientation as physical touchpad
        matrix = data[1:][::-1].reshape(m,n)
        print(np.amin(matrix), np.amax(matrix))
        plot.set_data(matrix)
        time_taken = time.time() - start
        #print(1. / time_taken)
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
