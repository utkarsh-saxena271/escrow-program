pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("BfJ9RytuctpknaRHwCbMQXwZ8WtMRdasPJSzHkKzzu7U");

#[program]
pub mod escrow_program {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn make(
        ctx: Context<Make>,
        seed: u64,
        deposit: u64,
        receive: u64,
        expiration: i64,
    ) -> Result<()> {
        ctx.accounts
            .make(seed, deposit, receive, expiration, &ctx.bumps)
    }

    pub fn take(ctx: Context<Take>) -> Result<()> {
        ctx.accounts.take()
    }

    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        ctx.accounts.refund()
    }

    pub fn update(ctx: Context<Update>, seed: u64, receive: u64, expiration: i64) -> Result<()> {
        ctx.accounts.update(seed, receive, expiration)
    }
}
