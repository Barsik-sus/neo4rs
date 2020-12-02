pub mod bye;
pub mod failure;
pub mod hello;
pub mod pull;
pub mod record;
pub mod run;
pub mod success;
use crate::error::*;
use crate::types::*;
use bye::Bye;
use bytes::*;
use failure::Failure;
use hello::Hello;
use pull::Pull;
use record::Record;
use run::Run;
use std::cell::RefCell;
use std::convert::{TryFrom, TryInto};
use std::rc::Rc;
use success::Success;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum BoltResponse {
    SuccessMessage(Success),
    FailureMessage(Failure),
    RecordMessage(Record),
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum BoltRequest {
    HelloMessage(Hello),
    RunMessage(Run),
    GoodByeMessage(Bye),
    PullMessage(Pull),
}

impl BoltRequest {
    pub fn hello(agent: &str, principal: String, credentials: String) -> BoltRequest {
        let mut data = BoltMap::new();
        data.put("user_agent".into(), agent.into());
        data.put("scheme".into(), "basic".into());
        data.put("principal".into(), principal.into());
        data.put("credentials".into(), credentials.into());
        BoltRequest::HelloMessage(Hello::new(data))
    }

    pub fn run(query: &str, params: BoltMap) -> BoltRequest {
        BoltRequest::RunMessage(Run::new(query.into(), params))
    }

    pub fn pull() -> BoltRequest {
        BoltRequest::PullMessage(Pull::default())
    }
}

impl TryInto<Bytes> for BoltRequest {
    type Error = Error;
    fn try_into(self) -> Result<Bytes> {
        let bytes: Bytes = match self {
            BoltRequest::HelloMessage(hello) => hello.try_into()?,
            BoltRequest::GoodByeMessage(bye) => bye.try_into()?,
            BoltRequest::RunMessage(run) => run.try_into()?,
            BoltRequest::PullMessage(pull) => pull.try_into()?,
        };
        Ok(bytes)
    }
}

impl TryFrom<Bytes> for BoltResponse {
    type Error = Error;

    fn try_from(response: Bytes) -> Result<BoltResponse> {
        match Rc::new(RefCell::new(response)) {
            input if success::is_present(input.clone()) => {
                Ok(BoltResponse::SuccessMessage(Success::try_from(input)?))
            }
            input if failure::is_present(input.clone()) => {
                Ok(BoltResponse::FailureMessage(Failure::try_from(input)?))
            }
            input if record::is_present(input.clone()) => {
                Ok(BoltResponse::RecordMessage(Record::try_from(input)?))
            }
            _ => Err(Error::UnknownMessageMarker),
        }
    }
}
