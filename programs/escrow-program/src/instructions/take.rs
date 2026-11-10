use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
    TransferChecked,
};

use crate::{error::EscrowError, Escrow, ESCROW_SEED};

#[derive(Accounts)]
pub struct Take<'info> {
    #[account(mut, address = escrow.maker)]
    /// CHECK: The address is constrained to the maker saved in the escrow.
    pub maker: UncheckedAccount<'info>,
    #[account(mut, address = escrow.taker)]
    pub taker: Signer<'info>,
    #[account(
        mut,
        close = maker,
        has_one = maker,
        has_one = taker,
        has_one = mint_a,
        has_one = mint_b,
        seeds = [ESCROW_SEED, escrow.maker.as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump
    )]
    pub escrow: Box<Account<'info, Escrow>>,
    #[account(
        mint::token_program = token_program
    )]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mint::token_program = token_program
    )]
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = taker,
        associated_token::token_program = token_program
    )]
    pub taker_ata_a: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = taker,
        associated_token::token_program = token_program
    )]
    pub taker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    pub maker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Take<'info> {
    pub fn take(&mut self) -> Result<()> {
        require!(
            Clock::get()?.unix_timestamp < self.escrow.expiration,
            EscrowError::EscrowExpired
        );
        let token_program = self.token_program.key();

        let transfer_payment = TransferChecked {
            from: self.taker_ata_b.to_account_info(),
            mint: self.mint_b.to_account_info(),
            to: self.maker_ata_b.to_account_info(),
            authority: self.taker.to_account_info(),
        };
        transfer_checked(
            CpiContext::new(token_program, transfer_payment),
            self.escrow.receive,
            self.mint_b.decimals,
        )?;

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
            to: self.taker_ata_a.to_account_info(),
            authority: self.escrow.to_account_info(),
        };
        transfer_checked(
            CpiContext::new_with_signer(token_program, transfer_deposit, signer),
            self.vault.amount,
            self.mint_a.decimals,
        )?;

        let close_vault = CloseAccount {
            account: self.vault.to_account_info(),
            destination: self.maker.to_account_info(),
            authority: self.escrow.to_account_info(),
        };
        close_account(CpiContext::new_with_signer(
            token_program,
            close_vault,
            signer,
        ))
    }
}
