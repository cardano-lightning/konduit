use cardano_sdk::Input;
use konduit_wire::reg::cobbl3::Credential;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::server;

use super::{Policies, RegPolicy, SquashPolicy};

/// Tag belongs in global store, since used as channel id
/// with respect to single signer context.
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Config {
    /// Keeps own copy of server config.
    /// Allow the safe deletion known `servers` without breaking a channel.
    #[n(0)]
    server: server::Config,
    #[n(1)]
    policies: Policies,
    #[n(2)]
    focus: Option<Input>,
    #[n(3)]
    #[serde_as(as = "Option<serde_with::DisplayFromStr>")]
    credential: Option<Credential>,
}

impl Config {
    pub fn new(server: server::Config) -> Self {
        Self {
            server,
            policies: Policies::default(),
            focus: None,
            credential: None,
        }
    }

    pub fn server(&self) -> &server::Config {
        &self.server
    }

    pub fn set_server(&mut self, server: server::Config) {
        self.server = server
    }

    pub fn policies(&self) -> &Policies {
        &self.policies
    }
    pub fn reg_policy(&self) -> &RegPolicy {
        self.policies().reg()
    }
    pub fn set_reg_policy(&mut self, policy: RegPolicy) {
        self.policies.set_reg(policy);
    }
    pub fn squash_policy(&self) -> &SquashPolicy {
        self.policies().squash()
    }
    pub fn set_squash_policy(&mut self, policy: SquashPolicy) {
        self.policies.set_squash(policy);
    }

    pub fn focus(&self) -> Option<&Input> {
        self.focus.as_ref()
    }
    pub fn set_focus(&mut self, focus: Option<Input>) {
        self.focus = focus;
    }

    pub fn credential(&self) -> Option<&Credential> {
        self.credential.as_ref()
    }
    pub fn set_credential(&mut self, credential: Option<Credential>) {
        self.credential = credential;
    }
}
