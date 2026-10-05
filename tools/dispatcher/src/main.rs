use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::str::FromStr;
use wallet::{account::AccountIdWithPrivacy, cli::CliAccountMention, WalletCore};

#[derive(Deserialize, Debug, Clone)]
pub struct DeploymentsConfig {
    pub network: String,
    pub sequencer_rpc: String,
    pub program_header: String,
    pub default_forum_id: String,
}

pub fn get_deployments() -> DeploymentsConfig {
    const EMBEDDED: &str = include_str!("../../../deployments.json");
    if let Ok(content) = std::fs::read_to_string("deployments.json") {
        if let Ok(cfg) = serde_json::from_str(&content) {
            return cfg;
        }
    }
    serde_json::from_str(EMBEDDED).expect("Failed to parse embedded deployments.json")
}

pub fn derive_forum_pda(program_id: &lee::AccountId, forum_id: &[u8; 32]) -> lee::AccountId {
    let mut seed_forum = [0u8; 32];
    seed_forum[..5].copy_from_slice(b"forum");

    let mut hasher = Sha256::new();
    hasher.update(&seed_forum);
    hasher.update(forum_id);
    let combined: [u8; 32] = hasher.finalize().into();

    let pda_seed = lee_core::program::PdaSeed::new(combined);
    lee_core::account::AccountId::for_public_pda(program_id, &pda_seed)
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Headless On-Chain Dispatcher for eCloak on LEZ", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(long)]
    program: Option<String>,
    #[arg(long, num_args = 1.., value_delimiter = ' ')]
    accounts: Option<Vec<String>>,
    #[arg(long)]
    data_hex: Option<String>,
    #[arg(long)]
    payer: Option<String>,
    #[arg(long)]
    json: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Submit raw instruction data with specified accounts
    Raw {
        #[arg(long)]
        program: String,
        #[arg(long, num_args = 1.., value_delimiter = ' ')]
        accounts: Vec<String>,
        #[arg(long)]
        data_hex: String,
        #[arg(long)]
        payer: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Register a member username on-chain
    RegisterUsername {
        #[arg(long)]
        username: String,
        #[arg(long)]
        commitment: String,
        #[arg(long)]
        forum_id: Option<String>,
        #[arg(long)]
        program: Option<String>,
        #[arg(long)]
        forum_pda: Option<String>,
        #[arg(long)]
        payer: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Register a member identity on-chain
    RegisterMember {
        #[arg(long)]
        commitment: String,
        #[arg(long, default_value = "0")]
        stake_amount: u64,
        #[arg(long)]
        forum_id: Option<String>,
        #[arg(long)]
        program: Option<String>,
        #[arg(long)]
        forum_pda: Option<String>,
        #[arg(long)]
        payer: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Initialize forum instance state
    InitializeForum {
        #[arg(long)]
        forum_id: Option<String>,
        #[arg(long, default_value = "3")]
        k_strikes: u32,
        #[arg(long, default_value = "2")]
        n_moderators: u32,
        #[arg(long, default_value = "3")]
        m_moderators: u32,
        #[arg(long)]
        program: Option<String>,
        #[arg(long)]
        forum_pda: Option<String>,
        #[arg(long)]
        payer: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

fn extract_account_id(acc: AccountIdWithPrivacy) -> lee::AccountId {
    match acc {
        AccountIdWithPrivacy::Public(id) | AccountIdWithPrivacy::Private(id) => id,
    }
}

fn encode_u32_word(val: u32, out: &mut Vec<u8>) {
    out.extend_from_slice(&val.to_le_bytes());
}

fn parse_hex_32(hex_str: &str, field_name: &str) -> Result<[u8; 32]> {
    let clean = hex_str.trim_start_matches("0x").trim();
    let bytes = hex::decode(clean).context(format!("Invalid hex for {field_name}"))?;
    if bytes.len() != 32 {
        anyhow::bail!(
            "{field_name} must be exactly 32 bytes (64 hex characters), got {}",
            bytes.len()
        );
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

fn encode_register_username(forum_id: &[u8; 32], commitment: &[u8; 32], username: &str) -> Vec<u8> {
    let mut data = Vec::new();
    encode_u32_word(7, &mut data); // instruction index 7
    for &b in forum_id {
        encode_u32_word(b as u32, &mut data);
    }
    for &b in commitment {
        encode_u32_word(b as u32, &mut data);
    }
    encode_u32_word(username.len() as u32, &mut data);
    for b in username.bytes() {
        encode_u32_word(b as u32, &mut data);
    }
    data
}

fn encode_register_member(
    forum_id: &[u8; 32],
    commitment: &[u8; 32],
    stake_amount: u64,
) -> Vec<u8> {
    let mut data = Vec::new();
    encode_u32_word(1, &mut data); // instruction index 1
    for &b in forum_id {
        encode_u32_word(b as u32, &mut data);
    }
    for &b in commitment {
        encode_u32_word(b as u32, &mut data);
    }
    encode_u32_word((stake_amount & 0xFFFF_FFFF) as u32, &mut data);
    encode_u32_word((stake_amount >> 32) as u32, &mut data);
    data
}

fn encode_initialize_forum(
    forum_id: &[u8; 32],
    k_strikes: u32,
    n_moderators: u32,
    m_moderators: u32,
) -> Vec<u8> {
    let mut data = Vec::new();
    encode_u32_word(0, &mut data); // instruction index 0
    for &b in forum_id {
        encode_u32_word(b as u32, &mut data);
    }
    encode_u32_word(k_strikes, &mut data);
    encode_u32_word(n_moderators, &mut data);
    encode_u32_word(m_moderators, &mut data);
    data
}

async fn execute_tx(
    program_str: &str,
    accounts_str: &[String],
    instruction_data: Vec<u8>,
    payer_str: Option<&str>,
    is_json: bool,
) -> Result<()> {
    let wallet_core = WalletCore::from_env()
        .await
        .context("Failed to init wallet")?;

    let program_mention =
        CliAccountMention::from_str(program_str).map_err(|e| anyhow::anyhow!("{e}"))?;
    let program_id = extract_account_id(program_mention.resolve(wallet_core.storage())?);

    let mut mentions = Vec::new();
    for raw in accounts_str {
        let trimmed = raw.trim();
        let (clean, force_sign) = if let Some(stripped) = trimmed.strip_suffix(":sign") {
            (stripped, Some(true))
        } else if let Some(stripped) = trimmed.strip_suffix(":nosign") {
            (stripped, Some(false))
        } else {
            (trimmed, None)
        };
        let mention = CliAccountMention::from_str(clean).map_err(|e| anyhow::anyhow!("{e}"))?;
        let resolved = mention.resolve(wallet_core.storage())?;
        let id = extract_account_id(resolved);
        let should_sign =
            force_sign.unwrap_or_else(|| wallet_core.get_account_public_signing_key(id).is_some());
        let identity = if should_sign {
            wallet::AccountIdentity::Public(id)
        } else {
            wallet::AccountIdentity::PublicNoSign(id)
        };
        mentions.push(identity.select_program_shard(program_id));
    }

    let payer_id = if let Some(p) = payer_str {
        let m = CliAccountMention::from_str(p).map_err(|e| anyhow::anyhow!("{e}"))?;
        Some(extract_account_id(m.resolve(wallet_core.storage())?))
    } else {
        None
    };

    if !is_json {
        println!("📤 Submitting transaction to LEZ Sequencer...");
    }
    let tx_hash = wallet_core
        .send_pub_tx_paid_by(mentions, instruction_data, program_id, payer_id)
        .await
        .context("Failed to submit transaction to sequencer")?;

    if !is_json {
        println!("tx_hash={tx_hash}");
        println!("Waiting for confirmation...");
    }
    let poller = wallet_core.poller_helm();
    let (_, block_id) = poller
        .poll_tx(tx_hash)
        .await
        .context("Transaction confirmation error")?;

    if is_json {
        println!(
            "{}",
            json!({
                "ok": true,
                "tx_hash": tx_hash.to_string(),
                "block_id": block_id.to_string()
            })
        );
    } else {
        println!("✅ Confirmed on-chain! tx_hash={tx_hash} block_id={block_id}");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let deployments = get_deployments();
    let wallet_core = WalletCore::from_env()
        .await
        .context("Failed to init wallet")?;

    let resolve_payer = |p_opt: Option<String>| -> Result<String> {
        if let Some(p) = p_opt {
            Ok(p)
        } else {
            let (id, _) = wallet_core
                .storage()
                .key_chain()
                .public_account_ids()
                .next()
                .ok_or_else(|| anyhow::anyhow!("No public accounts found in local wallet. Please initialize or import a wallet account."))?;
            Ok(format!("Public/{id}"))
        }
    };

    let resolve_program_and_pda = |prog_opt: Option<String>,
                                   forum_id_bytes: &[u8; 32],
                                   pda_opt: Option<String>|
     -> Result<(String, String)> {
        let prog = prog_opt.unwrap_or_else(|| deployments.program_header.clone());
        let program_mention =
            CliAccountMention::from_str(&prog).map_err(|e| anyhow::anyhow!("{e}"))?;
        let prog_id = extract_account_id(program_mention.resolve(wallet_core.storage())?);

        let pda_str = if let Some(pda) = pda_opt {
            pda
        } else {
            let pda_id = derive_forum_pda(&prog_id, forum_id_bytes);
            format!("Public/{pda_id}")
        };

        Ok((prog, pda_str))
    };

    match cli.command {
        Some(Commands::RegisterUsername {
            username,
            commitment,
            forum_id,
            program,
            forum_pda,
            payer,
            json,
        }) => {
            let forum_id_str = forum_id.unwrap_or_else(|| deployments.default_forum_id.clone());
            let forum_id_bytes = parse_hex_32(&forum_id_str, "forum_id")?;
            let (effective_program, effective_forum_pda) =
                resolve_program_and_pda(program, &forum_id_bytes, forum_pda)?;
            let commitment_bytes = parse_hex_32(&commitment, "commitment")?;
            let data = encode_register_username(&forum_id_bytes, &commitment_bytes, &username);
            let effective_payer = resolve_payer(payer)?;
            let accounts = vec![
                format!("{effective_forum_pda}:nosign"),
                format!("{effective_payer}:sign"),
            ];
            execute_tx(
                &effective_program,
                &accounts,
                data,
                Some(&effective_payer),
                json,
            )
            .await
        }
        Some(Commands::RegisterMember {
            commitment,
            stake_amount,
            forum_id,
            program,
            forum_pda,
            payer,
            json,
        }) => {
            let forum_id_str = forum_id.unwrap_or_else(|| deployments.default_forum_id.clone());
            let forum_id_bytes = parse_hex_32(&forum_id_str, "forum_id")?;
            let (effective_program, effective_forum_pda) =
                resolve_program_and_pda(program, &forum_id_bytes, forum_pda)?;
            let commitment_bytes = parse_hex_32(&commitment, "commitment")?;
            let data = encode_register_member(&forum_id_bytes, &commitment_bytes, stake_amount);
            let effective_payer = resolve_payer(payer)?;
            let accounts = vec![
                format!("{effective_forum_pda}:nosign"),
                format!("{effective_payer}:sign"),
            ];
            execute_tx(
                &effective_program,
                &accounts,
                data,
                Some(&effective_payer),
                json,
            )
            .await
        }
        Some(Commands::InitializeForum {
            forum_id,
            k_strikes,
            n_moderators,
            m_moderators,
            program,
            forum_pda,
            payer,
            json,
        }) => {
            let forum_id_str = forum_id.unwrap_or_else(|| deployments.default_forum_id.clone());
            let forum_id_bytes = parse_hex_32(&forum_id_str, "forum_id")?;
            let (effective_program, effective_forum_pda) =
                resolve_program_and_pda(program, &forum_id_bytes, forum_pda)?;
            let data =
                encode_initialize_forum(&forum_id_bytes, k_strikes, n_moderators, m_moderators);
            let effective_payer = resolve_payer(payer)?;
            let accounts = vec![
                format!("{effective_forum_pda}:nosign"),
                format!("{effective_payer}:sign"),
            ];
            execute_tx(
                &effective_program,
                &accounts,
                data,
                Some(&effective_payer),
                json,
            )
            .await
        }
        Some(Commands::Raw {
            program,
            accounts,
            data_hex,
            payer,
            json,
        }) => {
            let clean_hex = data_hex.trim_start_matches("0x");
            let data = hex::decode(clean_hex).context("Failed to decode data_hex")?;
            execute_tx(&program, &accounts, data, payer.as_deref(), json).await
        }
        None => {
            let program = cli
                .program
                .context("--program is required when no subcommand is specified")?;
            let accounts = cli
                .accounts
                .context("--accounts is required when no subcommand is specified")?;
            let data_hex = cli
                .data_hex
                .context("--data-hex is required when no subcommand is specified")?;
            let clean_hex = data_hex.trim_start_matches("0x");
            let data = hex::decode(clean_hex).context("Failed to decode data_hex")?;
            execute_tx(&program, &accounts, data, cli.payer.as_deref(), cli.json).await
        }
    }
}
