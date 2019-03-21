// Copyright 2019 Timely Data, Inc. All rights reserved.
//
// This file is part of Materialize. Materialize may not be used or
// distributed without the express permission of Timely Data, Inc.

use futures::future;
use futures::Future;
use lazy_static::lazy_static;
use std::net::SocketAddr;
use tokio_zookeeper::error::Delete;
use tokio_zookeeper::ZooKeeper;

use ore::closure;

lazy_static! {
    pub static ref ZOOKEEPER_ADDR: SocketAddr = "127.0.0.1:2181".parse().unwrap();
}

pub fn zk_delete_all(prefix: &str) -> Result<(), failure::Error> {
    fn work(zk: ZooKeeper, path: String) -> Box<dyn Future<Item = (), Error = failure::Error>> {
        Box::new(zk.get_children(&path).and_then(move |(zk, children)| {
            let child_futs: Vec<_> = children
                .unwrap_or(vec![])
                .iter()
                .map(closure!([clone zk, clone path] |child| {
                    work(zk.clone(), format!("{}/{}", path, child))
                }))
                .collect();
            future::join_all(child_futs).and_then(move |_| {
                zk.delete(&path, None).and_then(|(_, res)| match res {
                    Ok(_) | Err(Delete::NoNode) => Ok(()),
                    Err(e) => Err(e.into()),
                })
            })
        }))
    }

    let fut = ZooKeeper::connect(&ZOOKEEPER_ADDR)
        .and_then(|(zk, _watch)| work(zk, format!("/{}", prefix)));

    tokio::runtime::current_thread::block_on_all(fut)
}