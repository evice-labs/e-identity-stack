use anyhow::Result;
use common::transaction::LeeTransaction;
use wallet::WalletCore;
use lee::V03State;
use sequencer_service_rpc::RpcClient;

#[tokio::main]
async fn main() -> Result<()> {
    let b64 = "AFbVCvelfW/PjFkrMQWTbtm4wSKxyE/ieDvV2GILdAn3AgAAACeSyzF25T+rS+Huh5IdKCrxUKdcRI3Jd10fMwfuJqaIVtUK96V9b8+MWSsxBZNu2bjBIrHIT+J4O9XYYgt0CfeC7sibq0gaXXC7dTqxYq4PU9/Ldh81/SDCEJEekmtvBFbVCvelfW/PjFkrMQWTbtm4wSKxyE/ieDvV2GILdAn3AQAAAA0AAAAAAAAAAAAAAAAAAACQAAAAAAAAAAMAAAACAAAAAwAAAAQAAAAFAAAABgAAAAcAAAAIAAAACQAAABAAAAARAAAAEgAAABMAAAAUAAAAFQAAABYAAAAXAAAAGAAAABkAAAAgAAAAIQAAACIAAAAjAAAAJAAAACUAAAAmAAAAJwAAACgAAAApAAAAMAAAADEAAAAyAAAAAwAAAAIAAAADAAAAAYLuyJurSBpdcLt1OrFirg9T38t2HzX9IMIQkR6Sa28EQEIPAAAAAAAAAAAAAAAAAAA4MgQAAAAAAAAAAAAAAAABAAAAQSlVHgChbdKLBJwfatyc8OVVoBgUt7f8CN49mpdu6LkgS0CnQKSwcGQDFJBZKykNhA2CpHvfmWAGY+ED6APBi6vT59LAcNFAWwnJI5kvJKWt3/5P+w7AM49ieU0DYGwp";
    let tx: LeeTransaction = serde_json::from_str(&format!("\"{}\"", b64))?;
    let LeeTransaction::Public(ptx) = tx else { panic!("not public"); };

    let wallet = WalletCore::from_env().await?;
    let client = wallet.helm_owned();
    let mut state = V03State::new();

    let prog_id = ptx.message().program_account_id;
    let acc0 = ptx.message().shard_selectors[0].account_id;
    let acc1 = ptx.message().shard_selectors[1].account_id;

    for id in [prog_id, acc0, acc1] {
        let mut acc = client.get_account(id).await?;
        if id == acc1 {
            acc.nonce = lee_core::account::Nonce(13); // Pre-state nonce before block 10426
        }
        state = state.with_public_accounts([(id, acc)]);
    }

    // Load segment accounts from header
    let prog_acc = client.get_account(prog_id).await?;
    let header_bytes = prog_acc.data.shard(lee_core::program::PROGRAM_LOADER_ACCOUNT_ID);
    let header = lee_core::program::ProgramHeader::from_bytes(header_bytes)
        .ok_or_else(|| anyhow::anyhow!("failed to parse program header"))?;

    let mut next_seg = Some(header.program_first_segment);
    while let Some(seg_id) = next_seg {
        let seg_acc = client.get_account(seg_id).await?;
        let seg_bytes = seg_acc.data.shard(lee_core::program::PROGRAM_LOADER_ACCOUNT_ID);
        let seg = program_loader_core::ProgramSegment::from_bytes(seg_bytes)
            .ok_or_else(|| anyhow::anyhow!("failed to parse segment"))?;
        next_seg = seg.next_segment;
        state = state.with_public_accounts([(seg_id, seg_acc)]);
    }

    println!("Running simulation with pre-state nonce 13 and cycle budget 1,000,000...");
    let (charge, res) = lee::ValidatedStateDiff::from_public_transaction_metered(
        &ptx, &state, 10426, 0, 1_000_000
    );
    println!("Simulation charge: {:?}", charge);
    match res {
        Ok(_) => {
            println!("SUCCESS! State diff produced without error!");
        }
        Err(e) => {
            println!("REVERT / ERROR: {:?}", e);
        }
    }

    Ok(())
}
