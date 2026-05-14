use async_trait::async_trait;
use crate::users::User;

// Result type for command handling
#[derive(Debug, Clone, PartialEq)]
pub enum CmdResult {
    Success,
    Failure,
    Continue,
}

// Trait for module implementations
#[async_trait]
pub trait Module {
    fn read_config(&mut self, config: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}

// Trait for command implementations
#[async_trait]
pub trait Command {
    async fn handle(&self, user: &User, params: &Params) -> CmdResult;
}

// Command parameters
#[derive(Debug, Clone)]
pub struct Params {
    pub params: Vec<String>,
}

impl Params {
    pub fn new(params: Vec<String>) -> Self {
        Self { params }
    }

    pub fn get(&self, index: usize) -> Option<&String> {
        self.params.get(index)
    }

    pub fn len(&self) -> usize {
        self.params.len()
    }

    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }
}
