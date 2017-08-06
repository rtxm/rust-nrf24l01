A pure Rust driver for NRF24L01 transceivers

The aim of this driver is to provide a rustic, easy to use, no non-sense
API to drive an NRF24L01 transceiver.This is not a port from a C or C++ library.
It has been written from scratch based on the
[specs](https://duckduckgo.com/l/?kh=-1&uddg=https%3A%2F%2Fwww.sparkfun.com%2Fdatasheets%2FComponents%2FSMD%2FnRF24L01Pluss_Preliminary_Product_Specification_v1_0.pdf).

For the moment, the driver only offer an API for the most reliable communication
scheme offered by NRF24L01 chips, that is _Enhanced Shockburst™_, with
automatic (hardware) packet acknowledment, optional payload, dynamic payload length and two byte CRC.

In the future, I'd like to provide :

    * asynchronous operation, with mio;
    * a stream based API for fast, "real time", data transmission
at the cost of possible packet loss.

I'm still quite new to Rust, so the code may be suboptimal. Feel free to submit pull requests to improve it !
