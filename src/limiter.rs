use crate::options;
use crate::server::Result;
use bytestring::ByteString;
use ratelimit_meter::{DirectRateLimiter, KeyedRateLimiter, GCRA};
use std::num::NonZeroU32;
use thiserror::Error;
use tokio::time::delay_for;
use tokio::time::Duration;
#[derive(Error, Debug)]
pub enum LimitError {
    #[error("too many requests")]
    TooManyRequests(Duration),
}

pub struct Limiter<'a> {
    global: Option<DirectRateLimiter<GCRA>>,
    client: Option<KeyedRateLimiter<&'a ByteString, GCRA>>,
}

impl<'a> Limiter<'a> {
    pub fn new(limit: options::Limiter) -> Self {
        let global = if let Some(v) = limit.accept_per_second {
            Some(DirectRateLimiter::<GCRA>::per_second(
                NonZeroU32::new(v).unwrap(),
            ))
        } else {
            None
        };
        let client = if let Some(v) = limit.client_per_second {
            Some(KeyedRateLimiter::<&ByteString, GCRA>::per_second(
                NonZeroU32::new(v).unwrap(),
            ))
        } else {
            None
        };
        Self {
            global: global,
            client: client,
        }
    }

    pub async fn check_connect(&mut self) -> Result<()> {
        if let Some(lim) = &mut self.global {
            loop {
                match lim.check() {
                    Ok(_) => {
                        return Ok(());
                    }
                    Err(_) => {
                        delay_for(Duration::from_millis(5)).await;
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn check_client(&mut self, client_id: &'a ByteString) -> Result<()> {
        if let Some(lim) = &mut self.client {
            loop {
                match lim.check(client_id) {
                    Ok(_) => {
                        lim.cleanup(Duration::from_secs(5));
                        return Ok(());
                    }
                    Err(_) => {
                        delay_for(Duration::from_millis(5)).await;
                    }
                }
            }
        }
        Ok(())
    }
}
