use clap::ValueEnum;
use colored::{Color, Colorize};
use jsonrpsee::core::Serialize;
use serde::Deserialize;
use spaces_protocol::{
    bitcoin::{Amount, Network, OutPoint}, Covenant
};
use spaces_wallet::{
    address::SpaceAddress,
    bdk_wallet::KeychainKind,
    bitcoin::{Address, Txid},
    tx_event::{
        BidEventDetails, BidoutEventDetails, OpenEventDetails, SendEventDetails,
        TransferEventDetails, TxEventKind,
    },
    Balance, DoubleUtxo, WalletOutput,
};
use tabled::{Table, Tabled};

use crate::{
    rpc::ServerInfo,
    wallets::{ListSpacesResponse, TxInfo, TxResponse, WalletResponse},
};
use crate::wallets::{WalletInfoWithProgress, WalletStatus};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Text,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FormatRpcError {
    code: i32,
    message: String,
}

#[derive(Tabled)]
#[tabled(rename_all = "UPPERCASE")]
struct PendingSpaces {
    space: String,
}

#[derive(Tabled)]
#[tabled(rename_all = "UPPERCASE")]
struct WinningSpaces {
    space: String,
    bid: u64,
    #[tabled(rename = "CLAIM AT")]
    claim_at: String,
    #[tabled(rename = "DAYS LEFT")]
    days_left: String,
}

#[derive(Tabled)]
#[tabled(rename_all = "UPPERCASE")]
struct OutbidSpaces {
    space: String,
    #[tabled(rename = "LAST CONFIRMED BID")]
    last_confirmed_bid: u64,
    #[tabled(rename = "DAYS LEFT")]
    days_left: String,
}

#[derive(Tabled)]
#[tabled(rename_all = "UPPERCASE")]
struct RegisteredSpaces {
    space: String,
    #[tabled(rename = "EXPIRE AT")]
    expire_at: usize,
    #[tabled(rename = "DAYS LEFT")]
    days_left: String,
    #[tabled(rename = "UTXO")]
    utxo: OutPoint,
}

#[derive(Tabled)]
#[tabled(rename_all = "UPPERCASE")]
struct UnspentOutput {
    outpoint: OutPoint,
    value: Amount,
    confirmed: bool,
    external: bool,
}

#[derive(Tabled)]
#[tabled(rename_all = "UPPERCASE")]
struct Bidout {
    txid: Txid,
    vout_1: u32,
    vout_2: u32,
    confirmed: bool,
}

fn format_days_left(current_block: u32, claim_height: Option<u32>) -> String {
    match claim_height {
        Some(height) => {
            let blocks_remaining = height as isize - current_block as isize;
            let days_remaining = (blocks_remaining as f64 / 6.0 / 24.0).max(0.0);
            format!("{:.2}", days_remaining)
        }
        None => "--".to_string(),
    }
}

pub fn print_list_bidouts(bidouts: Vec<DoubleUtxo>, format: Format) {
    match format {
        Format::Text => {
            let all: Vec<_> = bidouts
                .into_iter()
                .map(|out| Bidout {
                    txid: out.spend.outpoint.txid,
                    vout_1: out.spend.outpoint.vout,
                    vout_2: out.auction.outpoint.vout,
                    confirmed: out.confirmed,
                })
                .collect();
            println!("{}", ascii_table(all));
        }
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(&bidouts).unwrap());
        }
    }
}

pub fn print_list_transactions(txs: Vec<TxInfo>, format: Format) {
    match format {
        Format::Text => {
            println!("{}", ascii_table(txs));
        }
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(&txs).unwrap());
        }
    }
}

pub fn print_list_unspent(utxos: Vec<WalletOutput>, format: Format) {
    match format {
        Format::Text => {
            let utxos: Vec<_> = utxos
                .iter()
                .map(|utxo| UnspentOutput {
                    outpoint: utxo.output.outpoint,
                    confirmed: utxo.output.chain_position.is_confirmed(),
                    value: utxo.output.txout.value,
                    external: utxo.output.keychain == KeychainKind::External,
                })
                .collect();
            println!("{}", ascii_table(utxos))
        }
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(&utxos).unwrap());
        }
    }
}

pub fn print_list_wallets(wallets: Vec<String>, format: Format) {
    match format {
        Format::Text => {
            println!("{}", wallets.join("\n"));
        }
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(&wallets).unwrap());
        }
    }
}

pub fn print_server_info(info: ServerInfo, format: Format) {
    match format {
        Format::Text => {
            println!("Network: {}", info.network);
            println!("Height {}", info.tip.height);
            println!("Hash {}", info.tip.hash);
            println!("Progress {:.2}%", info.progress * 100.0);
        }
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(&info).unwrap());
        }
    }
}

pub fn print_wallet_info(prog: WalletInfoWithProgress, format: Format) {
    match format {
        Format::Text => {
            println!("WALLET: {}", prog.info.label);
            println!("  Tip {}\n  Birthday {}", prog.info.tip, prog.info.start_block);

            println!("  Public descriptors");
            for desc in prog.info.descriptors {
                println!("    {}", desc.descriptor);
            }

            // Print sync status
            println!("  Sync Status:");
            let p = prog.sync.progress.unwrap_or(0.0);
            match prog.sync.status {
                WalletStatus::HeadersSync  => {
                    println!("    Syncing block headers");
                }
                WalletStatus::ChainSync => {
                    println!("    Chain Syncing: {:.1}%", p * 100.0);
                }
                WalletStatus::SpacesSync => {
                    println!("    Spaces Syncing: {:.1}%", p * 100.0);
                }
                WalletStatus::CbfFilterSync  => {
                    println!("    Filters Syncing: {:.1}%", p * 100.0);
                }
                WalletStatus::CbfProcessFilters => {
                    println!("    Processing Filters: {:.1}%", p* 100.0);
                }
                WalletStatus::CbfDownloadMatchingBlocks => {
                    println!("    Downloading Matching Blocks: {:.1}%", p * 100.0);
                }
                WalletStatus::CbfProcessMatchingBlocks => {
                    println!("    Processing Matching Blocks: {:.1}%",p * 100.0);
                }
                WalletStatus::Syncing => {
                    println!("    Syncing: In progress ({:.1}%):", prog.info.progress * 100.0);
                }
                WalletStatus::CbfApplyUpdate => {
                    println!("    Applying compact filters update");
                }
                WalletStatus::Complete => {
                    println!("    Complete");
                }

            }

            println!();
        }
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(&prog.info).unwrap());
        }
    }
}

fn ascii_table<I, T>(iter: I) -> String
where
    I: IntoIterator<Item=T>,
    T: Tabled,
{
    Table::new(iter)
        .with(tabled::settings::Style::modern_rounded())
        .to_string()
}

pub fn print_wallet_balance_response(balance: Balance, format: Format) {
    match format {
        Format::Text => {
            println!("Balance: {}", balance.balance.to_sat());
            println!(
                "  Confirmed         {:>14}",
                balance.details.balance.confirmed.to_sat()
            );
            println!(
                "  Trusted pending   {:>14}",
                balance.details.balance.trusted_pending.to_sat()
            );
            println!(
                "  Untrusted pending {:>14}",
                balance.details.balance.untrusted_pending.to_sat()
            );
            println!("  Dust & in-auction {:>14}", balance.details.dust.to_sat());
        }
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(&balance).unwrap());
        }
    }
}

pub fn print_list_spaces_response(
    current_block: u32,
    response: ListSpacesResponse,
    format: Format,
) {
    match format {
        Format::Text => {
            let mut pendings = Vec::new();
            let mut outbids = Vec::new();
            let mut winnings = Vec::new();
            let mut owned = Vec::new();
            for slabel in response.pending {
                pendings.push(PendingSpaces { space: slabel.to_string() });
            }
            for res in response.outbid {
                let space = res.spaceout.space.as_ref().expect("space");
                let mut outbid = OutbidSpaces {
                    space: space.name.to_string(),
                    last_confirmed_bid: 0,
                    days_left: "".to_string(),
                };
                match space.covenant {
                    Covenant::Bid {
                        total_burned,
                        claim_height,
                        ..
                    } => {
                        outbid.last_confirmed_bid = total_burned.to_sat();
                        outbid.days_left = format_days_left(current_block, claim_height);
                    }
                    _ => {}
                }
                outbids.push(outbid);
            }

            for res in response.winning {
                let space = res.spaceout.space.as_ref().expect("space");
                let mut winning = WinningSpaces {
                    space: space.name.to_string(),
                    bid: 0,
                    days_left: "--".to_string(),
                    claim_at: "--".to_string(),
                };
                match space.covenant {
                    Covenant::Bid {
                        total_burned,
                        claim_height,
                        ..
                    } => {
                        winning.bid = total_burned.to_sat();
                        winning.claim_at = claim_height
                            .map(|h| h.to_string())
                            .unwrap_or("--".to_string());
                        winning.days_left = format_days_left(current_block, claim_height);
                        if winning.days_left == "0.00" {
                            winning.days_left = "Ready to claim".to_string();
                        }
                    }
                    _ => {}
                }
                winnings.push(winning);
            }
            for res in response.owned {
                let space = res.spaceout.space.as_ref().expect("space");
                let mut registered = RegisteredSpaces {
                    space: space.name.to_string(),
                    expire_at: 0,
                    days_left: "--".to_string(),
                    utxo: res.outpoint(),
                };
                match &space.covenant {
                    Covenant::Transfer { expire_height, .. } => {
                        registered.expire_at = *expire_height as _;
                        registered.days_left =
                            format_days_left(current_block, Some(*expire_height));
                    }
                    _ => {}
                }
                owned.push(registered);
            }

            if !pendings.is_empty() {
                println!("⏳ PENDING ({} spaces): ", pendings.len().to_string().bold());
                let table = ascii_table(pendings);
                println!("{}", table);
            }

            if !outbids.is_empty() {
                println!("⚠️ OUTBID ({} spaces): ", outbids.len().to_string().bold());
                let table = ascii_table(outbids);
                println!("{}", table);
            }

            if !winnings.is_empty() {
                println!(
                    "{} WINNING ({} spaces):",
                    "✓".color(Color::Green),
                    winnings.len().to_string().bold()
                );
                let table = ascii_table(winnings);
                println!("{}", table);
            }

            if !owned.is_empty() {
                println!(
                    "{} ({} spaces): ",
                    "🔑 OWNED",
                    owned.len().to_string().bold()
                );
                let table = ascii_table(owned);
                println!("{}", table);
            }
        }
        Format::Json => println!("{}", serde_json::to_string_pretty(&response).unwrap()),
    }
}
pub fn print_wallet_response(network: Network, response: WalletResponse, format: Format) {
    match format {
        Format::Text => print_wallet_response_text(network, response),
        Format::Json => println!("{}", serde_json::to_string_pretty(&response).unwrap()),
    }
}

pub fn print_wallet_response_text(network: Network, response: WalletResponse) {
    let mut main_txs = Vec::new();
    let mut secondary_txs = Vec::new();

    for tx in response.result {
        if tx.events.iter().any(|event| match event.kind {
            TxEventKind::Open
            | TxEventKind::Bid
            | TxEventKind::Register
            | TxEventKind::Transfer
            | TxEventKind::Send
            | TxEventKind::Renew
            | TxEventKind::Buy => true,
            _ => false,
        }) {
            main_txs.push(tx);
        } else {
            secondary_txs.push(tx);
        }
    }

    for tx in main_txs {
        print_tx_response(network, tx);
    }

    for tx in secondary_txs {
        print_tx_response(network, tx);
    }
}

pub fn print_error_rpc_response(code: i32, message: String, format: Format) {
    match format {
        Format::Text => {
            println!("⚠️ {}", message);
        }
        Format::Json => {
            let error = FormatRpcError {
                code,
                message: message.to_string(),
            };
            println!("{}", serde_json::to_string_pretty(&error).unwrap());
        }
    }
}

fn print_tx_response(network: Network, response: TxResponse) {
    match response.error {
        None => {
            println!("{} Transaction {}", "✓".color(Color::Green), response.txid);
        }
        Some(errors) => {
            println!("⚠️ Transaction failed to broadcast");
            for (key, value) in errors.iter() {
                println!("{}: {}", key, value);
            }

            println!("\nAttempted actions:")
        }
    }

    for event in response.events {
        println!(
            " - {} {}",
            capitalize(event.kind.to_string()),
            event.space.unwrap_or("".to_string())
        );

        match event.kind {
            TxEventKind::Open => {
                let open_details: OpenEventDetails =
                    serde_json::from_value(event.details.expect("details"))
                        .expect("deserialize open event");

                println!("   Initial bid: {}", open_details.initial_bid.to_sat());
            }
            TxEventKind::Bid => {
                let bid_details: BidEventDetails =
                    serde_json::from_value(event.details.expect("details"))
                        .expect("deserialize bid event");
                println!(
                    "   New bid: {} (previous {})",
                    bid_details.current_bid.to_sat(),
                    bid_details.previous_bid.to_sat()
                );
            }
            TxEventKind::Send => {
                let send_details: SendEventDetails =
                    serde_json::from_value(event.details.expect("details"))
                        .expect("deserialize send event");

                let addr =
                    Address::from_script(send_details.recipient_script_pubkey.as_script(), network)
                        .expect("valid address");

                println!("   Amount: {}", send_details.amount.to_sat());
                println!("   Recipient: {}", addr);
            }
            TxEventKind::Transfer => {
                let transfer_details: TransferEventDetails =
                    serde_json::from_value(event.details.expect("details"))
                        .expect("deserialize transfer event");

                let addr = SpaceAddress(
                    Address::from_script(transfer_details.script_pubkey.as_script(), network)
                        .expect("valid address"),
                );
                println!("   Recipient: {}", addr);
            }
            TxEventKind::Bidout => {
                let bidout: BidoutEventDetails =
                    serde_json::from_value(event.details.expect("details"))
                        .expect("deserialize bidout event");
                println!("   Count: {}", bidout.count);
            }
            _ => {}
        }
    }
}

fn capitalize(mut s: String) -> String {
    if let Some(first) = s.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    s
}
