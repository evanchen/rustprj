use std::{thread, time};
extern crate net;
use net::rpc::rpc_sender;
use proto::allptos::ProtoType;
use rand::Rng;

#[test]
fn test_rpc_client() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _ = rt.block_on(async move {
        let mut rpc_sender = rpc_sender::RpcSender::new();
        let mut m = std::collections::HashMap::new();
        let mut rng = rand::thread_rng();
        for _ in 0..10000 {
            let proto_id = 101;
            let s_login = proto::s_login::s_login::default_with_random_value();
            let hostid = rng.gen_range(1001..1010);
            let _ = rpc_sender
                .send2host(hostid, proto_id, ProtoType::s_login(s_login))
                .await;
            if !m.contains_key(&hostid) {
                m.insert(hostid, true);
                println!("tick ");
                thread::sleep(time::Duration::from_millis(500));
            }
        }
    });
}
