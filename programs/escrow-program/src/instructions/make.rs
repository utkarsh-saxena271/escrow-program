use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::{error::EscrowError, Escrow, ESCROW_SEED};

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Make<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,
    /// The only account authorized to accept this escrow.
    /// CHECK: This account is stored as the designated taker.
    pub taker: UncheckedAccount<'info>,
    #[account(
        mint::token_program = token_program
    )]
    pub mint_a: InterfaceAccount<'info, Mint>,
    #[account(
        mint::token_program = token_program
    )]
    pub mint_b: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    pub maker_ata_a: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init,
        payer = maker,
        seeds = [ESCROW_SEED, maker.key().as_ref(), seed.to_le_bytes().as_ref()],
        space = Escrow::DISCRIMINATOR.len() + Escrow::INIT_SPACE,
        bump
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(
        init,
        payer = maker,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Make<'info> {
    pub fn make(
        &mut self,
        seed: u64,
        deposit: u64,
        receive: u64,
        expiration: i64,
        bumps: &MakeBumps,
    ) -> Result<()> {
        require!(
            self.maker.key() != self.taker.key(),
            EscrowError::MakerCannotBeTaker
        );
        require!(
            self.mint_a.key() != self.mint_b.key(),
            EscrowError::SameMint
        );
        require!(deposit > 0 && receive > 0, EscrowError::InvalidAmount);
        require!(
            expiration > Clock::get()?.unix_timestamp,
            EscrowError::InvalidExpiration
        );

        self.escrow.set_inner(Escrow {
            seed,
            maker: self.maker.key(),
            taker: self.taker.key(),
            mint_a: self.mint_a.key(),
            mint_b: self.mint_b.key(),
            deposit,
            receive,
            bump: bumps.escrow,
            expiration,
        });

        let transfer_accounts = TransferChecked {
            from: self.maker_ata_a.to_account_info(),
            mint: self.mint_a.to_account_info(),
            to: self.vault.to_account_info(),
            authority: self.maker.to_account_info(),
        };
        transfer_checked(
            CpiContext::new(self.token_program.key(), transfer_accounts),
            deposit,
            self.mint_a.decimals,
        )
    }
}
