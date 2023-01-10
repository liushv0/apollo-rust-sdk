use std::{collections::HashMap, sync::{Mutex, Arc}, thread};

use crypto::mac::Mac;
use log::{info, debug};
use serde::{Deserialize, Serialize};

use super::{meta_server::{MetaServer, ApolloServerEnum}, error::ApolloError};

/// 包含四个元素: 实际的client, 配置缓存, close signal sender channel, config change events receiver channel
pub struct ApolloConfigClient (Arc<Mutex<(apollo_config_client, config_cache, tokio::sync::watch::Sender<bool>, tokio::sync::broadcast::Receiver<Vec<ApolloChangeEvent>>)>>);

#[allow(non_camel_case_types)]
type config_cache = Vec<Arc<Mutex<apollo_namespace>>>;

/// 获取到的配置项
#[derive(Debug)]
pub struct ApolloConfigItem {
    pub config_key: String,
    pub config_value: String,
    pub namespace: String,
}

#[allow(non_camel_case_types)]
struct apollo_config_client {
    meta_server: MetaServer,
    config_srv_list: Vec<String>,
    app_id_default: String,
    cluster_default: String,
    secret: String,
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug)]
struct apollo_namespace {
    #[serde(rename = "appId")]
    app_id: String, 
    cluster: String, 
    #[serde(rename = "namespaceName")]
    namespace: String,
    #[serde(rename = "releaseKey")]
    release_key: String,
    configurations: HashMap<String, String>,
    
    #[serde(skip_deserializing)]
    notification_id: i32,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_camel_case_types)]
struct notification_item {
    #[serde(rename = "namespaceName")]
    namespace: String,
    #[serde(rename = "notificationId")]
    notification_id: i32,
}

#[derive(Debug, Clone)]
pub struct ApolloChangeEvent {
    namespace: String,
    key: String,
    new_value: String,
    action: ApolloChangeAction,
}

#[derive(Debug, Clone)]
pub enum ApolloChangeAction {
    DELETE,
    UPDATE,
    ADD,
}


pub async fn new(meta_server: Vec<&str>, app_id: &str, cluster_name: &str, namespaces: Option<Vec<&str>>, secret: Option<&str>) -> Result<ApolloConfigClient, ApolloError> {
    let ms = MetaServer::new(meta_server);
    let config_srvs = ms.get_config_servers(ApolloServerEnum::ConfigServer).await;
    if config_srvs.is_err() {
        return Err(config_srvs.unwrap_err());
    }
    let config_srvs = config_srvs.unwrap();
    if config_srvs.len() == 0 {
        return Err(ApolloError::new(111, "no valid config server address".to_string()));
    }

    let sign = secret.unwrap_or_default();
    let cc = apollo_config_client{
        meta_server: ms,
        config_srv_list: config_srvs,
        app_id_default: app_id.to_string(),
        cluster_default: cluster_name.to_string(),
        secret: sign.to_string(),
    };
    

    let ns_filter = |nss: Option<Vec<&str>>| -> Option<Vec<String>> {
        if nss.is_none() {
            return None;
        }
        let tmp = nss.unwrap();
        let mut ns: Vec<String> = Vec::new();
        for ele in tmp {
            if !ns.contains(&ele.to_string()) {
                ns.push(ele.to_string());
            }
        }
        return Some(ns);
    };

    let (close_tx, close_rx) = tokio::sync::watch::channel(false);
    let (change_event_tx, cheange_event_rx) = tokio::sync::broadcast::channel(10);

    let cc_arc = Arc::new(Mutex::new((cc, Vec::new(), close_tx, cheange_event_rx)));
    let cc_arc_clone = cc_arc.clone();
    let apc = ApolloConfigClient(cc_arc);
    let apc_2 = ApolloConfigClient(cc_arc_clone);

    let ns = ns_filter(namespaces);
    debug!("listen namespace {:?} when initial config client", ns);
    if ns.is_some() {
        let ns = ns.unwrap();
        for ele in ns {
            let err = apc.listen_namespace(&ele).await;
            if err.is_some() {
                let ae = err.unwrap();
                log::error!("listen namespace error: {:?}", &ae);
                return Err(ae);
            }
        }
    }
    
    let _ =thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(apc_2.loop_listening(close_rx, change_event_tx));
    });

    return Ok(apc); 
}

impl ApolloConfigClient {
    ///获取配置项，后监听的namespace优先级更高
    pub fn get_config(&self, key: &str) -> Option<ApolloConfigItem> {
        let cache = &self.0.lock().unwrap().1;
        let mut idx = cache.len() as i32 - 1;

        while idx >= 0 {
            let ele = cache.get(idx as usize);
            idx -= 1;
            if ele.is_none() {
                break;
            }
            let ele = ele.unwrap();
            let an = ele.lock().unwrap();

            let value = an.configurations.get(key);
            if value.is_none() {
                continue;
            }
            let item = ApolloConfigItem{
                config_key: key.to_string(),
                config_value: value.unwrap().to_string(),
                namespace: an.namespace.clone(),
            };
            return Some(item);
        }
        return None;
    }

    pub fn get_config_from_namespace(&self, key: &str, namespace: &str) -> Option<ApolloConfigItem> {
        let cache = &self.0.lock().unwrap().1;
        for ele in cache {
            let an = ele.lock().unwrap();
            if an.namespace == namespace {
                if an.configurations.contains_key(key) {
                    let value = an.configurations.get(key);
                    let item = ApolloConfigItem {
                        config_key: key.to_string(),
                        config_value: value.unwrap().to_string(),
                        namespace: an.namespace.clone(),
                    };
                    return Some(item);
                }
            }
        }
        return None;
    }

    /// pull config from namespace, and will listen change`s notify of this namespace, if namespace has be listened already, do nothing
    /// 如果先后监听了多个namespace，排在后面的配置优先级更高
    pub async fn listen_namespace(&self, namespace: &str) -> Option<ApolloError> {
        let load_res = self.load_namespace(namespace, false, None).await;
        if load_res.is_err() {
            let err = load_res.unwrap_err();
            return Some(err);
        }
        let cfg = load_res.unwrap();
        if cfg.is_none() {
            return None;
        }
        let cfg = cfg.unwrap();
        let mut apc = self.0.lock().unwrap();
        let cache = apc.1.clone();

        for ele in cache {
            let an = ele.lock().unwrap();
            if an.namespace == namespace {
                let err_msg = format!("concurrent load namespace {} ", namespace);
                return Some(ApolloError::new(255555, err_msg));
            }
        }
        apc.1.push(Arc::new(Mutex::new(cfg)));

        None
    }

    async fn load_namespace(&self, namespace: &str, force: bool, release_key: Option<String>) -> Result<Option<apollo_namespace>, ApolloError> {
        let apc = self.0.lock().unwrap();
        let cache = apc.1.clone();
        if !force {
            for ele in cache {
                let an = ele.lock().unwrap();
                if an.namespace == namespace {
                    return Ok(None);
                }
            }
        }
        
        let cfg_srv_list = &apc.0.config_srv_list;

        let mut res_err: Option<ApolloError> = None;
        let mut rk = "".to_string();
        if !force && release_key.is_some() {
            rk = release_key.unwrap();
        }

        let cli = reqwest::Client::new();

        for cfg_srv_addr in cfg_srv_list {
            let path = format!("/configs/{appId}/{clusterName}/{namespace}?releaseKey={releaseKey}", appId = &apc.0.app_id_default, clusterName = &apc.0.cluster_default, namespace = namespace, releaseKey=rk);
            let mut req_builder = cli.get(format!("{config_server_url}{path}", config_server_url=cfg_srv_addr, path=path));

            let headers = apollo_req_sign(&apc.0.secret, &apc.0.app_id_default, &path);
            for ele in headers {
                req_builder = req_builder.header(ele.0, ele.1);
            }

            let response = req_builder.send().await;
            if response.is_err() {
                let err = response.unwrap_err();
                log::error!("apollo config request execute failed, error:{:?}", &err);
                res_err = Some(ApolloError::new(211111, err.to_string()));
                continue;
            }
            let response = response.unwrap();
            
            if response.status() == 304 {
                return Ok(None);
            }

            if response.status() != 200 {
                let err = response.error_for_status().unwrap_err();
                log::error!("read config failed! error: {:?}", &err);
                res_err = Some(ApolloError::new(222222, err.to_string()));
                continue;
            }

            let cfg_resp = response.text().await;
            if cfg_resp.is_err() {
                let err = cfg_resp.unwrap_err();
                log::error!("can not read config string from response, error:{:?}", err);
                res_err = Some(ApolloError::new(23333, err.to_string()));
                continue;
            }
            let cfg_str = cfg_resp.unwrap();

            let cfg_de: Result<apollo_namespace, serde_json::Error> = serde_json::from_str(&cfg_str);
            if cfg_de.is_err() {
                let err = cfg_de.unwrap_err();
                log::error!("deserialize config failed! response: {}, error: {:?}", cfg_str, err);
                return Err(ApolloError::new(24444, err.to_string())); //deserialize error, do not retry
            }
            let cfg = cfg_de.unwrap();

            return Ok(Some(cfg));
        }
        if res_err.is_none() {
            Ok(None)
        }else {
            Err(res_err.unwrap())
        }
    }

    pub fn close(&self) {
        let mut apc = self.0.lock().unwrap();
        let res = apc.2.send(false);
        if res.is_err() {
            log::warn!("apollo client has closed...");
            return;
        }
        apc.1.clear();
        //todo 加个close标志位？
    }

    /// try fetch change event, non block
    pub fn fetch_change_event(&self) -> Option<Vec<ApolloChangeEvent>> {
        let mut apc = self.0.lock().unwrap();
        let rec = apc.3.try_recv();
        if rec.is_err() {
            return None;
        }
        let res = rec.unwrap();
        Some(res)
    }

    async fn namespace_notify(&self) -> Vec<notification_item> {
        let mut ns_list = Vec::new();
        let apc = self.0.lock().unwrap();
        for ele in &apc.1 {
            let an = ele.lock().unwrap();
            let ni = notification_item{
                namespace: an.namespace.clone(),
                notification_id: an.notification_id,
            };
            ns_list.push(ni);
        }

        let cfg_srv_addr = apc.0.config_srv_list.get(0);
        if cfg_srv_addr.is_none() {
            log::warn!("no valid config server address...ensure server is working.....");
            return Vec::new();
        }
        let cfg_srv_addr = cfg_srv_addr.unwrap().clone();
        
        if ns_list.len() == 0 {
            drop(apc);
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            return ns_list;
        }

        let notify_str = serde_json::to_string(&ns_list).unwrap();
        let notify_str: String = url::form_urlencoded::byte_serialize(notify_str.as_bytes()).collect();

        let notify_url_path = format!("/notifications/v2?appId={}&cluster={}&notifications={}", apc.0.app_id_default, apc.0.cluster_default, notify_str);
        
        let cli = reqwest::Client::new();
        let mut req_builder = cli.get(format!("{host}{path}", host=cfg_srv_addr, path=notify_url_path));
        let headers = apollo_req_sign(&apc.0.secret, &apc.0.app_id_default, &notify_url_path);
        for ele in headers {
            req_builder = req_builder.header(ele.0, ele.1);
        }
        drop(apc);

        let resp = req_builder.send().await;
        if resp.is_err() {
            let err = resp.unwrap_err();
            log::warn!("apollo notification failed, error: {:?}", err);
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            return Vec::new();
        }
        
        let resp = resp.unwrap();
        let cont = resp.text().await;
        if cont.is_err() {
            log::warn!("read notification result failed! error: {:?}", cont.unwrap_err());
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            return Vec::new();
        }
        let cont = cont.unwrap();
        if cont.len() < 10 {
            return Vec::new();
        }

        let ns_changed: Result<Vec<notification_item>, serde_json::Error> = serde_json::from_str(&cont);
        if ns_changed.is_err() {
            log::warn!("can not deserialize notification response! response str: {}, error:{:?}", cont, ns_changed.unwrap_err());
            return Vec::new();
        }
        let ns_changed = ns_changed.unwrap();
        ns_changed
    }

    async fn loop_listening(&self, mut close_rx: tokio::sync::watch::Receiver<bool>, change_event_tx: tokio::sync::broadcast::Sender<Vec<ApolloChangeEvent>>) {
        let start = tokio::time::Instant::now().checked_add(tokio::time::Duration::from_secs(5)).unwrap();
        let mut meta_refresh_ticker = tokio::time::interval_at(start, std::time::Duration::from_secs(30));
        
        loop {
            tokio::select! {
                //监听关闭
                _ = close_rx.changed() => {
                    //todo 
                    info!("apollo client closed, listener thread exit now..");
                    return ;
                }
                //刷新config srv
                _ = meta_refresh_ticker.tick() => {
                    let mut apc = self.0.lock().unwrap();
                    let cfg_srv_res = apc.0.meta_server.get_config_servers(ApolloServerEnum::ConfigServer).await;

                    if cfg_srv_res.is_err() {
                        log::warn!("get apollo config server addr failed! {:?}", cfg_srv_res.unwrap_err());
                    }else {
                        let config_srvs = cfg_srv_res.unwrap();
                        if config_srvs.len() == 0 {
                            log::warn!("apollo config server addr list is empty, check servers status...");
                        }else {
                            debug!("apollo config  server address:{:?}", config_srvs);
                            apc.0.config_srv_list = config_srvs;
                        }
                    }
                }
                //监听配置变更
                v = self.namespace_notify() => {
                    let apc = self.0.lock().unwrap();
                    let cache = &apc.1;
                    let mut change_ns = HashMap::new();
                    let mut release_key_map = HashMap::new();

                    for ele in cache {
                        let an = ele.lock().unwrap();
                        release_key_map.insert(an.namespace.clone(), an.release_key.clone());
                    }
                    drop(cache);
                    drop(apc);

                    for ele in v {
                        let release_key = release_key_map.remove(&ele.namespace);
                        if release_key.is_none() {
                            log::warn!("invalid release key...namespace:{}", &ele.namespace);
                            continue;
                        }
                        let cfg_res = self.load_namespace(&ele.namespace, true, release_key.clone()).await;
                        if cfg_res.is_err() {
                            let err = cfg_res.unwrap_err();
                            log::error!("reload config for namespace {} failed, notifyId: {}, releaseKey:{:?}, error:{:?}", &ele.namespace, ele.notification_id, &release_key, err);
                            continue;
                        }
                        let cfg = cfg_res.unwrap();
                        if cfg.is_none() {
                            log::debug!("config no changed. namespace:{}, releaseKey:{:?}, notifyId:{}", &ele.namespace, &release_key, ele.notification_id);
                            continue;
                        }
                        let cfg = cfg.unwrap();
                        let cfg_new = apollo_namespace{
                            notification_id: ele.notification_id,
                            ..cfg
                        };
                        change_ns.insert(ele.namespace, cfg_new);
                    }
                    
                    log::debug!("config change, new config: {:?}", change_ns);
                    if change_ns.len() > 0 {
                        let mut apc = self.0.lock().unwrap();
                        let cache = &apc.1.clone();

                        let mut cache_new = Vec::new();
                        for ele in cache {
                            let an = ele.lock().unwrap();
                            if change_ns.contains_key(&an.namespace) {
                                let cfg = change_ns.remove(&an.namespace).unwrap();
                                let diff = apollo_namespace_diff(&cfg, &an);
                                if diff.len() > 0 {
                                    let _ = change_event_tx.send(diff);     //check result?
                                }
                                cache_new.push(Arc::new(Mutex::new(cfg)));
                            }else {
                                cache_new.push(ele.clone());
                            }
                        }
                        apc.1 = cache_new;
                    }

                }       //config change listening
            }
        }
    }
}

/// request signature
fn apollo_req_sign(secert: &str, app_id: &str, path: &str) -> Vec<(String, String)> {
    let mut res = Vec::new();
    if secert.len() == 0 {
        return res;
    }
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
    let raw_str = format!("{}\n{}", ts.to_string(), &path);
    let mut mac = crypto::hmac::Hmac::new(crypto::sha1::Sha1::new(), secert.as_bytes());
    mac.input(raw_str.as_bytes());
    let binding = mac.result();
    let token = binding.code();
    let token = base64::encode(token);

    let sign = format!("Apollo {}:{}", app_id, token);

    res.push(("Timestamp".to_string(), ts.to_string()));
    res.push(("Authorization".to_string(), sign));
    return res;
}

/// 比较新旧配置区别
fn apollo_namespace_diff(new_cfg: &apollo_namespace, old_cfg: &apollo_namespace) -> Vec<ApolloChangeEvent> {
    let mut res = Vec::new();
    for (key, value) in &old_cfg.configurations {
        let v_new = new_cfg.configurations.get(key);
        if v_new.is_none() {
            let event = ApolloChangeEvent{
                namespace: old_cfg.namespace.clone(),
                key: key.clone(),
                new_value: "".to_string(),
                action: ApolloChangeAction::DELETE,
            };
            res.push(event);
            continue;
        }
        let v_new = v_new.unwrap();
        if v_new != value {
            let event = ApolloChangeEvent{
                namespace: old_cfg.namespace.clone(),
                key: key.clone(),
                new_value: v_new.to_string(),
                action: ApolloChangeAction::UPDATE,
            };
            res.push(event);
            continue;
        }
        continue;
    }
    for (key, value) in &new_cfg.configurations {
        let v_old = old_cfg.configurations.get(key);
        if v_old.is_none() {
            let event = ApolloChangeEvent{
                namespace: old_cfg.namespace.clone(),
                key: key.clone(),
                new_value: value.to_string(),
                action: ApolloChangeAction::ADD,
            };
            res.push(event);
            continue;
        }
    }
    return res;
}
