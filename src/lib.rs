
pub mod client;

#[cfg(test)]
mod tests {
    use crate::client::apollo_openapi_client::{ApolloOpenApiClient, CreateClusterReq, CreateNamespaceReq, CreateConfigItemReq, UpdateConfigItemReq, ReleaseConfigReq};

    use super::*;

    macro_rules! async_test {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }
    
    const META_SRV_ADDR: &str = "http://localhost:8080/";
    const NS_NS1: &str = "application";
    const NS_NS2: &str = "ns2";
    const APP_ID: &str = "SampleApp";
    const CLUSTER: &str = "DEV";
    const SECERT: &str = "10b890f2d8b642c885f250ec19f1d0ac";
    const KEY: &str = "timeout";

    const TOKEN: &str = "fea70126ea2f59d128f3d78db7c494e95fd980e6";     //SampleApp app 权限
    const THIRD_PARTY_APP: &str = "demo";   
    const PORTAL_URL: &str = "http://localhost:8070";

    #[test]
    fn test_apollo_config_cli() {
        let meta = vec![META_SRV_ADDR];
        
        let conn_future = client::apollo_config_client::new(meta, APP_ID, CLUSTER, None, Some(SECERT));

        let conn_res = async_test!(conn_future);
        assert!(conn_res.is_ok());
        
        let apc = conn_res.unwrap();
        let res = apc.listen_namespace(NS_NS1);
        let res = async_test!(res);
        assert!(res.is_none());
        
        let value = apc.get_config(KEY);
        assert!(value.is_some());
        assert_eq!(value.unwrap().config_value, "100");

        let res = apc.listen_namespace(NS_NS2);
        let res = async_test!(res);
        assert!(res.is_none());

        let value = apc.get_config(KEY);
        assert!(value.is_some());
        assert_eq!(value.unwrap().config_value, "9090");

        let value = apc.get_config_from_namespace(KEY, NS_NS1);
        assert!(value.is_some());
        assert_eq!(value.unwrap().config_value, "100");

    }

    #[test]
    fn test_init_with_ns() {
        let meta = vec![META_SRV_ADDR];
        let ns = Some(vec![NS_NS1, NS_NS2]);
        
        let conn_future = client::apollo_config_client::new(meta, APP_ID, CLUSTER, ns, Some(SECERT));

        let conn_res = async_test!(conn_future);
        assert!(conn_res.is_ok());
        
        let apc = conn_res.unwrap();
        let value = apc.get_config(KEY);
        assert!(value.is_some());
        assert_eq!(value.unwrap().config_value, "9090");

        let value = apc.get_config_from_namespace(KEY, NS_NS1);
        assert!(value.is_some());
        assert_eq!(value.unwrap().config_value, "100");

    }

    #[test]
    fn test_open_api() {
        create_config();
    }

    fn get_env_and_cluster() {
        let api_cli = ApolloOpenApiClient::new(PORTAL_URL, TOKEN);
        let app = "SampleApp";
        let res = api_cli.get_app_env_clusters(app);
        let res = async_test!(res);
        println!("{:?}", res);
    }

    fn get_apps() {
        let api_cli = ApolloOpenApiClient::new(PORTAL_URL, TOKEN);
        let res = api_cli.get_apps();
        let res = async_test!(res);
        println!("{:?}", res);
    }

    fn get_cluster() {
        let api_cli = ApolloOpenApiClient::new(PORTAL_URL, TOKEN);
        let app = "SampleApp";
        let env = "DEV";
        let cluster = "default";
        let res = api_cli.get_cluster(env, app, cluster);
        let res = async_test!(res);
        println!("{:?}", res);
    }

    fn create_cluster() {
        let api_cli = ApolloOpenApiClient::new(PORTAL_URL, TOKEN);
        let env = "DEV";
        let req = CreateClusterReq{
            name: "oapi".to_string(),
            appId: "SampleApp".to_string(),
            dataChangeCreatedBy: "apollo".to_string(),
        };
        let res = api_cli.create_cluster(env, &req);
        let res = async_test!(res);
        println!("{:?}", res);
    }

    fn get_all_namespaces() {
        let api_cli = ApolloOpenApiClient::new(PORTAL_URL, TOKEN);
        let app = "SampleApp";
        let env = "DEV";
        let cluster = "default";
        let namespace = "application";
        let res = api_cli.get_all_namespaces(env, app, cluster);
        let res = async_test!(res);
        println!("{:?}", res);
    }

    fn get_ns_detail() {
        let api_cli = ApolloOpenApiClient::new(PORTAL_URL, TOKEN);
        let app = "SampleApp";
        let env = "DEV";
        let cluster = "default";
        let namespace = "application";

        let res = api_cli.get_namespace_detail(env, app, cluster, namespace);
        let res = async_test!(res);
        println!("{:?}", res);
    }

    fn create_namespace() {
        let api_cli = ApolloOpenApiClient::new(PORTAL_URL, TOKEN);
        let app = "SampleApp";
        let env = "DEV";
        let cluster = "default";
        let namespace = "application";

        let req = CreateNamespaceReq {
            name: "newNs".to_string(),
            appId: app.to_string(),
            format: "properties".to_string(),
            isPublic: false,
            comment: "this is xx".to_string(),
            dataChangeCreatedBy: "apollo".to_string(),
        };
        let res = api_cli.create_namespace(&req);
        let res = async_test!(res);
        println!("{:?}", res);

    }

    fn get_current_editor() {
        let api_cli = ApolloOpenApiClient::new(PORTAL_URL, TOKEN);
        let app = "SampleApp";
        let env = "DEV";
        let cluster = "default";
        let namespace = "application";
        let res = api_cli.get_current_editor(env, app, cluster, namespace);
        let res = async_test!(res);
        println!("{:?}", res);
    }

    fn get_config() {
        let api_cli = ApolloOpenApiClient::new(PORTAL_URL, TOKEN);
        let app = "SampleApp";
        let env = "DEV";
        let cluster = "default";
        let namespace = "application";
        let res = api_cli.get_config(env, app, cluster, namespace, "timeout");
        let res = async_test!(res);
        println!("{:?}", res);
    }

    fn create_config() {
        let api_cli = ApolloOpenApiClient::new(PORTAL_URL, TOKEN);
        let app = "SampleApp";
        let env = "DEV";
        let cluster = "default";
        let namespace = "application";

        let req = CreateConfigItemReq{
            key: "create.new.key11".to_string(),
            value: "create.new.value".to_string(),
            comment: "xx".to_string(),
            dataChangeCreatedBy: "apollo".to_string(),
        };
        let res = api_cli.create_config(env, app, cluster, namespace, &req);
        let res = async_test!(res);
        println!("{:?}", res);
    }

    fn update_config() {
        let api_cli = ApolloOpenApiClient::new(PORTAL_URL, TOKEN);
        let app = "SampleApp";
        let env = "DEV";
        let cluster = "default";
        let namespace = "application";

        let req = UpdateConfigItemReq{
            key: "create.new.key".to_string(),
            value: "create.new.value---2s".to_string(),
            comment: "yyyy".to_string(),
            dataChangeCreatedBy: "apollo".to_string(),
            dataChangeLastModifiedBy: "apollo".to_string(),
        };
        let res = api_cli.update_config(env, app, cluster, namespace, &req);
        let res = async_test!(res);
        println!("{:?}", res);

    }

    fn delete_config() {
        let api_cli = ApolloOpenApiClient::new(PORTAL_URL, TOKEN);
        let app = "SampleApp";
        let env = "DEV";
        let cluster = "default";
        let namespace = "application";
        let res = api_cli.delete_config(env, app, cluster, namespace, "create.new.key", "apollo");
        let res = async_test!(res);
        println!("{:?}", res);
    }

    fn release_config() {
        let api_cli = ApolloOpenApiClient::new(PORTAL_URL, TOKEN);
        let app = "SampleApp";
        let env = "DEV";
        let cluster = "default";
        let namespace = "application";
        let req = ReleaseConfigReq{
            releaseTitle: "release title".to_string(),
            releaseComment: "commentxx".to_string(),
            releasedBy: "apollo".to_string(),
        };
        let res = api_cli.release_config(env, app, cluster, namespace, &req);
        let res = async_test!(res);
        println!("{:?}", res);
    }

    fn get_namespace_latest_release() {
        let api_cli = ApolloOpenApiClient::new(PORTAL_URL, TOKEN);
        let app = "SampleApp";
        let env = "DEV";
        let cluster = "default";
        let namespace = "application";
        let res = api_cli.get_namespace_latest_release(env, app, cluster, namespace);
        let res = async_test!(res);
        println!("{:?}", res);
    }

}
