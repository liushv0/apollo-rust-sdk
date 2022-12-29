use std::{collections::HashMap};

use reqwest::{Client, StatusCode};
use serde::{Serialize, Deserialize};

use super::error::ApolloError;


#[allow(non_camel_case_types)]
pub struct ApolloOpenApiClient {
    token: String,
    portal_url: String,
    client: Client,
}

impl ApolloOpenApiClient {
    pub fn new(portal_url: &str, token: &str) -> ApolloOpenApiClient {
        let cli = reqwest::Client::new();
        let mut base_url = portal_url.to_string();
        if portal_url.ends_with("/") {
            base_url = portal_url.strip_suffix("/").unwrap().to_string();
        }
        ApolloOpenApiClient { token: token.to_string(), portal_url: base_url, client: cli }
    }

    async fn exec_req<T: for<'a> serde::Deserialize<'a>>(&self, uri: &str, method: reqwest::Method, body: Option<String>, arg: T) -> Result<T, ApolloError> {
        let url = format!("{}{}", self.portal_url, uri);
        let mut req_builder = self.client.get(&url);
        if method == reqwest::Method::POST {
            req_builder = self.client.post(&url);
        }else if method == reqwest::Method::PUT {
            req_builder = self.client.put(&url);
        }else if method == reqwest::Method::DELETE {
            req_builder = self.client.delete(&url);
        }

        if body.is_some() {
            req_builder = req_builder.body(body.unwrap());
        }

        req_builder = req_builder.header("Authorization", self.token.to_string());
        req_builder = req_builder.header("Content-Type", "application/json;charset=UTF-8");
        let resp = req_builder.send().await;
        if resp.is_err() {
            let err = resp.unwrap_err();
            log::error!("open api {} exec failed...error: {:?}", uri, err);
            return Err(ApolloError::new(330990, "open api exec failed...".to_string()));
        }
        let resp = resp.unwrap();
        let status = resp.status();
        let text = resp.text().await;
        if text.is_err() {
            let err = text.unwrap_err();
            log::error!("read open api response failed...error: {:?}", err);
            return Err(ApolloError::new(330991, "read response failed..".to_string()));
        }
        let text = text.unwrap();

        if status != StatusCode::OK {
            let res_de: Result<BadResponse, serde_json::Error> = serde_json::from_str(&text);
            if res_de.is_err() {
                log::error!("apollo portal request exec failed....unexpected response:{}", &text);
                return Err(ApolloError::new(330991, "unexpected response".to_string()));
            }
            let resp = res_de.unwrap();
            return Err(ApolloError::new(330992, resp.message));
        }

        if text.len() == 0 {
            return Ok(arg);
        }


        let res_de: Result<T, serde_json::Error> = serde_json::from_str(&text);
        if res_de.is_err() {
            let err = res_de.err();
            println!("{:?}", err);
            return Err(ApolloError::new(330993, "response deserialize failed".to_string()));
        }
        
        Ok(res_de.unwrap())

    }

    ///获取App的环境，集群信息
    pub async fn get_app_env_clusters(&self, app_id: &str) -> Result<Vec<EnvCluster>, ApolloError>{
        let uri = format!("/openapi/v1/apps/{appId}/envclusters", appId=app_id);
        let arg: Vec<EnvCluster> = Vec::new();
        return self.exec_req(&uri, reqwest::Method::GET, None, arg).await;
    }

    ///获取App信息
    pub async fn get_apps(&self) -> Result<Vec<AppInfo>, ApolloError> {
        let uri = "/openapi/v1/apps";
        let arg: Vec<AppInfo> = Vec::new();
        return self.exec_req(&uri, reqwest::Method::GET, None, arg).await;
    }

    ///获取集群接口
    pub async fn get_cluster(&self, env: &str, app_id: &str, cluster: &str) -> Result<ClusterInfo, ApolloError> {
        let uri = format!("/openapi/v1/envs/{}/apps/{}/clusters/{}", env, app_id, cluster);
        let arg: ClusterInfo = ClusterInfo::default();
        return self.exec_req(&uri, reqwest::Method::GET, None, arg).await;
    }

    ///创建集群接口
    pub async fn create_cluster(&self, env: &str, create_req: &CreateClusterReq) -> Result<ClusterInfo, ApolloError> {
        let uri = format!("/openapi/v1/envs/{}/apps/{}/clusters", env, create_req.appId);
        let arg: ClusterInfo = ClusterInfo::default();
        let srt = serde_json::to_string(&create_req).unwrap();
        return self.exec_req(&uri, reqwest::Method::POST, Some(srt), arg).await;
    }

    ///获取集群下所有Namespace信息接口
    pub async fn get_all_namespaces(&self, env: &str, app_id: &str, cluster: &str) -> Result<Vec<NamespaceDetail>, ApolloError> {
        let uri = format!("/openapi/v1/envs/{}/apps/{}/clusters/{}/namespaces", env, app_id, cluster);
        let arg: Vec<NamespaceDetail> = Vec::new();
        return self.exec_req(&uri, reqwest::Method::GET, None, arg).await;
    }

    ///获取某个Namespace信息接口
    pub async fn get_namespace_detail(&self, env: &str, app_id: &str, cluster: &str, namespace: &str) -> Result<NamespaceDetail, ApolloError> {
        let uri = format!("/openapi/v1/envs/{}/apps/{}/clusters/{}/namespaces/{}", env, app_id, cluster, namespace);
        let arg: NamespaceDetail = NamespaceDetail::default();
        return self.exec_req(&uri, reqwest::Method::GET, None, arg).await;
    }

    ///创建Namespace
    pub async fn create_namespace(&self, create_req: &CreateNamespaceReq) -> Result<NamespaceCreated, ApolloError>{
        let uri = format!("/openapi/v1/apps/{}/appnamespaces", create_req.appId);
        let arg: NamespaceCreated = NamespaceCreated::default();
        let body = serde_json::to_string(create_req).unwrap();
        return self.exec_req(&uri, reqwest::Method::POST, Some(body), arg).await;
    }

    ///获取某个Namespace当前编辑人接口
    pub async fn get_current_editor(&self, env: &str, app_id: &str, cluster: &str, namespace: &str) -> Result<CurrentEditor, ApolloError> {
        let uri = format!("/openapi/v1/envs/{}/apps/{}/clusters/{}/namespaces/{}/lock", env, app_id, cluster, namespace);
        let arg: CurrentEditor = CurrentEditor::default();
        return self.exec_req(&uri, reqwest::Method::GET, None, arg).await;
    }

    ///读取配置接口
    pub async fn get_config(&self, env: &str, app_id: &str, cluster: &str, namespace: &str, key: &str) -> Result<ConfigItem, ApolloError> {
        let uri = format!("/openapi/v1/envs/{}/apps/{}/clusters/{}/namespaces/{}/items/{}", env, app_id, cluster, namespace, key);
        let arg: ConfigItem = ConfigItem::default();
        return self.exec_req(&uri, reqwest::Method::GET, None, arg).await;
    }

    /// 新增配置接口
    pub async fn create_config(&self, env: &str, app_id: &str, cluster: &str, namespace: &str, create_req: &CreateConfigItemReq) -> Result<ConfigItem, ApolloError> {
        let uri = format!("/openapi/v1/envs/{}/apps/{}/clusters/{}/namespaces/{}/items", env, app_id, cluster, namespace);
        let arg: ConfigItem = ConfigItem::default();
        let body = serde_json::to_string(create_req).unwrap();
        return self.exec_req(&uri, reqwest::Method::POST, Some(body), arg).await;
    }

    ///修改配置接口， 无返回值
    pub async fn update_config(&self, env: &str, app_id: &str, cluster: &str, namespace: &str, create_req: &UpdateConfigItemReq) -> Result<String, ApolloError> {
        let uri = format!("/openapi/v1/envs/{}/apps/{}/clusters/{}/namespaces/{}/items/{}?createIfNotExists=true", env, app_id, cluster, namespace, create_req.key);
        let arg = "".to_string();
        let body = serde_json::to_string(create_req).unwrap();
        return self.exec_req(&uri, reqwest::Method::PUT, Some(body), arg).await;
    }

    ///删除配置接口， 无返回值
    pub async fn delete_config(&self, env: &str, app_id: &str, cluster: &str, namespace: &str, key: &str, operator: &str) -> Result<String, ApolloError> {
        let uri = format!("/openapi/v1/envs/{}/apps/{}/clusters/{}/namespaces/{}/items/{}?operator={}", env, app_id, cluster, namespace, key, operator);
        let arg = "".to_string();
        return self.exec_req(&uri, reqwest::Method::DELETE, None, arg).await;
    }

    ///发布namespace
    pub async fn release_config(&self, env: &str, app_id: &str, cluster: &str, namespace: &str, req: &ReleaseConfigReq) -> Result<ReleaseConfigResp, ApolloError> {
        let uri = format!("/openapi/v1/envs/{}/apps/{}/clusters/{}/namespaces/{}/releases", env, app_id, cluster, namespace);
        let arg: ReleaseConfigResp = ReleaseConfigResp::default();
        let body = serde_json::to_string(req).unwrap();
        return self.exec_req(&uri, reqwest::Method::POST, Some(body), arg).await;
    }

    ///获取某个Namespace当前生效的已发布配置接口
    pub async fn get_namespace_latest_release(&self, env: &str, app_id: &str, cluster: &str, namespace: &str) -> Result<ReleaseConfigResp, ApolloError> {
        let uri = format!("/openapi/v1/envs/{env}/apps/{appId}/clusters/{clusterName}/namespaces/{namespaceName}/releases/latest", env = env, appId = app_id, clusterName = cluster, namespaceName = namespace);
        let arg = ReleaseConfigResp::default();
        return self.exec_req(&uri, reqwest::Method::GET, None, arg).await;
    }

    ///回滚已发布的release 但似乎并没有哪个接口返回releaseId...wtf...
    pub async fn rollback_release(&self, env: &str, release_id: &str, operator: &str) -> Result<String, ApolloError> {
        let uri = format!("/openapi/v1/envs/{env}/releases/{releaseId}/rollback?operator={operator}", env = env, releaseId = release_id, operator = operator);
        return self.exec_req(&uri, reqwest::Method::PUT, None, "".to_string()).await;
    }

}

#[derive(Serialize, Deserialize, Debug)]
pub struct EnvCluster {
    pub env: String,
    pub clusters: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct AppInfo {
    pub name: String,
    pub appId: String,
    pub orgId: String,
    pub orgName: String,
    pub ownerName: String,
    pub ownerEmail: String,
    pub dataChangeCreatedBy: String,
    pub dataChangeLastModifiedBy: String,
    pub dataChangeCreatedTime: String,
    pub dataChangeLastModifiedTime: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[allow(non_snake_case)]
pub struct ClusterInfo {
    pub name: String,
    pub appId: String,
    pub dataChangeCreatedBy: String,
    #[serde(default)]
    pub dataChangeLastModifiedBy: String,
    pub dataChangeCreatedTime: String,
    pub dataChangeLastModifiedTime: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[allow(non_snake_case)]
pub struct CreateClusterReq {
    pub name: String,
    pub appId: String,
    pub dataChangeCreatedBy: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[allow(non_snake_case)]
pub struct NamespaceDetail {
    pub appId: String,
    pub clusterName: String,
    pub namespaceName: String,
    #[serde(default)]
    pub comment: String,
    pub format: String,
    pub isPublic: bool,
    pub items: Vec<ConfigItem>,
    pub dataChangeCreatedBy: String,
    pub dataChangeLastModifiedBy: String,
    pub dataChangeCreatedTime: String,
    pub dataChangeLastModifiedTime: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[allow(non_snake_case)]
pub struct ConfigItem {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub comment: String,
    #[serde(default)]
    pub dataChangeCreatedBy: String,
    #[serde(default)]
    pub dataChangeLastModifiedBy: String,
    #[serde(default)]
    pub dataChangeCreatedTime: String,
    #[serde(default)]
    pub dataChangeLastModifiedTime: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[allow(non_snake_case)]
pub struct NamespaceCreated {
    pub name: String,
    pub appId: String,
    pub format: String,
    pub isPublic: bool,
    pub comment: String,
    pub dataChangeCreatedBy: String,
    pub dataChangeLastModifiedBy: String,
    pub dataChangeCreatedTime: String,
    pub dataChangeLastModifiedTime: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[allow(non_snake_case)]
pub struct CreateNamespaceReq {
    pub name: String,
    pub appId: String,
    pub format: String,
    pub isPublic: bool,
    pub comment: String,
    pub dataChangeCreatedBy: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[allow(non_snake_case)]
pub struct CurrentEditor {
    pub namespaceName: String,
    pub isLocked: bool,
    #[serde(default)]
    pub lockedBy: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[allow(non_snake_case)]
pub struct CreateConfigItemReq {
    pub key: String,
    pub value: String,
    pub comment: String,
    pub dataChangeCreatedBy: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[allow(non_snake_case)]
pub struct UpdateConfigItemReq {
    pub key: String,
    pub value: String,
    pub comment: String,
    pub dataChangeLastModifiedBy: String,
    pub dataChangeCreatedBy: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ReleaseConfigReq {
    pub releaseTitle: String,
    pub releaseComment: String,
    pub releasedBy: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ReleaseConfigResp {
    pub appId: String, 
    pub clusterName: String,
    pub namespaceName: String,
    pub name: String,
    pub configurations: HashMap<String, String>,
    pub comment: String,
    pub dataChangeCreatedBy: String,
    pub dataChangeLastModifiedBy: String,
    pub dataChangeCreatedTime: String,
    pub dataChangeLastModifiedTime: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct BadResponse {
    pub exception: String,
    pub message: String, 
    #[serde(skip)]
    pub status: i8, //目前返回了一个float 应该是server端的bug 先忽略这个字段
    pub timestamp: String,
}
