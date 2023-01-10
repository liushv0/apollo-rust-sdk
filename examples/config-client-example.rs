//config client 提供基础的配置拉取/监听功能

#[tokio::main]
async fn main() {
    let meta = vec!["http://1.2.3.4:8080"];   //结尾不需要斜杠
    let app_id = "SampleApp";
    let cluster = "DEV";
    let ns = Some(vec!["application", "ns2"]);
    let secret = Some("....secret....");

    //初始化时可以不监听任何namespace  
    let acc = apollo_sdk::client::apollo_config_client::new(meta, app_id, cluster, ns, None).await;
    if acc.is_err() {
        panic!("can not connect apollo server....error:{:?}", acc.err());
    }

    let acc = acc.unwrap();

    //优先从较晚被监听的namespace中取值 即先被监听的namespace优先级更低
    let value = acc.get_config("testKey");
    if value.is_some() {
        let value = value.unwrap();
        println!("config key:{}, config value: {}, from namespace: {}", value.config_key, value.config_value, value.namespace);
    }

    let value = acc.get_config_from_namespace("testKey", "application");
    if value.is_some() {
        let value = value.unwrap();
        println!("config key:{}, config value: {}, from namespace: {}", value.config_key, value.config_value, value.namespace);
    }

    //追加监听一个新的namespace  如果已经被监听过  直接返回
    let listen_res = acc.listen_namespace("ns2").await;
    if listen_res.is_some() {
        panic!("fail to listening namespace....error:{:?}", listen_res.unwrap());
    }

    std::thread::sleep(std::time::Duration::from_secs(30));

    let change = acc.fetch_change_event();
    if change.is_some() {
        let event = change.unwrap();
        println!("change event: {:?}", event);
    }

    acc.close();
}