// use cardano_sdk::Input;
// use konduit_data::{Duration, Lock, Locked, Secret, Tag};
// use konduit_wire::reg::cobbl3::Credential;
// use minicbor::{Decode, Encode};
// use serde::{Deserialize, Serialize};
//
// use super::Cache;
// use super::{Config, RegPolicy, SquashPolicy};
//
// #[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
// pub struct State {
//     #[n(0)]
//     config: Config,
//     #[n(1)]
//     cache: Cache,
// }
//
// impl State {
//     pub fn new(tag: Tag) -> Self {
//         Self {
//             config: Config::new(tag),
//             cache: Cache::default(),
//         }
//     }
//
//     // -- Identity / Policies (Config) --
//     pub fn tag(&self) -> Tag {
//         self.config.tag()
//     }
//     pub fn set_tag(&mut self, tag: Tag) {
//         self.config.set_tag(tag);
//     }
//     pub fn reg_policy(&self) -> RegPolicy {
//         self.config.reg_policy()
//     }
//     pub fn set_reg_policy(&mut self, policy: RegPolicy) {
//         self.config.set_reg_policy(policy);
//     }
//     pub fn squash_policy(&self) -> SquashPolicy {
//         self.config.squash_policy()
//     }
//     pub fn set_squash_policy(&mut self, policy: SquashPolicy) {
//         self.config.set_squash_policy(policy);
//     }
//     pub fn focus(&self) -> Option<&Input> {
//         self.config.focus()
//     }
//     pub fn set_focus(&mut self, focus: Option<Input>) {
//         self.config.set_focus(focus);
//     }
//
//     // -- Credential / pay request (Cache) --
//     pub fn credential(&self) -> Option<Credential> {
//         self.cache.credential()
//     }
//     pub fn set_credential(&mut self, credential: Option<Credential>) {
//         self.cache.set_credential(credential);
//     }
//     pub fn pay_request(&self) -> Option<Vec<u8>> {
//         self.cache.pay_request()
//     }
//     pub fn set_pay_request(&mut self, pay_request: Vec<u8>) {
//         self.cache.set_pay_request(pay_request);
//     }
//     pub fn clear_pay_request(&mut self) {
//         self.cache.clear_pay_request();
//     }
//
//     // -- Commitments --
//
//     /// Turn the pending pay request into a new payment, first commit
//     /// against `locked.lock()`, requested `at`. Consumes the cached
//     /// pending pay request.
//     pub(crate) fn commit_at(&mut self, at: Duration, locked: Locked) -> Result<(), Error> {
//         let pay_request = self.cache.pay_request().ok_or(Error::NoPendingPayRequest)?;
//         self.commitments.insert(at, pay_request, locked)?;
//         self.cache.clear_pay_request();
//         Ok(())
//     }
//
//     /// Retry the payment at `index` against its existing lock, with new
//     /// `amount`/`timeout`. No lock-reuse check needed — a retry never
//     /// introduces a new lock.
//     pub(crate) fn retry_at(
//         &mut self,
//         index: u64,
//         at: Duration,
//         amount: u64,
//         timeout: Duration,
//     ) -> Result<(), Error> {
//         Ok(self.commitments.retry(index, at, amount, timeout)?)
//     }
//
//     pub fn set_ok(&mut self, index: u64, at: Duration, secret: Secret) -> Result<(), Error> {
//         Ok(self.commitments.set_ok(index, at, secret)?)
//     }
//
//     pub fn set_ko(&mut self, index: u64, at: Duration, ko: Ko) -> Result<(), Error> {
//         Ok(self.commitments.set_ko(index, at, ko)?)
//     }
//
//     pub(crate) fn drop(&mut self, n: usize) -> Vec<Lock> {
//         self.commitments.drop(n)
//     }
//
//     pub fn payment_by_lock(&self, lock: &Lock) -> Option<&Entry> {
//         self.commitments.get_by_lock(lock)
//     }
//
//     pub fn payment_at_index(&self, index: u64) -> Option<&Entry> {
//         self.commitments.get(index)
//     }
//
//     pub fn payments(&self) -> impl Iterator<Item = &Entry> {
//         self.commitments.iter()
//     }
//
//     /// Delegates to `Commitments::validate`
//     pub fn validate(&self) -> Result<(), Error> {
//         Ok(self.commitments.validate()?)
//     }
//
//     pub fn decode_validated(bytes: &[u8]) -> Result<Self, Error> {
//         let state: State = minicbor::decode(bytes)?;
//         state.validate()?;
//         Ok(state)
//     }
// }
//
// #[derive(Debug, thiserror::Error)]
// pub enum Error {
//     #[error("no pending pay request to commit")]
//     NoPendingPayRequest,
//     #[error(transparent)]
//     Commitments(#[from] commitments::Error),
//     #[error(transparent)]
//     Decode(#[from] minicbor::decode::Error),
// }
