use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
    TransferChecked,
};

use crate::{error::EscrowError, Escrow, ESCROW_SEED};

#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut, address = escrow.maker)]
    pub maker: Signer<'info>,
    #[account(
        mut,
        close = maker,
        has_one = maker,
        has_one = mint_a,
        seeds = [ESCROW_SEED, escrow.maker.as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(
        mint::token_program = token_program
    )]
    pub mint_a: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    pub maker_ata_a: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Refund<'info> {
    pub fn refund(&mut self) -> Result<()> {
        require!(
            Clock::get()?.unix_timestamp >= self.escrow.expiration,
            EscrowError::EscrowNotExpired
        );

        let escrow_seed = self.escrow.seed.to_le_bytes();
        let bump = [self.escrow.bump];
        let signer_seeds: &[&[u8]] = &[
            ESCROW_SEED,
            self.escrow.maker.as_ref(),
            escrow_seed.as_ref(),
            bump.as_ref(),
        ];
        let signer = &[signer_seeds];
        let transfer_deposit = TransferChecked {
            from: self.vault.to_account_info(),
            mint: self.mint_a.to_account_info(),
            to: self.maker_ata_a.to_account_info(),
            authority: self.escrow.to_account_info(),
        };
        transfer_checked(
            CpiContext::new_with_signer(self.token_program.key(), transfer_deposit, signer),
            self.vault.amount,
            self.mint_a.decimals,
        )?;

        let close_vault = CloseAccount {
            account: self.vault.to_account_info(),
            destination: self.maker.to_account_info(),
            authority: self.escrow.to_account_info(),
        };
        close_account(CpiContext::new_with_signer(
            self.token_program.key(),
            close_vault,
            signer,
        ))
    }
}
