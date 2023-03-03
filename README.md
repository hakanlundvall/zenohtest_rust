# Test application for Zenoh

This is a small test application consisting of a publisher executable and a subscriber executable:
* `zenohtest` - creates  5000 publishers and then publish on 375 of them once every 80 ms 
* `testsubscriber` - subscribes to all then topics using wildcard  key expression

The payload of each put contains a timestamp of when it was sent and information about how long after it should have ideally have been sent (jitter). The subscriber logs the delay from send to receive, provided the sender's and receiver's clocks are synchronized.

The sender logs if any `put` operation takes unusually long time or if the combined time for publishing all topics in one batch takes unusually long time.

## Performing tests
start publisher:
``` 
$ ./target/release/zenohtest
``` 

When `zenohtest` reports that it has started publishing, start first subscriber:
``` 
$ ./target/release/testsubscriber > /tmp/subscriber1.log
``` 

Optionally start a second subscriber running for 2 seconds:
``` 
$ ./target/release/testsubscriber -t 2> /tmp/subscriber2.log
``` 

## Results
When running this test on somewhat CPU limited hardware I experience long delays on `put` when a new subscriber is started.

The delay can also be noticed in the logged delay for the first subscriber and it takes a few publishing cycles before the the sender is in sync with publications again. And event longer before the delay experienced at th subscriber is back to normal.

Tested CPUs:
* AMD  GX-210JC  (Dual core x86, 1 GHz)
* iMX8M Quad (Arm IMX8 quad core 1.3 GHz)

## Build prerequisites
Aside from Cargo there need to be a protobuf code generator (`protoc`) installed.

Build using: `cargo build --release`