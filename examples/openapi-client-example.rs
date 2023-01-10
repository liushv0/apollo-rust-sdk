//openapi client 提供Apollo 开放平台接口操作能力
//接口信息参考 https://www.apolloconfig.com/#/zh/usage/apollo-open-api-platform?id=%e4%b8%89%e3%80%81-%e6%8e%a5%e5%8f%a3%e6%96%87%e6%a1%a3

#[tokio::main]
async fn main() {
    let portal_url = "http://1.2.3.4:8070";
    let token = "open_api_token";
    let app = "SampleApp";
    let env = "DEV";
    let cluster = "default";
    
    let api_cli = apollo_sdk::client::apollo_openapi_client::ApolloOpenApiClient::new(portal_url, token);
    let res = api_cli.get_all_namespaces(env, app, cluster).await;
    if res.is_ok() {
        println!("all namespace: {:?}", res);
    }else {
        println!("api call failed....error: {:?}", res.unwrap_err());
    }
    

    let res = api_cli.get_apps().await;
    if res.is_ok() {
        println!("all application: {:?}", res);
    }else {
        println!("api call failed....error: {:?}", res.unwrap_err());
    }
}