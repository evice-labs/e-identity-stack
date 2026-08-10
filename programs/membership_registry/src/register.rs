use crate::state::ForumInstance;

pub fn process_register(
    forum: &mut ForumInstance,
    commitment_bytes: [u8; 32],
    stake_amount: u64,
) -> Result<(), &'static str> {
    if stake_amount < 1000 {
        return Err("Registration failed: Stake amount is below the minimum limit (1000).");
    }

    if forum.registered_commitments.contains(&commitment_bytes) {
        return Err("Registration failed: This commitment is already registered.");
    }

    forum.registered_commitments.push(commitment_bytes);
    forum.member_stakes.push((commitment_bytes, stake_amount));
    forum.total_staked += stake_amount;

    Ok(())
}
