
# Websocket client
The client is non-blocking and it's modelled as a state machine using io_uring to handle IO without sys-calls. 

### Example
A simple example is within [examples](/examples/main.rs), to run the example do:
```console
foo@bar:~$ cd test_server
foo@bar:~$ npm i
foo@bar:~$ node server.js
```
open a new terminal and run (you can skip --release if you want):
```console
foo@bar:~$ cargo run --release --example main
```

### DNS
Using the client  never blocks *except* on DNS lookup, when creating a new client object a dns lookup is made. You can therefore assume ~1ms of blocking on new(). 


### io_uring
This crate uses raw file descriptors and io_uring, it's therefore platform dependant and requires a >= 5.1 kernel. To check wether or not you system supports io_uring you can run:
```console
foo@bar:~$ grep -i uring /boot/config-$(uname -r)
```

