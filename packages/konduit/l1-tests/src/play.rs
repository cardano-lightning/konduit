use crate::config::{self, Config};
use crate::runner::Action;
use crate::scenario::Tx;
use crate::scenario::{
    l2_resolver::{self, L2Resolver},
    schedule::Schedule,
};
use crate::{Runner, now, scenario};
use konduit_data::{Constants, Stage};
use konduit_tx::{Bounds, Channel, Open, Variables};
use serde_json::{Value, json}; // adjust path if `Action` lives/re-exports elsewhere

pub async fn run(config: Config, from: usize) -> anyhow::Result<()> {
    let mut runner = Runner::build(config.clone()).await?;
    let num_accounts = runner.config().accounts.len();

    runner.reload_channels().await?;
    let n_channels = runner.channels().len();
    let n_fuel = runner.fuel().len();

    println!("n channels: {}", n_channels);

    for channel in runner.channels().iter() {
        print_channel(&config, channel.data())?;
    }

    if n_channels == 0 {
        let mut staged_tx = runner.stage_tx(Bounds::five_mins());
        for (idx, open) in config.scenario.opens.iter().enumerate() {
            let account = config.accounts[idx].clone();
            let (sub_vkey, close_period) = config.adaptor.constants();
            let constants = account.constants(sub_vkey, close_period);
            staged_tx.apply_open(Open::new(*open, constants, None));
        }
        let tx = staged_tx.commit()?;
        let id = runner.sign_and_submit(tx).await?;
        runner.wait_til(&id).await?;
    }

    let mut resolvers = config
        .accounts
        .iter()
        .zip(config.scenario.l2.iter())
        .map(|(account, stubs)| {
            L2Resolver::starting_now(
                config.l2_resolver.clone(),
                account.clone(),
                Schedule::new(stubs),
            )
            .expect("time")
        })
        .collect::<Vec<_>>();
    for (i, entry) in config.scenario.l1.iter().enumerate() {
        for resolver in resolvers.iter_mut() {
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
            resolver.tick()?;
        }
        tracing::info!(info=%i, entry=%entry);

        let mut staged_tx = runner.stage_tx(Bounds::five_mins());
        match entry {
            Tx::Consumer(btree_map) => {
                tracing::info!(entry = i, "consumer tx");
                for (account_index, step) in btree_map.iter() {
                    let account = &config.accounts[account_index.0];
                    tracing::debug!(entry = i, account = account_index.0, step = %step, "applying consumer step");
                    match step {
                        scenario::ConsumerStep::Step => {
                            // TODO(see chat): needs L2Resolver to decide
                            // Open/Add/Close/etc for this account's current
                            // stage. Don't know L2Resolver's API for this.
                        }
                        scenario::ConsumerStep::Add(amount) => {
                            staged_tx.apply_action(account, Action::Add { amount: *amount })?;
                        }
                        scenario::ConsumerStep::Close => {
                            staged_tx.apply_action(account, Action::Close)?;
                        }
                        scenario::ConsumerStep::Elapse => {
                            staged_tx.apply_action(account, Action::Elapse)?;
                        }
                        scenario::ConsumerStep::Expire => {
                            staged_tx.apply_action(account, Action::Expire)?;
                        }
                        scenario::ConsumerStep::End => {
                            staged_tx.apply_action(account, Action::End)?;
                        }
                    }
                }
            }
            Tx::Adaptor(btree_map) => {
                tracing::info!(entry = i, "adaptor tx");
                for (account_index, step) in btree_map.iter() {
                    let account = &config.accounts[account_index.0];
                    tracing::debug!(entry = i, account = account_index.0, step = %step, "applying adaptor step");
                    match step {
                        scenario::AdaptorStep::Claim => {
                            let resolver = &resolvers[account_index.0];
                            let receipt = resolver.receipt().clone();
                            tracing::debug!(
                                entry = i,
                                account = account_index.0,
                                "claiming with receipt"
                            );
                            staged_tx.apply_action(account, Action::Claim { receipt })?;
                        }
                    }
                }
            }
            Tx::Skip => {
                tracing::info!(entry = i, "skipping");
            }
        }

        if !staged_tx.is_empty() {
            tracing::info!(
                "entry {i}: committing ({} opens, {} steps)",
                staged_tx.opens_len(),
                staged_tx.steppeds_len()
            );
            let tx = staged_tx.commit()?;
            let id = runner.sign_and_submit(tx).await?;
            runner.wait_til(&id).await?;
        } else {
            tracing::info!("entry {i}: nothing to do, skipping");
        }
    }

    Ok(())
}

/// Determine whether this channel belongs to the configured provider (adaptor).
fn is_provider(config: &Config, constants: &Constants) -> bool {
    config.adaptor.verifying_key() == constants.sub_vkey
}

/// Determine which configured account this channel belongs to, if any.
fn matched_account(config: &Config, constants: &Constants) -> Option<usize> {
    config
        .accounts
        .iter()
        .position(|a| a.verifying_key() == constants.add_vkey)
}

/// Build the base JSON representation of a channel, including stage/amount.
fn channel_to_value(data: &Channel) -> serde_json::Result<Value> {
    let mut value = serde_json::to_value(data.constants())?;
    if let Some(obj) = value.as_object_mut() {
        obj.insert("stage".into(), json!(data.variables().stage()));
        obj.insert("amount".into(), json!(data.variables().amount()));
    }
    Ok(value)
}

/// Overwrite add_vkey/sub_vkey with human-readable labels where known.
fn label_channel(mut value: Value, provider: bool, account: Option<usize>) -> Value {
    if let Some(obj) = value.as_object_mut() {
        if provider {
            obj.insert("sub_vkey".to_string(), json!("PROVIDER"));
        }
        if let Some(i) = account {
            obj.insert("add_vkey".to_string(), json!(format!("ACCOUNT {i}")));
        }
    }
    value
}

/// Print a single channel: filters, labels, and pretty-prints it if relevant.
fn print_channel(config: &Config, data: &Channel) -> serde_json::Result<()> {
    let constants = data.constants();

    let provider = is_provider(config, constants);
    let account = matched_account(config, constants);

    if !provider && account.is_none() {
        return Ok(());
    }

    let value = channel_to_value(data)?;
    let value = label_channel(value, provider, account);

    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
