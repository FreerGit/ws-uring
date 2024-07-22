## Implementation details
### DNS
This crate uses raw file descriptors and io_uring, it's therefore platform dependant. Using the non-blocking version never blocks *except* on DNS lookup, haven't figured that one out yet. You can therefore assume ~1ms of blocking on connect, the call itself still requires you to handle non-blocking behaviour since the connecting itself is non-blocking. 

Handle lookup individually? Let the user decide? What do.
