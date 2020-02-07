use config::{Config, ConfigError, Environment, File};

#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
pub struct Limiter {
    pub max_conns: u32,                   //最大连接数
    pub max_conn_pre_second: Option<u32>, //每秒连接速率
}
#[derive(Debug, Clone, Deserialize)]
pub struct Authentication {
    pub method: String, //anonymous｜webhook
    pub file_path: Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Webhook {
    pub on_connect: Option<String>,
    pub on_disconnect: Option<String>,
    pub on_subscribe: Option<String>,
    pub on_unsubscribe: Option<String>,
    pub on_publish: Option<String>,
    pub on_delivered: Option<String>,
    pub on_deliver_timeout: Option<String>,
    pub on_kick: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
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
        s.set_default("listener.webapi_laddr", "0.0.0.0:5555")?;
        s.set_default("limiter.max_conns", 1024)?;
        s.set_default("limiter.max_conn_pre_second", 200)?;
        s.set_default("authentication.method", "anonymous")?;
        s.merge(Environment::with_prefix("rmqx")).unwrap();
        s.merge(File::with_name("/etc/rmqx/rmqx").required(false))?;
        s.try_into()
    }
}
