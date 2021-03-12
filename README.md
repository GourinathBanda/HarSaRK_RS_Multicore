# HarSaRK-RS-multi

A safe and lightweight dual-core real-time Kernel written in Rust. Designed for NXP LPC55S69 Cortex M-33 dual core microcontroller.

## Features

* Due to the usage of boolean vectors, the kernel does not use and intensive data- structure like queue or list.
* Scheduling, Software bus, and resource management is implemented by boolean vectors, which reduce the memory and performance overhead of the kernel.
* Non-blocking Synchronisation and communication between tasks are achieved through boolean vector semaphores.
* Event manager with lightweight event handlers: This helps keep the execution time of interrupts very low.
* Resource management through Stack-based priority ceiling protocol: This guarantees not only mutually exclusive allocation of resources but also deadlock-free execution of tasks.
* Dual-core support

For examples, take a look at `/examples`.

[API Reference](https://docs.rs/harsark/0.3.5/harsark/)

## References

Gourinath Banda. “Scalable Real-Time Kernel for Small Embedded Systems”. English. MSEngg Dissertation. Denmark: University of Southern Denmark, June 2003. URL: http://citeseerx.ist.psu.edu/viewdoc/download;jsessionid=84D11348847CDC13691DFAED09883FCB?doi=10.1.1.118.1909&rep=rep1&type=pdf.

A. Burns and A. J. Wellings, "A Schedulability Compatible Multiprocessor Resource Sharing Protocol -- MrsP," 2013 25th Euromicro Conference on Real-Time Systems, Los Alamitos, CA, USA, 2013, pp. 282-291, doi: 10.1109/ECRTS.2013.37. URL: https://www-users.cs.york.ac.uk/~burns/MRSPpaper.pdf


## Future Work

Task migration mechanism in the kernel is designed for dual-core systems. We are in the process of making a generic multi-core implementation of the same.

## License

This project is under the MIT license.
