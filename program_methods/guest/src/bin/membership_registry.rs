#![no_main]

use membership_registry::{ 
    state::ForumInstance,
    initialize, join_room, record_strike, register, 
    register_room, slash, verify_post,
};
use nssa_core::account::AccountWithMetadata;
use spel_framework::prelude::*;

risc0_zkvm::guest::entry!(main);

#[lez_program]
mod forum_registry {

    #[instruction]
    pub fn initialize_forum(
        #[account(init, pda = [literal("forum"), arg("forum_id")])] state: AccountWithMetadata,
        #[account(signer)] admin: AccountWithMetadata,
        forum_id: [u8; 32],
        k_strikes: u32,
        n_moderators: u32,
        m_moderators: u32,
    ) -> SpelResult {
        let mut forum = initialize::process_initialize(k_strikes, n_moderators, m_moderators)
            .map_err(|e| spel_framework::error::SpelError::Custom {
                code: 1,
                message: e.into(),
            })?;

        forum.admin_pubkey = *admin.account_id.value();

        let mut state = state;
        state.account.data = borsh::to_vec(&forum)
            .map_err(|_| spel_framework::error::SpelError::Custom {
                code: 2,
                message: "Serialization error".into(),
            })?
            .try_into()
            .map_err(|_| spel_framework::error::SpelError::Custom {
                code: 9,
                message: "Data too large".into(),
            })?;

        Ok(SpelOutput::execute(
            vec![state.account, admin.account],
            vec![],
        ))
    }

    #[instruction]
    pub fn register_member(
        #[account(mut, pda = [literal("forum"), arg("forum_id")])] state: AccountWithMetadata,
        #[account(signer)] member: AccountWithMetadata,
        forum_id: [u8; 32],
        commitment: [u8; 32],
        stake_amount: u64,
    ) -> SpelResult {
        let mut forum: ForumInstance = borsh::from_slice(&state.account.data).map_err(|_| {
            spel_framework::error::SpelError::Custom {
                code: 3,
                message: "Deserialization error".into(),
            }
        })?;

        register::process_register(&mut forum, commitment, stake_amount).map_err(|e| {
            spel_framework::error::SpelError::Custom {
                code: 4,
                message: e.into(),
            }
        })?;

        // Stake deduction via chained call (CPI) to auth-transfer.
        // LEZ Rule 5 only lets the owning program decrease a balance.
        // Since the member account is owned by auth-transfer, we delegate
        // the balance deduction instead of mutating it directly.
        // See: https://github.com/logos-co/spel/pull/268
        let stake_u128 = stake_amount as u128;
        if member.account.balance < stake_u128 {
            return Err(spel_framework::error::SpelError::Custom {
                code: 13,
                message: "Insufficient balance for stake".into(),
            });
        }

        let auth_transfer_id: nssa_core::program::ProgramId = member.account.program_owner;

        let mut authorized_member = member.clone();
        authorized_member.is_authorized = true;

        let stake_call = nssa_core::program::ChainedCall::new(
            auth_transfer_id,
            vec![authorized_member, state.clone()],
            &authenticated_transfer_core::Instruction::Transfer {
                amount: stake_u128,
            },
        );

        let mut state = state;
        state.account.data = borsh::to_vec(&forum)
            .map_err(|_| spel_framework::error::SpelError::Custom {
                code: 5,
                message: "Serialization error".into(),
            })?
            .try_into()
            .map_err(|_| spel_framework::error::SpelError::Custom {
                code: 10,
                message: "Data too large".into(),
            })?;

        // Member account is returned unchanged - the actual balance mutation
        // is delegated to auth-transfer via the chained call.
        Ok(SpelOutput::execute(
            vec![state.account, member.account],
            vec![stake_call],
        ))
    }

    #[instruction]
    pub fn join_room(
        #[account(mut, pda = [literal("forum"), arg("forum_id")])] state: AccountWithMetadata,
        #[account(signer)] member: AccountWithMetadata,
        forum_id: [u8; 32],
        room_id: [u8; 32],
        member_commitment: [u8; 32],
    ) -> SpelResult {
        let mut forum: ForumInstance = borsh::from_slice(&state.account.data).map_err(|_| {
            spel_framework::error::SpelError::Custom {
                code: 30,
                message: "Deserialization error".into(),
            }
        })?;

        join_room::process_join_room(&mut forum, room_id, member_commitment).map_err(|e| {
            spel_framework::error::SpelError::Custom {
                code: 31,
                message: e.into(),
            }
        })?;

        let mut state = state;
        state.account.data = borsh::to_vec(&forum)
            .map_err(|_| spel_framework::error::SpelError::Custom {
                code: 32,
                message: "Serialization error".into(),
            })?
            .try_into()
            .map_err(|_| spel_framework::error::SpelError::Custom {
                code: 33,
                message: "Data too large".into(),
            })?;

        Ok(SpelOutput::execute(vec![state.account, member.account], vec![]))
    }

    #[instruction]
    pub fn register_room(
        #[account(mut, pda = [literal("forum"), arg("forum_id")])] state: AccountWithMetadata,
        #[account(signer)] admin: AccountWithMetadata,
        forum_id: [u8; 32],
        admin_commitment: [u8; 32],
        n_mod_threshold: u32,
        m_mod_total: u32,
        moderator_pubkeys: Vec<[u8; 32]>,
        min_members_for_maturity: u32,
    ) -> SpelResult {
        let mut forum: ForumInstance = borsh::from_slice(&state.account.data).map_err(|_| {
            spel_framework::error::SpelError::Custom {
                code: 14,
                message: "Deserialization error".into(),
            }
        })?;

        if forum.admin_pubkey != *admin.account_id.value() {
            return Err(spel_framework::error::SpelError::Custom {
                code: 18,
                message: "Unauthorized: only admin can register room".into(),
            });
        }

        register_room::process_register_room(
            &mut forum,
            admin_commitment,
            n_mod_threshold,
            m_mod_total,
            moderator_pubkeys,
            min_members_for_maturity,
        )
        .map_err(|e| spel_framework::error::SpelError::Custom {
            code: 15,
            message: e.into(),
        })?;

        let mut state = state;
        state.account.data = borsh::to_vec(&forum)
            .map_err(|_| spel_framework::error::SpelError::Custom {
                code: 16,
                message: "Serialization error".into(),
            })?
            .try_into()
            .map_err(|_| spel_framework::error::SpelError::Custom {
                code: 17,
                message: "Data too large".into(),
            })?;

        Ok(SpelOutput::execute(
            vec![state.account, admin.account],
            vec![],
        ))
    }

    #[instruction]
    pub fn record_strike(
        #[account(mut, pda = [literal("forum"), arg("forum_id")])] state: AccountWithMetadata,
        forum_id: [u8; 32],
        room_id: [u8; 32],
        target_commitment: [u8; 32],
        evidence_hash: [u8; 32],
        n_valid_sigs: u32,
    ) -> SpelResult {
        let mut forum: ForumInstance = borsh::from_slice(&state.account.data).map_err(|_| {
            spel_framework::error::SpelError::Custom {
                code: 19,
                message: "Deserialization error".into(),
            }
        })?;

        record_strike::process_record_strike(
            &mut forum,
            room_id,
            target_commitment,
            evidence_hash,
            n_valid_sigs,
        )
        .map_err(|e| spel_framework::error::SpelError::Custom {
            code: 20,
            message: e.into(),
        })?;

        let mut state = state;
        state.account.data = borsh::to_vec(&forum)
            .map_err(|_| spel_framework::error::SpelError::Custom {
                code: 21,
                message: "Serialization error".into(),
            })?
            .try_into()
            .map_err(|_| spel_framework::error::SpelError::Custom {
                code: 22,
                message: "Data too large".into(),
            })?;

        Ok(SpelOutput::execute(vec![state.account], vec![]))
    }

    #[instruction]
    pub fn verify_post(
        #[account(mut, pda = [literal("forum"), arg("forum_id")])] state: AccountWithMetadata,
        forum_id: [u8; 32],
        registry_root: [u8; 32],
        tracing_tag: [u8; 32],
    ) -> SpelResult {
        let mut forum: ForumInstance = borsh::from_slice(&state.account.data).map_err(|_| {
            spel_framework::error::SpelError::Custom {
                code: 23,
                message: "Deserialization error".into(),
            }
        })?;

        verify_post::process_verify_post(&mut forum, registry_root, tracing_tag).map_err(|e| {
            spel_framework::error::SpelError::Custom {
                code: 24,
                message: e.into(),
            }
        })?;

        let mut state = state;
        state.account.data = borsh::to_vec(&forum)
            .map_err(|_| spel_framework::error::SpelError::Custom {
                code: 25,
                message: "Serialization error".into(),
            })?
            .try_into()
            .map_err(|_| spel_framework::error::SpelError::Custom {
                code: 26,
                message: "Data too large".into(),
            })?;

        Ok(SpelOutput::execute(vec![state.account], vec![]))
    }

    /// Slash a member identity: validate strike thresholds, revoke the commitment,
    /// and confiscate stake via chained call (CPI) to auth-transfer.
    ///
    /// `target_member` is the on-chain account of the member being slashed.
    /// The program validates that this account matches the target commitment,
    /// then delegates the balance transfer (confiscation) to auth-transfer.
    #[instruction]
    pub fn slash_member(
        #[account(mut, pda = [literal("forum"), arg("forum_id")])] state: AccountWithMetadata,
        #[account(signer)] authority: AccountWithMetadata,
        target_member: AccountWithMetadata,
        forum_id: [u8; 32],
        target_commitment: [u8; 32],
        k_rooms_min: u32,
        min_room_age_indexes: u64,
        min_room_members: u32,
    ) -> SpelResult {
        let mut forum: ForumInstance = borsh::from_slice(&state.account.data).map_err(|_| {
            spel_framework::error::SpelError::Custom {
                code: 6,
                message: "Deserialization error".into(),
            }
        })?;

        if forum.admin_pubkey != *authority.account_id.value() {
            return Err(spel_framework::error::SpelError::Custom {
                code: 18,
                message: "Unauthorized: only admin can execute slashing".into(),
            });
        }

        let confiscated = slash::process_slash(
            &mut forum,
            target_commitment,
            k_rooms_min,
            min_room_age_indexes,
            min_room_members,
        )
        .map_err(|e| spel_framework::error::SpelError::Custom {
            code: 7,
            message: e.into(),
        })?;

        // Confiscate stake via chained call (CPI) to auth-transfer.
        // Transfer confiscated amount from target_member to authority (admin).
        // See: https://github.com/logos-co/spel/pull/268
        let mut chained_calls = vec![];
        if confiscated > 0 {
            let auth_transfer_id: nssa_core::program::ProgramId =
                target_member.account.program_owner;

            let mut authorized_target = target_member.clone();
            authorized_target.is_authorized = true;

            let mut authorized_authority = authority.clone();
            authorized_authority.is_authorized = true;

            chained_calls.push(nssa_core::program::ChainedCall::new(
                auth_transfer_id,
                vec![authorized_target, authorized_authority],
                &authenticated_transfer_core::Instruction::Transfer {
                    amount: confiscated as u128,
                },
            ));
        }

        let mut state = state;
        state.account.data = borsh::to_vec(&forum)
            .map_err(|_| spel_framework::error::SpelError::Custom {
                code: 8,
                message: "Serialization error".into(),
            })?
            .try_into()
            .map_err(|_| spel_framework::error::SpelError::Custom {
                code: 11,
                message: "Data too large".into(),
            })?;

        // Both target_member and authority are returned unchanged —
        // the actual balance mutation is delegated to auth-transfer.
        Ok(SpelOutput::execute(
            vec![state.account, authority.account, target_member.account],
            chained_calls,
        ))
    }
}
