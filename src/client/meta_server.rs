use std::{time::SystemTime, sync::{Arc, mpsc::{self, Sender, Receiver}}};

use log::warn;
use serde::{Serialize, Deserialize};
use tokio::{task::JoinHandle, join};

use super::error::ApolloError;

#[derive(PartialEq, Eq)]
pub enum ApolloServerEnum {
    ConfigServer,
    PortalServer,
}

#[derive(Debug)]
pub struct MetaServer {
    server_list: Vec<String>,
}

impl MetaServer {
    pub fn new(server_list: Vec<&str>) -> MetaServer {
        let mut list = Vec::new();
        for ele in server_list {
            list.push(ele.to_string());
        }
        let ms = MetaServer{
            server_list: list,
        };
        ms
    }

    pub async fn get_config_servers(&self, server_kind: ApolloServerEnum) -> Result<Vec<String>, ApolloError> {
        let mut result: Vec<String> = Vec::new();
        let kind = {
            if server_kind == ApolloServerEnum::ConfigServer {
                "APOLLO-CONFIGSERVICE"
            }else if server_kind == ApolloServerEnum::PortalServer {
                "APOLLO-ADMINSERVICE"
            }else {
                "UNKNOWN"
            }
        };
        let mut handler_list: Vec<JoinHandle<()>> = Vec::new();

        let start = SystemTime::now();
        let (tx, rx): (Sender<Result<Vec<String>, ApolloError>>, Receiver<Result<Vec<String>, ApolloError>>)= mpsc::channel();
        let client = reqwest::Client::new();
        let client = Arc::new(client);
 
        //todo 似乎没必要全轮询一遍
        for ele in &self.server_list {
            let url = format!("{}{}", ele, "/eureka/apps");
            let req = client.get(url).header(reqwest::header::ACCEPT, "application/json").build().unwrap();           
            
            let tx1 = tx.clone();
            let client = Arc::clone(&client);

            let handler = tokio::spawn(async move {
                let mut addr: Vec<String> = Vec::new();
                
                let res = client.execute(req).await;  
                if res.is_err() {
                    tx1.send(Err(ApolloError::new(1212, "meta server".to_string()))).unwrap();
                    return ;
                }
                let res_str = res.unwrap().text().await.unwrap();
                let eureka_resp: EurekaResp = serde_json::from_str(&res_str).unwrap();
                for ele in &eureka_resp.applications.application {
                    if ele.name == kind {
                        for ins in &ele.instance {
                            if ins.securePort.enabled == "false" {
                                addr.push(format!("http://{}:{}", ins.ipAddr, ins.port.port));
                            }else{
                                addr.push(format!("https://{}:{}", ins.ipAddr, ins.securePort.port));
                            }
                        }
                    }
                }
            
                tx1.send(Ok(addr)).unwrap();
            });    
            handler_list.push(handler);
        }

        for h in handler_list {
            let _ = join!(h);
        }
        
        loop {
            let r = rx.try_recv();
            if r.is_err() {
                break;
            }
            let r = r.unwrap();
            if r.is_ok() {
                for e in r.unwrap() {
                    if !result.contains(&e) {
                        result.push(e);
                    }
                }
            }else {
                warn!("read from meta server failed...{:?}", r);
            }
        }
        
        let end = SystemTime::now().duration_since(start).unwrap();

        println!("{:?} {:?}", end, result);

        Ok(result)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
struct EurekaResp {
    applications: EurekaApplications,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
struct EurekaApplications {
    application: Vec<EurekaApplication>,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
struct EurekaApplication {
    name: String,
    instance: Vec<EurekaInstance>,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
struct EurekaInstance {
    instanceId: String,
    hostName: String,
    app: String,
    ipAddr: String,
    status: String,
    homePageUrl: String,
    port: EurekaInstancePort,
    securePort: EurekaInstanceSecurePort,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct EurekaInstancePort {
    #[serde(rename = "$")]
    port: usize,
    #[serde(rename = "@enabled")]
    enabled: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
struct EurekaInstanceSecurePort {
    #[serde(rename = "$")]
    port: usize,
    #[serde(rename = "@enabled")]
    enabled: String,
}