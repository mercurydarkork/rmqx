#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate serde_json;

mod codec;
mod peer;
mod router;
mod server;

use futures::join;
use server::*;
use std::*;
use tokio::runtime::Builder;

// #[tokio::main]
// async fn main() -> Result<()> {
//     let laddr = env::args()
//         .nth(1)
//         .unwrap_or_else(|| "0.0.0.0:1883".to_string());
//     let laddr2 = "0.0.0.0:8883".to_owned();
//     let laddr3 = "0.0.0.0:80".to_owned();
//     let laddr4 = "0.0.0.0:443".to_owned();
//     let laddr5 = "0.0.0.0:5555".to_owned();
//     let laddr6 = "0.0.0.0:5683".to_owned();
//     let run = async move {
//         let coap = async {
//             if let Err(err) = serve_coap(&laddr6).await {
//                 println!("listening coap.udp on {}, {}", laddr6, err);
//             }
//         };
//         let task = async {
//             if let Err(err) = serve(&laddr).await {
//                 println!("listening mqtt.tcp on {}, {}", laddr, err);
//             }
//         };
//         let task2 = async {
//             if let Err(err) = serve_tls(&laddr2, "server.p12", "111111").await {
//                 println!("listening mqtt.tls on {}, {}", laddr2, err);
//             }
//         };
//         let ws = async {
//             if let Err(err) = serve_ws(&laddr3).await {
//                 println!("listening mqtt.ws on {}, {}", laddr3, err);
//             }
//         };
//         let wss = async {
//             if let Err(err) = serve_wss(&laddr4).await {
//                 println!("listening mqtt.wss on {}, {}", laddr4, err);
//             }
//         };
//         let wapi = async {
//             if let Err(err) = serve_webapi(&laddr5).await {
//                 println!("listening webapi on {}, {}", laddr5, err);
//             }
//         };
//         let _ = join!(coap, task, task2, ws, wss, wapi);
//     };
//     run.await;
//     Ok(())
// }

fn main() {
    // let laddr = env::args()
    //     .nth(1)
    //     .unwrap_or_else(|| "0.0.0.0:1883".to_string());
    let laddr = "0.0.0.0:6315".to_owned();
    let laddr2 = "0.0.0.0:8883".to_owned();
    let laddr3 = "0.0.0.0:80".to_owned();
    let laddr4 = "0.0.0.0:443".to_owned();
    let laddr5 = "0.0.0.0:5555".to_owned();
    let laddr6 = "0.0.0.0:5683".to_owned();
    // std::thread::spawn(move || {
    //     Builder::new()
    //         .basic_scheduler()
    //         .build()
    //         .unwrap()
    //         .block_on(async move {
    //             println!("basic_scheduler");
    //         });
    // });
    // Builder::new()
    //     .threaded_scheduler()
    //     .enable_all()
    //     .build()
    //     .unwrap()
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move {
            // let coap = async {
            //     if let Err(err) = serve_coap(&laddr6).await {
            //         println!("listening coap.udp on {}, {}", laddr6, err);
            //     }
            // };
            let task = async {
                if let Err(err) = serve(&laddr).await {
                    println!("listening mqtt.tcp on {}, {}", laddr, err);
                }
            };
            // let task2 = async {
            //     if let Err(err) = serve_tls(&laddr2, "server.p12", "111111").await {
            //         println!("listening mqtt.tls on {}, {}", laddr2, err);
            //     }
            // };
            // let ws = async {
            //     if let Err(err) = serve_ws(&laddr3).await {
            //         println!("listening mqtt.ws on {}, {}", laddr3, err);
            //     }
            // };
            // let wss = async {
            //     if let Err(err) = serve_wss(&laddr4).await {
            //         println!("listening mqtt.wss on {}, {}", laddr4, err);
            //     }
            // };
            let wapi = async {
                if let Err(err) = serve_webapi(&laddr5).await {
                    println!("listening webapi on {}, {}", laddr5, err);
                }
            };
            let _ret = join!(task, wapi);
        });
}
