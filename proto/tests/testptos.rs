use proto::{MsgRead, MsgWrite};

extern crate proto;

#[test]
fn testallptos() {
    println!("allptos:");
}

#[test]
fn testrwpto() {
    let mut s_equip_bag = proto::s_item_bag::s_item_bag::default();
    s_equip_bag.bagtype = 255;
    println!("{:?}", s_equip_bag);
    let msglen = s_equip_bag.size();
    println!("s_equip_bag.size: {}", s_equip_bag.size());
    let mut buf = Vec::with_capacity(msglen);
    let mut w = proto::BytesWriter::new(&mut buf);
    s_equip_bag.write(&mut w).unwrap();
    println!("s_equip_bag into buf: {:?}", w);

    let mut r = proto::BytesReader::new(0, buf.len());
    let s2 = proto::s_item_bag::s_item_bag::read(&mut r, &buf).unwrap();
    println!("s_equip_bag from buf: {:?}", s2);
}
