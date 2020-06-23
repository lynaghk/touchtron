import numpy as np
import matplotlib.pyplot as plt
import matplotlib.animation as animation
import time
import matplotlib
#matplotlib.use("Qt5Agg")
matplotlib.use("MACOSX")

#This runs about 19FPS fullscreen on my mac (50ish if you make the window very tiny)

N = 10
fig = plt.figure()
ax = fig.add_subplot()
plot = ax.imshow(np.random.rand(N,N), vmin=0, vmax=1, interpolation="nearest", cmap="viridis")

LastFrameTime = time.time()
def animate(i):
    global LastFrameTime
    start = time.time()

    data = np.random.rand(N,N)

    plot.set_data(data)
    time_taken = time.time() - start
    fps_calc = 1. / time_taken
    fps_actual = 1. / (start - LastFrameTime)
    print(fps_calc, fps_actual)
    LastFrameTime = start
        

ani = animation.FuncAnimation(fig, animate, fargs=(), interval=16)

plt.show()


