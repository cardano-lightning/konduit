
pub const MAINNET_START_TIME : u64=1506203091;
pub const MAINNET_FIRST_SHELLEY_SLOT : u64=4492800;
pub const PREVIEW_START_TIME : u64=1666656000;
pub const PREVIEW_FIRST_SHELLEY_SLOT : u64=0;
pub const PREPROD_START_TIME : u64=1654041600;
pub const PREPROD_FIRST_SHELLEY_SLOT : u64=86400;


use cardano_tx_builder::NetworkId;

fn start_time(network : NetworkId) -> u64 {
    if network 
}
