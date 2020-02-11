#![allow(dead_code)]
#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;
// #[cfg(unix)]
// #[cfg(feature = "alloc-jemalloc")]
// extern crate jemallocator;
extern crate config;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
// #[macro_use]
// extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate futures;

// #[cfg(unix)]
// #[cfg(feature = "alloc-jemalloc")]
// #[global_allocator]
// static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

mod codec;
mod limiter;
mod options;
mod server;
mod webhook;

use crate::server::*;
use options::Options;
use std::*;

#[tokio::main]
async fn main() -> Result<()> {
    let options = Options::new().unwrap();
    let run = async move {
        let coap = async {
            if let Some(laddr) = &options.listener.coap_laddr {
                if let Err(err) = serve_coap(&laddr).await {
                    println!("error: coap listen on {}, {}", &laddr, err);
                }
            }
        };
        let mqtt = async {
            if let Err(err) = serve(&options.listener.mqtt_laddr).await {
                println!(
                    "error: mqtt listen on {}, {}",
                    &options.listener.mqtt_laddr, err
                );
            }
        };
        let mqtt_tls = async {
            if let Some(laddr) = &options.listener.mqtt_tls_laddr {
                if let Some(p12) = &options.listener.p12_file {
                    if let Some(passwd) = &options.listener.p12_passwd {
                        if let Err(err) = serve_tls(&laddr, &p12, &passwd).await {
                            println!("error: mqtt.tls listen on {}, {}", &laddr, err);
                        }
                    }
                }
            }
        };
        let mqtt_ws = async {
            if let Some(laddr) = &options.listener.mqtt_ws_laddr {
                if let Err(err) = serve_ws(&laddr).await {
                    println!("error: mqtt.ws listen on {}, {}", &laddr, err);
                }
            }
        };
        let mqtt_wss = async {
            if let Some(laddr) = &options.listener.mqtt_wss_laddr {
                if let Some(p12) = &options.listener.p12_file {
                    if let Some(passwd) = &options.listener.p12_passwd {
                        if let Err(err) = serve_wss(&laddr, &p12, &passwd).await {
                            println!("error: mqtt.wss listen on {}, {}", &laddr, err);
                        }
                    }
                }
            }
        };
        let wapi = async {
            if let Err(err) = serve_webapi(&options.listener.webapi_laddr).await {
                println!(
                    "error: webapi listen on {}, {}",
                    &options.listener.webapi_laddr, err
                );
            }
        };
        // let ctrlc = async {
        //     signal::ctrl_c().await.unwrap();
        //     println!("ctrl-c received!");
        // };
        let _ = join!(mqtt, mqtt_tls, wapi, mqtt_ws, mqtt_wss, coap);
    };
    // signal::ctrl_c().await?;
    run.await;
    Ok(())
}
