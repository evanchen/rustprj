use conf::conf::Conf;
extern crate net;

#[test]
fn test_http_client() {
    let conf = Conf::new();
    let port = conf.get_http_port();
    let client = reqwest::blocking::Client::new();
    let target = format!("http://127.0.0.1:{}", port);

    let url = format!("{}/req/server/all", target);
    let res = client.get(&url).send().unwrap();
    println!("get {},\nres={}", url, res.text().unwrap());

    let url = format!("{}/req/server/1123", target);
    let res = client.get(&url).send().unwrap();
    println!("get {},\nres={}", url, res.text().unwrap());
}
