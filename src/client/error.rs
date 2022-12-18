#[derive(Debug)]
pub struct ApolloError {
    pub code: usize,
    pub msg: String,
    // source_error: dyn std::error::Error,
}

impl std::fmt::Display for ApolloError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CustomError is here!")
    }
}

impl std::error::Error for ApolloError {
    // fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    //     // Some(&self.err)
    // }
}

impl ApolloError {
    pub fn new(code: usize, msg: String) -> ApolloError {
        ApolloError { code: code, msg: msg} //, source_error: source }
    }
}