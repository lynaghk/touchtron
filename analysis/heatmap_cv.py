import cv2
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.animation as animation
import time


####################################

N = 15
M = 10
bytes_to_read = (1 + N * M) * 2 #u16 vals

import usb.core
d = usb.core.find(idVendor=0x16c0, idProduct=0x27dd)
d.set_configuration()

address = 0x81 #this is always the address of first endpoint.

def read_touchpad():
    buffer = d.read(address, bytes_to_read)
    data = np.frombuffer(buffer, dtype = np.dtype(np.uint16))
    num_samples = data[0]
    return (data[1:][::-1].reshape(M,N) / num_samples).astype(np.uint8)

# def read_touchpad():    
#     return cv2.imread("/Users/dev/Desktop/blobs.png", cv2.IMREAD_GRAYSCALE)

Offset = 50

def process(m):
    m = np.maximum(m.astype(np.int16) - Offset, 0)
    m = cv2.pyrUp(m, m.size*8)
    m = cv2.resize(m, (600,400), interpolation=cv2.INTER_NEAREST)
    return m.astype(np.uint8)




params = cv2.SimpleBlobDetector_Params() 
# Change thresholds
params.minThreshold = 80
params.maxThreshold = 255

params.filterByColor = True
params.blobColor = 255

# Filter by Area.
params.filterByArea = True
params.minArea = 10

# Filter by Circularity
params.filterByCircularity = False
params.minCircularity = 0.1

# Filter by Convexity
params.filterByConvexity = False
params.minConvexity = 0.87

# Filter by Inertia
params.filterByInertia = False
params.minInertiaRatio = 0.01

Detector = cv2.SimpleBlobDetector_create(params)

def on_track(_):
    global Detector
    global Offset
    Offset = cv2.getTrackbarPos("Offset", "processed")
    params.minArea = cv2.getTrackbarPos("Min Area", "processed")
    params.minThreshold = cv2.getTrackbarPos("Min Threshold", "processed")
    Detector = cv2.SimpleBlobDetector_create(params)


def createWindow(name):
    cv2.namedWindow(name, cv2.WINDOW_NORMAL)
    cv2.resizeWindow(name, 600,400)


createWindow("raw")
createWindow("processed")
#createWindow("text")
cv2.createTrackbar("Offset", "processed", Offset, 255, on_track)
cv2.createTrackbar("Min Area", "processed", int(params.minArea), 1000, on_track)
cv2.createTrackbar("Min Threshold", "processed", int(params.minThreshold), 255, on_track)




def step():
    m = read_touchpad()
    #cv2.imshow("raw", cv2.applyColorMap(m, cv2.COLORMAP_VIRIDIS))
    cv2.imshow("raw", m)
    m = process(m)
    keypoints = Detector.detect(m)
    m = cv2.applyColorMap(m, cv2.COLORMAP_VIRIDIS)

    m = cv2.drawKeypoints(m, keypoints, np.array([]), (0,0,255), cv2.DRAW_MATCHES_FLAGS_NOT_DRAW_SINGLE_POINTS)
    #m = cv2.drawKeypoints(m, keypoints, np.array([]), (0,0,255), cv2.DRAW_MATCHES_FLAGS_DRAW_RICH_KEYPOINTS)
    cv2.imshow("processed", m)

    # Render debug info
    # text = np.zeros((300,600), dtype=np.uint8)
    # margin_left = 5
    # margin_top = 30
    # line_height = 30
    # color = (255,255,255)
    # for idx, k in enumerate(keypoints):
    #     cv2.putText(text, f"({int(k.pt[0])}, {int(k.pt[1])}) {int(k.size)}", (margin_left, margin_top + idx*line_height), cv2.FONT_HERSHEY_PLAIN, 1, color)

    # cv2.imshow("text", text)




while(True):
    try:
        step()
    except Exception as e:
        print(e)
    if cv2.waitKey(10) & 0xFF == ord('q'):
        break

#cv2.destroyAllWindows()
exit(0)
