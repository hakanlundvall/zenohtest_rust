//
// Copyright (c) 2022 ZettaScale Technology
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ZettaScale Zenoh Team, <zenoh@zettascale.tech>
//
use async_std::task::sleep;
use clap::{App, Arg};
use protobuf::well_known_types::timestamp::Timestamp;
use protobuf::{Message, MessageField};
use std::time::{Duration, Instant};
use zenoh::config::Config;
use zenoh::prelude::r#async::*;

include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));
use test::Data;

fn to_secs_and_nanos(d: Duration) -> (i64, i32) {
    let seconds = d.as_secs();
    let part = Duration::from_secs(seconds);
    let nanos = (d - part).as_nanos() as i32;

    (seconds as i64, nanos)
}

#[async_std::main]
async fn main() {
    // Initiate logging
    env_logger::init();

    let config = parse_args();

    println!("Opening session...");
    let session = zenoh::open(config).res().await.unwrap();

    let mut publishers = Vec::new();

    for idx in 0..5000 {
        let key_expr = format!("prefix/db/DBI_{}", idx);
        let publisher = session.declare_publisher(key_expr).res().await.unwrap();
        publishers.push(publisher);
    }

    println!("Start publishing");
    let mut ts = Instant::now() + Duration::from_millis(80);
    loop {
        let now = Instant::now();
        if now < ts {
            sleep(ts - now).await;
        }

        let ts_all = Instant::now();
        for (idx, publisher) in publishers.iter().enumerate().take(375) {
            let jitter = Instant::now() - ts;
            let mut msg = Data::new();
            let mut seconds: i64 = 0;
            let mut nanos: i32 = 0;
            if let Ok(sendtime) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            {
                (seconds, nanos) = to_secs_and_nanos(sendtime);
            }

            msg.id = idx as i32;
            let mut ts = Timestamp::new();
            ts.seconds = seconds as i64;
            ts.nanos = nanos;
            msg.ts = MessageField::some(ts);
            msg.jitter = jitter.as_micros() as i64;

            let buf = msg.write_to_bytes();
            match buf {
                Ok(buf) => {
                    let ts1 = Instant::now();
                    publisher.put(buf).res().await.unwrap();
                    let elapsed = ts1.elapsed();
                    if elapsed > Duration::from_millis(10) {
                        println!("Large single blocking time: {} us", elapsed.as_micros());
                    }
                }
                Err(e) => {
                    println!("Encode Error: {}", e);
                }
            }
        }
        let elapsed = ts_all.elapsed();
        if elapsed > Duration::from_millis(20) {
            println!("Large total blocking time: {} us", elapsed.as_micros());
        }

        ts += Duration::from_millis(80);
    }
}

fn parse_args() -> Config {
    let args = App::new("zenoh pub example")
        .arg(
            Arg::from_usage("-m, --mode=[MODE] 'The zenoh session mode (peer by default).")
                .possible_values(["peer", "client"]),
        )
        .arg(Arg::from_usage(
            "-e, --connect=[ENDPOINT]...  'Endpoints to connect to.'",
        ))
        .arg(Arg::from_usage(
            "-l, --listen=[ENDPOINT]...   'Endpoints to listen on.'",
        ))
        .arg(Arg::from_usage(
            "-c, --config=[FILE]      'A configuration file.'",
        ))
        .arg(Arg::from_usage(
            "--no-multicast-scouting 'Disable the multicast-based scouting mechanism.'",
        ))
        .get_matches();

    let mut config = if let Some(conf_file) = args.value_of("config") {
        Config::from_file(conf_file).unwrap()
    } else {
        Config::default()
    };
    if let Some(Ok(mode)) = args.value_of("mode").map(|mode| mode.parse()) {
        config.set_mode(Some(mode)).unwrap();
    }
    if let Some(values) = args.values_of("connect") {
        config
            .connect
            .endpoints
            .extend(values.map(|v| v.parse().unwrap()))
    }
    if let Some(values) = args.values_of("listen") {
        config
            .listen
            .endpoints
            .extend(values.map(|v| v.parse().unwrap()))
    }
    if args.is_present("no-multicast-scouting") {
        config.scouting.multicast.set_enabled(Some(false)).unwrap();
    }

    config
}
