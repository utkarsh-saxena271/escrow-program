use anchor_lang::{
    prelude::{Clock, Pubkey},
    solana_program::{instruction::Instruction, program_option::COption, program_pack::Pack},
    AccountDeserialize, InstructionData, ToAccountMetas,
};
use anchor_spl::{
    associated_token::{get_associated_token_address, ID as ASSOCIATED_TOKEN_PROGRAM_ID},
    token::spl_token::{
        state::{Account as SplAccount, AccountState, Mint},
        ID as TOKEN_PROGRAM_ID,
    },
};
use litesvm::LiteSVM;
use solana_account::Account;
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

const INITIAL_TIME: i64 = 100;
const DEPOSIT: u64 = 50;
const REQUESTED: u64 = 80;
const EXPIRATION: i64 = 200;

struct TestContext {
    svm: LiteSVM,
    maker: Keypair,
    taker: Keypair,
    attacker: Keypair,
    mint_a: Pubkey,
    mint_b: Pubkey,
    maker_ata_a: Pubkey,
    maker_ata_b: Pubkey,
    taker_ata_a: Pubkey,
    taker_ata_b: Pubkey,
    attacker_ata_a: Pubkey,
    attacker_ata_b: Pubkey,
}

impl TestContext {
    fn new() -> Self {
        let program_id = escrow_program::id();
        let mut svm = LiteSVM::new();
        let program = include_bytes!(concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/../deploy/escrow_program.so"
        ));
        svm.add_program(program_id, program).unwrap();

        let maker = Keypair::new();
        let taker = Keypair::new();
        let attacker = Keypair::new();
        for key in [&maker, &taker, &attacker] {
            svm.airdrop(&key.pubkey(), 1_000_000_000).unwrap();
        }

        let mint_a = Pubkey::new_unique();
        let mint_b = Pubkey::new_unique();
        set_mint(&mut svm, mint_a);
        set_mint(&mut svm, mint_b);

        let maker_ata_a = add_token_account(&mut svm, &maker.pubkey(), &mint_a, 100);
        let maker_ata_b = add_token_account(&mut svm, &maker.pubkey(), &mint_b, 0);
        let taker_ata_a = add_token_account(&mut svm, &taker.pubkey(), &mint_a, 0);
        let taker_ata_b = add_token_account(&mut svm, &taker.pubkey(), &mint_b, 100);
        let attacker_ata_a = add_token_account(&mut svm, &attacker.pubkey(), &mint_a, 0);
        let attacker_ata_b = add_token_account(&mut svm, &attacker.pubkey(), &mint_b, 100);

        let mut clock: Clock = svm.get_sysvar();
        clock.unix_timestamp = INITIAL_TIME;
        svm.set_sysvar(&clock);

        Self {
            svm,
            maker,
            taker,
            attacker,
            mint_a,
            mint_b,
            maker_ata_a,
            maker_ata_b,
            taker_ata_a,
            taker_ata_b,
            attacker_ata_a,
            attacker_ata_b,
        }
    }

    fn escrow(&self, seed: u64) -> Pubkey {
        Pubkey::find_program_address(
            &[
                escrow_program::constants::ESCROW_SEED,
                self.maker.pubkey().as_ref(),
                seed.to_le_bytes().as_ref(),
            ],
            &escrow_program::id(),
        )
        .0
    }

    fn vault(&self, seed: u64) -> Pubkey {
        get_associated_token_address(&self.escrow(seed), &self.mint_a)
    }

    fn make_ix(&self, seed: u64) -> Instruction {
        self.make_with(
            seed,
            self.taker.pubkey(),
            self.mint_b,
            DEPOSIT,
            REQUESTED,
            EXPIRATION,
        )
    }

    fn make_with(
        &self,
        seed: u64,
        taker: Pubkey,
        mint_b: Pubkey,
        deposit: u64,
        receive: u64,
        expiration: i64,
    ) -> Instruction {
        Instruction::new_with_bytes(
            escrow_program::id(),
            &escrow_program::instruction::Make {
                seed,
                deposit,
                receive,
                expiration,
            }
            .data(),
            escrow_program::accounts::Make {
                maker: self.maker.pubkey(),
                taker,
                mint_a: self.mint_a,
                mint_b,
                maker_ata_a: self.maker_ata_a,
                escrow: self.escrow(seed),
                vault: self.vault(seed),
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: anchor_lang::system_program::ID,
            }
            .to_account_metas(None),
        )
    }

    fn take_ix(
        &self,
        seed: u64,
        taker: Pubkey,
        taker_ata_a: Pubkey,
        taker_ata_b: Pubkey,
    ) -> Instruction {
        Instruction::new_with_bytes(
            escrow_program::id(),
            &escrow_program::instruction::Take {}.data(),
            escrow_program::accounts::Take {
                maker: self.maker.pubkey(),
                taker,
                escrow: self.escrow(seed),
                mint_a: self.mint_a,
                mint_b: self.mint_b,
                vault: self.vault(seed),
                taker_ata_a,
                taker_ata_b,
                maker_ata_b: self.maker_ata_b,
                token_program: TOKEN_PROGRAM_ID,
            }
            .to_account_metas(None),
        )
    }

    fn refund_ix(&self, seed: u64) -> Instruction {
        self.refund_by(seed, self.maker.pubkey(), self.maker_ata_a)
    }

    fn refund_by(&self, seed: u64, signer: Pubkey, signer_ata_a: Pubkey) -> Instruction {
        Instruction::new_with_bytes(
            escrow_program::id(),
            &escrow_program::instruction::Refund {}.data(),
            escrow_program::accounts::Refund {
                maker: signer,
                escrow: self.escrow(seed),
                mint_a: self.mint_a,
                vault: self.vault(seed),
                maker_ata_a: signer_ata_a,
                token_program: TOKEN_PROGRAM_ID,
            }
            .to_account_metas(None),
        )
    }

    fn update_ix(&self, seed: u64, receive: u64, expiration: i64) -> Instruction {
        Instruction::new_with_bytes(
            escrow_program::id(),
            &escrow_program::instruction::Update {
                seed,
                receive,
                expiration,
            }
            .data(),
            escrow_program::accounts::Update {
                maker: self.maker.pubkey(),
                escrow: self.escrow(seed),
            }
            .to_account_metas(None),
        )
    }

    fn set_time(&mut self, unix_timestamp: i64) {
        let mut clock: Clock = self.svm.get_sysvar();
        clock.unix_timestamp = unix_timestamp;
        self.svm.set_sysvar(&clock);
    }

    fn token_balance(&self, address: Pubkey) -> u64 {
        let account = self.svm.get_account(&address).unwrap();
        SplAccount::unpack(&account.data).unwrap().amount
    }
}

fn send(
    svm: &mut LiteSVM,
    payer: &Keypair,
    signers: &[&Keypair],
    ix: Instruction,
) -> Result<(), String> {
    svm.expire_blockhash();
    let blockhash = svm.latest_blockhash();
    let message = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let transaction =
        VersionedTransaction::try_new(VersionedMessage::Legacy(message), signers).unwrap();
    svm.send_transaction(transaction)
        .map(|_| ())
        .map_err(|failure| format!("{:?}\n{}", failure.err, failure.meta.logs.join("\n")))
}

fn set_mint(svm: &mut LiteSVM, address: Pubkey) {
    let mut data = vec![0; Mint::LEN];
    Mint::pack(
        Mint {
            mint_authority: COption::None,
            supply: 300,
            decimals: 0,
            is_initialized: true,
            freeze_authority: COption::None,
        },
        &mut data,
    )
    .unwrap();
    svm.set_account(
        address,
        Account {
            lamports: 1_000_000,
            data,
            owner: TOKEN_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();
}

fn add_token_account(svm: &mut LiteSVM, authority: &Pubkey, mint: &Pubkey, amount: u64) -> Pubkey {
    let address = get_associated_token_address(authority, mint);
    let mut data = vec![0; SplAccount::LEN];
    SplAccount::pack(
        SplAccount {
            mint: *mint,
            owner: *authority,
            amount,
            delegate: COption::None,
            state: AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        },
        &mut data,
    )
    .unwrap();
    svm.set_account(
        address,
        Account {
            lamports: 1_000_000,
            data,
            owner: TOKEN_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();
    address
}

fn assert_escrow_state(context: &TestContext, seed: u64, receive: u64, expiration: i64) {
    let account = context.svm.get_account(&context.escrow(seed)).unwrap();
    let mut data: &[u8] = &account.data;
    let escrow = escrow_program::state::Escrow::try_deserialize(&mut data).unwrap();
    assert_eq!(escrow.maker, context.maker.pubkey());
    assert_eq!(escrow.taker, context.taker.pubkey());
    assert_eq!(escrow.deposit, DEPOSIT);
    assert_eq!(escrow.receive, receive);
    assert_eq!(escrow.expiration, expiration);
}

#[test]
fn make_and_update_escrow() {
    let mut context = TestContext::new();
    let seed = 7;

    let make = context.make_ix(seed);
    send(&mut context.svm, &context.maker, &[&context.maker], make).unwrap();
    assert_eq!(context.token_balance(context.maker_ata_a), 100 - DEPOSIT);
    assert_eq!(context.token_balance(context.vault(seed)), DEPOSIT);
    assert_escrow_state(&context, seed, REQUESTED, EXPIRATION);

    let update = context.update_ix(seed, 90, 250);
    send(&mut context.svm, &context.maker, &[&context.maker], update).unwrap();
    assert_escrow_state(&context, seed, 90, 250);

    let unauthorized = Instruction::new_with_bytes(
        escrow_program::id(),
        &escrow_program::instruction::Update {
            seed,
            receive: 1,
            expiration: 300,
        }
        .data(),
        escrow_program::accounts::Update {
            maker: context.attacker.pubkey(),
            escrow: context.escrow(seed),
        }
        .to_account_metas(None),
    );
    assert!(send(
        &mut context.svm,
        &context.attacker,
        &[&context.attacker],
        unauthorized
    )
    .is_err());
    assert_escrow_state(&context, seed, 90, 250);
}

#[test]
fn only_designated_taker_can_take_before_expiration() {
    let mut context = TestContext::new();
    let seed = 8;
    let make = context.make_ix(seed);
    send(&mut context.svm, &context.maker, &[&context.maker], make).unwrap();

    let unauthorized = context.take_ix(
        seed,
        context.attacker.pubkey(),
        context.attacker_ata_a,
        context.attacker_ata_b,
    );
    assert!(send(
        &mut context.svm,
        &context.attacker,
        &[&context.attacker],
        unauthorized
    )
    .is_err());
    assert_eq!(context.token_balance(context.vault(seed)), DEPOSIT);

    context.set_time(EXPIRATION - 1);
    let take = context.take_ix(
        seed,
        context.taker.pubkey(),
        context.taker_ata_a,
        context.taker_ata_b,
    );
    send(&mut context.svm, &context.taker, &[&context.taker], take).unwrap();
    assert_eq!(context.token_balance(context.taker_ata_a), DEPOSIT);
    assert_eq!(context.token_balance(context.taker_ata_b), 100 - REQUESTED);
    assert_eq!(context.token_balance(context.maker_ata_b), REQUESTED);
    assert!(context.svm.get_account(&context.escrow(seed)).is_none());
    assert!(context.svm.get_account(&context.vault(seed)).is_none());
}

#[test]
fn maker_can_refund_only_at_or_after_expiration() {
    let mut context = TestContext::new();
    let seed = 9;
    let make = context.make_ix(seed);
    send(&mut context.svm, &context.maker, &[&context.maker], make).unwrap();

    let refund = context.refund_ix(seed);
    assert!(send(&mut context.svm, &context.maker, &[&context.maker], refund).is_err());
    assert_eq!(context.token_balance(context.vault(seed)), DEPOSIT);

    context.set_time(EXPIRATION);
    let late_take = context.take_ix(
        seed,
        context.taker.pubkey(),
        context.taker_ata_a,
        context.taker_ata_b,
    );
    assert!(send(
        &mut context.svm,
        &context.taker,
        &[&context.taker],
        late_take
    )
    .is_err());
    assert_eq!(context.token_balance(context.vault(seed)), DEPOSIT);

    let refund = context.refund_ix(seed);
    send(&mut context.svm, &context.maker, &[&context.maker], refund).unwrap();
    assert_eq!(context.token_balance(context.maker_ata_a), 100);
    assert!(context.svm.get_account(&context.escrow(seed)).is_none());
    assert!(context.svm.get_account(&context.vault(seed)).is_none());
}

// ---------- make: validation failures ----------

#[test]
fn make_rejects_same_mint() {
    let mut context = TestContext::new();
    let seed = 10;
    let ix = context.make_with(
        seed,
        context.taker.pubkey(),
        context.mint_a, // mint_b == mint_a
        DEPOSIT,
        REQUESTED,
        EXPIRATION,
    );
    assert!(send(&mut context.svm, &context.maker, &[&context.maker], ix).is_err());
    assert!(context.svm.get_account(&context.escrow(seed)).is_none());
    assert_eq!(context.token_balance(context.maker_ata_a), 100);
}

#[test]
fn make_rejects_expiration_in_the_past() {
    let mut context = TestContext::new();
    let seed = 11;

    for expiration in [INITIAL_TIME, INITIAL_TIME - 1] {
        let ix = context.make_with(
            seed,
            context.taker.pubkey(),
            context.mint_b,
            DEPOSIT,
            REQUESTED,
            expiration,
        );
        assert!(send(&mut context.svm, &context.maker, &[&context.maker], ix).is_err());
    }
    assert!(context.svm.get_account(&context.escrow(seed)).is_none());
    assert_eq!(context.token_balance(context.maker_ata_a), 100);
}

#[test]
fn make_rejects_zero_amounts() {
    let mut context = TestContext::new();
    let seed = 12;

    let zero_deposit = context.make_with(
        seed,
        context.taker.pubkey(),
        context.mint_b,
        0,
        REQUESTED,
        EXPIRATION,
    );
    assert!(send(
        &mut context.svm,
        &context.maker,
        &[&context.maker],
        zero_deposit
    )
    .is_err());

    let zero_receive = context.make_with(
        seed,
        context.taker.pubkey(),
        context.mint_b,
        DEPOSIT,
        0,
        EXPIRATION,
    );
    assert!(send(
        &mut context.svm,
        &context.maker,
        &[&context.maker],
        zero_receive
    )
    .is_err());

    assert!(context.svm.get_account(&context.escrow(seed)).is_none());
    assert_eq!(context.token_balance(context.maker_ata_a), 100);
}

#[test]
fn make_rejects_maker_as_taker() {
    let mut context = TestContext::new();
    let seed = 13;
    let ix = context.make_with(
        seed,
        context.maker.pubkey(), // taker == maker
        context.mint_b,
        DEPOSIT,
        REQUESTED,
        EXPIRATION,
    );
    assert!(send(&mut context.svm, &context.maker, &[&context.maker], ix).is_err());
    assert!(context.svm.get_account(&context.escrow(seed)).is_none());
    assert_eq!(context.token_balance(context.maker_ata_a), 100);
}

// ---------- update: failures ----------

#[test]
fn update_fails_after_expiry() {
    let mut context = TestContext::new();
    let seed = 14;
    let make = context.make_ix(seed);
    send(&mut context.svm, &context.maker, &[&context.maker], make).unwrap();

    context.set_time(EXPIRATION);
    let update = context.update_ix(seed, 90, 300);
    assert!(send(&mut context.svm, &context.maker, &[&context.maker], update).is_err());
    assert_escrow_state(&context, seed, REQUESTED, EXPIRATION);
}

#[test]
fn update_rejects_zero_receive_and_past_expiration() {
    let mut context = TestContext::new();
    let seed = 15;
    let make = context.make_ix(seed);
    send(&mut context.svm, &context.maker, &[&context.maker], make).unwrap();

    let zero_receive = context.update_ix(seed, 0, 300);
    assert!(send(
        &mut context.svm,
        &context.maker,
        &[&context.maker],
        zero_receive
    )
    .is_err());

    let past_expiration = context.update_ix(seed, 90, INITIAL_TIME);
    assert!(send(
        &mut context.svm,
        &context.maker,
        &[&context.maker],
        past_expiration
    )
    .is_err());

    assert_escrow_state(&context, seed, REQUESTED, EXPIRATION);
}

// ---------- refund: failure ----------

#[test]
fn refund_by_non_maker_fails() {
    let mut context = TestContext::new();
    let seed = 16;
    let make = context.make_ix(seed);
    send(&mut context.svm, &context.maker, &[&context.maker], make).unwrap();

    // past expiry, so the only reason to fail is the wrong signer
    context.set_time(EXPIRATION);
    let ix = context.refund_by(seed, context.attacker.pubkey(), context.attacker_ata_a);
    assert!(send(&mut context.svm, &context.attacker, &[&context.attacker], ix).is_err());

    assert_eq!(context.token_balance(context.vault(seed)), DEPOSIT);
    assert_eq!(context.token_balance(context.attacker_ata_a), 0);
    assert!(context.svm.get_account(&context.escrow(seed)).is_some());
}

// ---------- take: failure ----------

#[test]
fn take_fails_when_taker_cannot_pay() {
    let mut context = TestContext::new();
    let seed = 17;
    // taker only holds 100 of mint B, so ask for more than that
    let make = context.make_with(
        seed,
        context.taker.pubkey(),
        context.mint_b,
        DEPOSIT,
        150,
        EXPIRATION,
    );
    send(&mut context.svm, &context.maker, &[&context.maker], make).unwrap();

    let take = context.take_ix(
        seed,
        context.taker.pubkey(),
        context.taker_ata_a,
        context.taker_ata_b,
    );
    assert!(send(&mut context.svm, &context.taker, &[&context.taker], take).is_err());

    assert_eq!(context.token_balance(context.vault(seed)), DEPOSIT);
    assert_eq!(context.token_balance(context.taker_ata_a), 0);
    assert_eq!(context.token_balance(context.taker_ata_b), 100);
    assert_eq!(context.token_balance(context.maker_ata_b), 0);
    assert!(context.svm.get_account(&context.escrow(seed)).is_some());
}