use crate::state::{ForumInstance, OnChainUsername};

/// Register a member identity commitment with optional staking collateral.
pub fn process_register(
    forum: &mut ForumInstance,
    commitment_bytes: [u8; 32],
    stake_amount: u64,
) -> Result<(), &'static str> {
    // Staking collateral is optional: accounts can register freely (stake_amount = 0)
    // or with bonded collateral (stake_amount > 0).

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

/// Register or update an on-chain username mapping for a commitment.
pub fn process_register_username(
    forum: &mut ForumInstance,
    commitment_bytes: [u8; 32],
    username: String,
) -> Result<(), &'static str> {
    // 1. Validate username format: 3..=32 chars, ASCII alphanumeric or underscore
    if username.len() < 3 || username.len() > 32 {
        return Err("Username must be between 3 and 32 characters");
    }
    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err("Username can only contain alphanumeric characters and underscores");
    }

    // 2. Reject if commitment is revoked
    if forum.revoked_commitments.contains(&commitment_bytes) {
        return Err("Registration failed: This commitment has been revoked.");
    }

    // 3. Ensure commitment is a registered member (if not, auto-register with 0 stake)
    if !forum.registered_commitments.contains(&commitment_bytes) {
        forum.registered_commitments.push(commitment_bytes);
        forum.member_stakes.push((commitment_bytes, 0));
    }

    // 4. Check case-insensitive uniqueness across existing registered usernames
    for entry in &forum.usernames {
        if entry.username.eq_ignore_ascii_case(&username) && entry.commitment != commitment_bytes {
            return Err("Username already taken");
        }
    }

    // 5. Update or insert mapping
    if let Some(entry) = forum
        .usernames
        .iter_mut()
        .find(|e| e.commitment == commitment_bytes)
    {
        entry.username = username;
    } else {
        forum.usernames.push(OnChainUsername {
            commitment: commitment_bytes,
            username,
        });
    }

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
        assert!(process_register(&mut forum, commitment, 150).is_ok());
        assert_eq!(forum.registered_commitments.len(), 1);
        assert_eq!(forum.total_staked, 150);
    }

    #[test]
    fn test_register_zero_stake_allowed() {
        let mut forum = process_initialize(3, 2, 3).unwrap();
        let commitment = [1u8; 32];
        assert!(process_register(&mut forum, commitment, 0).is_ok());
        assert_eq!(forum.registered_commitments.len(), 1);
        assert_eq!(forum.total_staked, 0);
    }

    #[test]
    fn test_register_duplicate_commitment() {
        let mut forum = process_initialize(3, 2, 3).unwrap();
        let commitment = [1u8; 32];
        assert!(process_register(&mut forum, commitment, 150).is_ok());
        assert!(process_register(&mut forum, commitment, 150).is_err());
    }

    #[test]
    fn test_register_revoked_commitment_rejected() {
        let mut forum = process_initialize(3, 2, 3).unwrap();
        let commitment = [2u8; 32];
        forum.revoked_commitments.push(commitment);
        let res = process_register(&mut forum, commitment, 150);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            "Registration failed: This commitment has been revoked."
        );
    }

    #[test]
    fn test_register_username_success() {
        let mut forum = process_initialize(3, 2, 3).unwrap();
        let commitment = [1u8; 32];
        assert!(process_register_username(&mut forum, commitment, "Syafiqeil".to_string()).is_ok());
        assert_eq!(forum.usernames.len(), 1);
        assert_eq!(forum.usernames[0].username, "Syafiqeil");
        assert_eq!(forum.usernames[0].commitment, commitment);
        assert!(forum.registered_commitments.contains(&commitment));
    }

    #[test]
    fn test_register_username_case_insensitive_taken() {
        let mut forum = process_initialize(3, 2, 3).unwrap();
        let comm1 = [1u8; 32];
        let comm2 = [2u8; 32];
        assert!(process_register_username(&mut forum, comm1, "Syafiqeil".to_string()).is_ok());
        assert_eq!(
            process_register_username(&mut forum, comm2, "syafiqeil".to_string()).unwrap_err(),
            "Username already taken"
        );
    }

    #[test]
    fn test_update_own_username() {
        let mut forum = process_initialize(3, 2, 3).unwrap();
        let comm = [1u8; 32];
        assert!(process_register_username(&mut forum, comm, "OldName".to_string()).is_ok());
        assert!(process_register_username(&mut forum, comm, "NewName".to_string()).is_ok());
        assert_eq!(forum.usernames.len(), 1);
        assert_eq!(forum.usernames[0].username, "NewName");
    }

    #[test]
    fn test_invalid_username_format() {
        let mut forum = process_initialize(3, 2, 3).unwrap();
        let comm = [1u8; 32];
        assert!(process_register_username(&mut forum, comm, "ab".to_string()).is_err());
        assert!(
            process_register_username(&mut forum, comm, "invalid name with space".to_string())
                .is_err()
        );
        assert!(process_register_username(&mut forum, comm, "invalid!@#$".to_string()).is_err());
    }

    #[test]
    fn test_username_revoked_commitment_rejected() {
        let mut forum = process_initialize(3, 2, 3).unwrap();
        let comm = [9u8; 32];
        forum.revoked_commitments.push(comm);
        assert!(process_register_username(&mut forum, comm, "RevokedUser".to_string()).is_err());
    }
}
