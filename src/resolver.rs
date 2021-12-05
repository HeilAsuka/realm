use anyhow::{anyhow, Error, Result};
use std::net::{self, IpAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use trust_dns_resolver::config::*;
use trust_dns_resolver::TokioAsyncResolver;

async fn resolve_single(resolver: &TokioAsyncResolver, addr: &str) -> Result<net::IpAddr, Error> {
    if addr.parse::<net::IpAddr>().is_ok() {
        return Ok(addr.parse::<net::IpAddr>().unwrap());
    }

    let remote_addr = format!("{}.", addr);
    let res = resolver.lookup_ip(remote_addr).await.unwrap();

    match res.iter().find(|ip| ip.is_ipv4()) {
        Some(ip_v4) => Ok(ip_v4),
        None => {
            if let Some(ip_v6) = res.iter().find(|ip| ip.is_ipv6()) {
                Ok(ip_v6)
            } else {
                Err(anyhow!("Cannot resolve {}", addr))
            }
        }
    }
}

pub async fn resolve(addr_list: Vec<String>, ip_list: Vec<Arc<RwLock<net::IpAddr>>>) {
    let resolver =
        async { TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default()) }
            .await
            .unwrap();
    let cache: IpAddr = "0.0.0.0".parse().unwrap();
    let size = addr_list.len();
    let mut cache_list = vec![cache; size];
    loop {
        for (i, addr) in addr_list.iter().enumerate() {
            match resolve_single(&resolver, addr).await {
                Ok(new_ip) => {
                    if new_ip != cache_list[i] {
                        cache_list[i] = new_ip;
                        let mut w = ip_list[i].write().await;
                        *w = new_ip;
                        drop(w);
                    }
                }
                Err(_) => panic!("Cannot resolve address {}", addr),
            }
        }

        sleep(Duration::from_secs(60)).await;
    }
}
