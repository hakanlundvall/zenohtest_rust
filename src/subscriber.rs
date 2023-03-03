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
use futures::prelude::*;
use futures::select;
use protobuf::Message;
use std::time::{Duration, Instant};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use zenoh::config::Config;
use zenoh::prelude::r#async::*;
use std::io::{Write, stdout};
include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));
use test::Data;
use log::{info, error};

#[async_std::main]
async fn main() {
    // Initiate logging
    env_logger::init();

    let (config, t) = parse_args();

    let key_expr = String::from("prefix/**");

    println!("Opening session... running for {} s", t);
    let session = zenoh::open(config).res().await.unwrap();

    info!("Declaring Subscriber on '{}'...", &key_expr);

    let subscriber = session.declare_subscriber(&key_expr).res().await.unwrap();

    println!("Enter 'q' to quit...");
    let mut stdin = async_std::io::stdin();
    let mut input = [0_u8];
    let mut stdout_locked = stdout().lock();
    let end_time = Instant::now() + Duration::from_secs(t);
    loop {
        select!(
            _ = sleep(end_time-Instant::now()).fuse() =>
            {
                break;
            }

            sample = subscriber.recv_async() => {
                let now = std::time::SystemTime::now();
                let dt : OffsetDateTime = now.into();
                let recvtime = now.duration_since(std::time::UNIX_EPOCH).unwrap();
                let sample = sample.unwrap();
                match Data::parse_from_bytes(sample.payload.contiguous().as_ref()) {
                    Ok(msg) => {
                        let sendtime = Duration::from_secs(msg.ts.seconds as u64) + Duration::from_nanos(msg.ts.nanos as u64);
                        writeln!(stdout_locked, "{}:  {}, ts = {}.{}, send jitter = {} us, delay = {} us",
                            dt.format(&Rfc3339).unwrap(),
                            msg.id,
                            msg.ts.seconds,
                            msg.ts.nanos / 1000000,
                            msg.jitter,
                            (recvtime-sendtime).as_micros()).unwrap();
                    },
                    Err(e) => {
                        error!("Parse Error: {}", e);
                    }
                }
            },

            _ = stdin.read_exact(&mut input).fuse() => {
                match input[0] {
                    b'q' => break,
                    0 => sleep(Duration::from_secs(1)).await,
                    _ => (),
                }
            },

        );
    }
}

fn parse_args() -> (Config, u64) {
    let args = App::new("zenoh sub example")
        .arg(
            Arg::from_usage("-t, --time=[TIME]  'stop after TIME seconds."),
        )
        .arg(
            Arg::from_usage("-m, --mode=[MODE]  'The zenoh session mode (peer by default).")
                .possible_values(["peer", "client"]),
        )
        .arg(Arg::from_usage(
            "-e, --connect=[ENDPOINT]...   'Endpoints to connect to.'",
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

    let mut t : u64 = 1000000;
    if let Some(v) = args.value_of("time") {
        t = if let Ok(v) = v.parse() { v } else { t };
    }

    (config, t)
}
