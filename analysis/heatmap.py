import numpy as np
import matplotlib.pyplot as plt
import matplotlib.animation as animation
import time

nrow = 8
ncol = 12

fig = plt.figure()
ax = fig.add_subplot()
plot = ax.imshow(np.random.rand(nrow, ncol),
                 interpolation="nearest",
                 cmap="viridis")

def animate(i):
    # start = time.time()
    m = np.random.rand(nrow, ncol)
    plot.set_data(m)
    # time_taken = time.time() - start
    # print(1. / time_taken)

ani = animation.FuncAnimation(fig, animate, fargs=(), interval=16)
plt.show()
