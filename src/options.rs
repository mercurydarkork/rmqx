use config::{Config, ConfigError, Environment, File};

#[derive(Debug, Deserialize)]
pub struct Listener {
    pub mqtt_laddr: String,
    pub mqtt_tls_laddr: Option<String>,
    pub mqtt_ws_laddr: Option<String>,
    pub mqtt_wss_laddr: Option<String>,
    pub coap_laddr: Option<String>,
    pub webapi_laddr: String,
    pub p12_file: Option<String>,
    pub p12_passwd: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct Limiter {
    pub max_conns: u32,           //最大连接数
    pub max_conn_pre_second: u32, //每秒连接速率
}
#[derive(Debug, Deserialize)]
pub struct Authentication {
    pub method: String, //anonymous｜host｜file｜webhook
    pub file_path: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct Webhook {
    pub on_connect: String,
    pub on_disconnect: String,
    pub on_subscribe: String,
    pub on_unsubscribe: String,
    pub on_publish: String,
    pub on_publish_ack: String,
    pub on_publish_release: String,
    pub on_publish_complete: String,
}

#[derive(Debug, Deserialize)]
pub struct Options {
    pub listener: Listener,
    pub limiter: Limiter,
    pub authentication: Authentication,
    pub webhook: Option<Webhook>,
}

impl Options {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();
        s.set_default("listener.mqtt_laddr", "0.0.0.0:1883")?;
        s.set_default("listener.mqtt_tls_laddr", "0.0.0.0:8883")?;
        s.set_default("listener.mqtt_ws_laddr", "0.0.0.0:80")?;
        s.set_default("listener.mqtt_wss_laddr", "0.0.0.0:443")?;
        s.set_default("listener.coap_laddr", "0.0.0.0:5683")?;
        s.set_default("listener.webapi_laddr", "0.0.0.0:5555")?;
        s.set_default("limiter.max_conns", 1024)?;
        s.set_default("limiter.max_conn_pre_second", 200)?;
        s.set_default("authentication.method", "anonymous")?;
        s.merge(Environment::with_prefix("rmqx")).unwrap();
        s.merge(File::with_name("/etc/rmqx/rmqx").required(false))?;
        s.try_into()
    }
}
