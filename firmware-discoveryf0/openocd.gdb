target extended-remote :3333
set print asm-demangle on

# set backtrace limit to not have infinite backtrace loops
set backtrace limit 32
monitor arm semihosting enable

# detect unhandled exceptions, hard faults and panics
break DefaultHandler
break HardFault

# set history save on
# set confirm off
# monitor reset halt

load
continue