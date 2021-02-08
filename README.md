# Touchtron

Misc. experiments with Rust + USB, Kevin making a touchpad.

See [background + video demo](https://kevinlynagh.com/touchpad/).
The [firmware-f0](firmware-f0/) is the fully-baked code running on the [custom PCB](pcb/).

This was a personal research project that I'm sharing in case it's helpful for others playing with touchpads, embedded Rust, etc.

## MacOS Setup 

```
#for stm32f0 (kevin custom board)
rustup target add thumbv6m-none-eabi 

#for stm32f1 (blue pill board)
rustup target add thumbv7m-none-eabi

pip install numpy matplotlib scipy opencv-python

pip install pyusb
brew install libusb 
```
