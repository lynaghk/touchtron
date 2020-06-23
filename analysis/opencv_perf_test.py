import cv2
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.animation as animation
import time

N = 10

# pip install opencv-python
# runs at 60 fps w/ 14 ms waitkey. setting waitkey 1 runs at 200 fps

LastFrameTime = time.time()

def animate():
    global LastFrameTime
    start = time.time()

    m = np.random.randint(0, 255, size=(N,N), dtype=np.uint8)
    m = cv2.resize(m, (400,400), interpolation=cv2.INTER_NEAREST)
    m = cv2.applyColorMap(m, cv2.COLORMAP_JET)
    cv2.imshow("foo", m)
    time_taken = time.time() - start
    fps_calc = 1. / time_taken
    fps_actual = 1. / (start - LastFrameTime)
    print(fps_calc, fps_actual)
    LastFrameTime = start


while(True):

    try:
        animate()
    except Exception as e:
        # swallow exception so that runner doesn't crash
        print(e)
    if cv2.waitKey(1) & 0xFF == ord('q'):
        break

cv2.destroyAllWindows()
