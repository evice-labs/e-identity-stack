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

    // Reject commitments that were previously revoked via slashing.
    // This prevents re-use of a compromised identity whose NSK was exposed.
    if forum.revoked_commitments.contains(&commitment_bytes) {
        return Err("Registration failed: This commitment has been revoked.");
    }

    forum.registered_commitments.push(commitment_bytes);
    forum.member_stakes.push((commitment_bytes, stake_amount));
    forum.total_staked += stake_amount;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::initialize::process_initialize;

    #[test]
    fn test_register_success() {
        let mut forum = process_initialize(3, 2, 3).unwrap();
        let commitment = [1u8; 32];
        assert!(process_register(&mut forum, commitment, 1000).is_ok());
        assert_eq!(forum.registered_commitments.len(), 1);
        assert_eq!(forum.total_staked, 1000);
    }

    #[test]
    fn test_register_insufficient_stake() {
        let mut forum = process_initialize(3, 2, 3).unwrap();
        let commitment = [1u8; 32];
        assert!(process_register(&mut forum, commitment, 999).is_err());
    }

    #[test]
    fn test_register_duplicate_commitment() {
        let mut forum = process_initialize(3, 2, 3).unwrap();
        let commitment = [1u8; 32];
        assert!(process_register(&mut forum, commitment, 1000).is_ok());
        assert!(process_register(&mut forum, commitment, 1000).is_err());
    }

    #[test]
    fn test_register_revoked_commitment_rejected() {
        let mut forum = process_initialize(3, 2, 3).unwrap();
        let commitment = [2u8; 32];
        forum.revoked_commitments.push(commitment);
        let res = process_register(&mut forum, commitment, 1000);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            "Registration failed: This commitment has been revoked."
        );
    }
}
