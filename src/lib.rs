use std::str::FromStr;


pub mod client;


#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! async_test {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }
    
    const META_SRV_ADDR: &str = "http://127.0.0.1:8080";
    const NS_NS1: &str = "application";
    const NS_NS2: &str = "ns2";
    const APP_ID: &str = "SampleApp";
    const CLUSTER: &str = "DEV";
    const SECERT: &str = "10b890f2d8b642c885f250ec19f1d0ac";
    const KEY: &str = "timeout";

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
}
