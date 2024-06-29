use std::net::Ipv4Addr;
use std::sync::Arc;

use anyhow::Context;
use chrono::Utc;
use surge_ping::{IcmpPacket, PingIdentifier, PingSequence};

use crate::db;

pub struct PingLoopOptions {
    db: db::Database,
    client: surge_ping::Client,
    delay_ms: u64,
    timeout_ms: u64,
    targets: Arc<[Ipv4Addr]>,
}

impl PingLoopOptions {
    pub fn try_from_env(db: db::Database) -> anyhow::Result<PingLoopOptions> {
        let ping_delay_ms = std::env::var("FRITZBOX_PING_DELAY_MS")
            .context("missing FRITZBOX_PING_DELAY_MS")
            .and_then(|s| {
                s.parse::<u64>()
                    .context("couldn't parse FRITZBOX_PING_DELAY_MS")
            })?;

        let ping_timeout_ms = std::env::var("FRITZBOX_PING_TIMEOUT_MS")
            .context("missing FRITZBOX_PING_TIMEOUT_MS")
            .and_then(|s| {
                s.parse::<u64>()
                    .context("couldn't parse FRITZBOX_PING_TIMEOUT_MS")
            })?;

        let ping_targets = std::env::var("FRITZBOX_PING_TARGETS_V4")
            .context("missing FRITZBOX_PING_TARGETS_V4")
            .and_then(|s| {
                s.split(',')
                    .map(|s| {
                        s.parse::<Ipv4Addr>().with_context(|| {
                            format!("couldn't parse FRITZBOX_PING_TARGETS_V4 target {}", s)
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map(|vec| vec.into())
            })?;

        let client = surge_ping::Client::new(
            &surge_ping::Config::builder()
                .kind(surge_ping::ICMP::V4)
                .build(),
        )
        .context("create ping client")?;

        Ok(PingLoopOptions {
            db,
            client,
            delay_ms: ping_delay_ms,
            timeout_ms: ping_timeout_ms,
            targets: ping_targets,
        })
    }
}

enum PingResult {
    Ok(db::Ping),
    Timeout(db::Ping),
    Err(anyhow::Error),
}

async fn ping_target(
    client: surge_ping::Client,
    target: Ipv4Addr,
    timeout_ms: u64,
    payload: Arc<[u8]>,
) -> PingResult {
    let mut pinger = client
        .pinger(target.into(), PingIdentifier(rand::random()))
        .await;
    pinger.timeout(std::time::Duration::from_millis(timeout_ms));

    let ping_result = match pinger.ping(PingSequence(0), &payload).await {
        Ok(ping_result) => ping_result,
        Err(surge_ping::SurgeError::Timeout { .. }) => {
            return PingResult::Timeout(db::Ping {
                id: None,
                target: target.to_string(),
                datetime: Utc::now(),
                duration_ms: None,
                bytes: None,
                ttl: None,
            });
        }
        Err(err) => {
            return PingResult::Err(anyhow::anyhow!(
                "couldn't ping target `{}`: {:?}",
                target,
                err
            ));
        }
    };

    let (IcmpPacket::V4(packet), duration) = ping_result else {
        return PingResult::Err(anyhow::anyhow!("unexpected ICMP packet type"));
    };
    let duration_ms = (duration.as_secs_f64() * 1e3).ceil() as i64;

    PingResult::Ok(db::Ping {
        id: None,
        target: target.to_string(),
        datetime: Utc::now(),
        duration_ms: Some(duration_ms),
        bytes: Some(payload.len() as i64),
        ttl: Some(packet.get_ttl().map_or(0, |ttl| ttl as i64)),
    })
}

pub async fn ping_loop(opts: PingLoopOptions) -> ! {
    let payload: Arc<[u8]> = Arc::new([0u8; 56]);

    let mut interval = tokio::time::interval(std::time::Duration::from_millis(opts.delay_ms));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        for target in opts.targets.iter().copied() {
            let ping_result = ping_target(
                opts.client.clone(),
                target,
                opts.timeout_ms,
                Arc::clone(&payload),
            )
            .await;

            match ping_result {
                PingResult::Ok(ping_result) | PingResult::Timeout(ping_result) => {
                    if let Err(err) = opts.db.insert_ping(&ping_result).await {
                        log::warn!("couldn't insert ping into db: {:?}", err);
                    };
                }
                PingResult::Err(err) => {
                    log::warn!("couldn't ping target: {:?}", err);
                }
            };
        }
    }
}
