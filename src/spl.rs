use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

pub fn transfer_instruction_with_program_id(
    source_address: &Pubkey,
    destination_address: &Pubkey,
    authority_address: &Pubkey,
    amount: u64,
    token_program_id: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: *token_program_id,
        accounts: vec![
            AccountMeta::new(*source_address, false),
            AccountMeta::new(*destination_address, false),
            AccountMeta::new_readonly(*authority_address, true),
        ],
        data: [vec![3], amount.to_le_bytes().to_vec()].concat(),
    }
}