# Phase 12B-D US30 qualification replay

This nonvisual open-prices replay enables both the five-file structural oracle
and the seven canonical auction research ledgers. It is the first bounded
real-history development fixture for independent Rust 12B-D reconstruction.

The deterministic finalization cutoff is the final tradable US30 M5 bar at
22:55. A midnight cutoff is invalid for this session because the tester shuts
its agent down before the indicator can seal an END receipt.

It is not an every-tick production certificate. Once the implementation agrees
on this development window, a separate every-tick qualification and untouched
verification window remain mandatory before B-D are closed.
