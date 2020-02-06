#![allow(dead_code)]
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

mod codec;
mod options;
mod server;
mod webhook;

use crate::server::*;
use futures::join;
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
        let mqtt_ws = async {
            if let Some(laddr) = &options.listener.mqtt_ws_laddr {
                if let Err(err) = serve_ws(&laddr).await {
                    println!("error: mqtt.ws listen on {}, {}", &laddr, err);
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
        let _ = join!(mqtt, wapi, mqtt_ws, coap);
    };
    run.await;
    Ok(())
}
