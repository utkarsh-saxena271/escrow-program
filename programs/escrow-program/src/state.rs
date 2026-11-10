use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account(discriminator = 1)]
pub struct Escrow {
    pub seed: u64, // seed for the pda escrow
    pub maker : Pubkey, //pubkey of authority
    pub mint_a: Pubkey, // pubkey of token x
    pub mint_b: Pubkey, // pubkey of token b
    pub receive: u64, // amount of lamports
    pub bump : u8, // bump number for escrow pda
    pub expiration: i64 // expiration time of escrow
}
