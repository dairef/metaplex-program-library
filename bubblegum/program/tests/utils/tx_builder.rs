use anchor_lang::{self, InstructionData, ToAccountMetas};
use solana_program::pubkey::Pubkey;
use solana_program_test::BanksClient;
use solana_sdk::{instruction::AccountMeta, signature::Keypair, transaction::Transaction};

use std::cell::{RefCell, RefMut};

use super::{clone_keypair, instruction, tree::Tree, Error, Result};

// Helper object to execute and easily alter characteristics of transactions
// which contain a Bubblegum instruction. There's one instantiation for each
// particular operation (when T and U become concrete types), which are
// aliased a bit further below for convenience. The point of these objects
// is to be easy to set up for the common case of each operation, but at the
// same time easy to tweak via the pub fields and methods (including ones that
// can be added) such that it's easy to validate various test cases.
pub struct TxBuilder<'a, T, U, V, const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> {
    // This is the accounts structure that holds all the pubkeys, and for
    // each particular op we'll use the one generated by Anchor.
    pub accounts: T,
    pub additional_accounts: Vec<AccountMeta>,
    // Similar to the above, but for instruction data.
    pub data: U,
    // The currently configured payer for the tx.
    pub payer: Pubkey,
    // Using `RefCell` to provide interior mutability and circumvent some
    // annoyance with the borrow checker (i.e. provide helper methods that
    // only need &self, vs &mut self); if we'll ever need to use this
    // in a context with multiple threads, we can just replace the wrapper
    // with a `Mutex`.
    pub client: RefCell<BanksClient>,
    // Currently configured signers for the tx. Using only `Keypair`s as
    // signers for now; can make this more generic if needed.
    pub signers: Vec<Keypair>,
    pub tree: &'a Tree<MAX_DEPTH, MAX_BUFFER_SIZE>,
    // This member holds data of a custom type that can be specific to each kind
    // of transaction.
    pub inner: V,
}

pub trait OnSuccessfulTxExec {
    fn on_successful_execute(&mut self);
}

impl<'a, T, U, V, const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize>
    TxBuilder<'a, T, U, V, MAX_DEPTH, MAX_BUFFER_SIZE>
where
    T: ToAccountMetas,
    U: InstructionData,
{
    fn client(&self) -> RefMut<BanksClient> {
        self.client.borrow_mut()
    }

    pub async fn execute(&mut self) -> Result<()>
    where
        Self: OnSuccessfulTxExec,
    {
        let recent_blockhash = self
            .client()
            .get_latest_blockhash()
            .await
            .map_err(Error::BanksClient)?;

        let mut ix = instruction(&self.accounts, &self.data);

        // Add the additional accounts metas (if any) as well.
        ix.accounts.append(&mut self.additional_accounts.clone());

        let mut tx = Transaction::new_with_payer(&[ix], Some(&self.payer));

        // Using `try_partial_sign` to avoid panics (and get an error when something is
        // wrong instead) no matter what signers are configured.
        tx.try_partial_sign(&self.signers.iter().collect::<Vec<_>>(), recent_blockhash)
            .map_err(Error::Signer)?;

        self.client()
            .process_transaction(tx)
            .await
            .map_err(Error::BanksClient)?;

        self.on_successful_execute();

        Ok(())
    }

    // Returning `&mut Self` to allow method chaining.
    pub fn set_signers(&mut self, signers: &[&Keypair]) -> &mut Self {
        self.signers = signers.iter().map(|k| clone_keypair(k)).collect();
        self
    }

    pub fn set_payer(&mut self, key: Pubkey) -> &mut Self {
        self.payer = key;
        self
    }

    pub fn set_additional_account_metas(&mut self, metas: &[AccountMeta]) -> &mut Self {
        self.additional_accounts = metas.iter().cloned().collect();
        self
    }

    // Populate the `additional_account` member with read-only and non-signer accounts based
    // on the provided public keys.
    pub fn set_additional_accounts(&mut self, keys: &[Pubkey]) -> &mut Self {
        self.additional_accounts = keys
            .iter()
            .map(|key| AccountMeta::new_readonly(*key, false))
            .collect();
        self
    }

    pub fn set_additional_accounts_from_arrays(&mut self, keys: &[[u8; 32]]) -> &mut Self {
        self.set_additional_accounts(
            keys.iter()
                .copied()
                .map(Pubkey::new_from_array)
                .collect::<Vec<_>>()
                .as_slice(),
        )
    }
}

// TODO: Temporary implementation so things still build properly.
// Will be removed in a following commit.
impl<'a, T, U, V, const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> OnSuccessfulTxExec
    for TxBuilder<'a, T, U, V, MAX_DEPTH, MAX_BUFFER_SIZE>
{
    fn on_successful_execute(&mut self) {
        // Do nothing.
    }
}

// The types below have "builder" in their names because we're essentially
// implementing a lightweight builder patter to instantiate, customize, and
// execute transactions.
pub type CreateBuilder<'a, const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> = TxBuilder<
    'a,
    mpl_bubblegum::accounts::CreateTree,
    mpl_bubblegum::instruction::CreateTree,
    (),
    MAX_DEPTH,
    MAX_BUFFER_SIZE,
>;

pub type MintV1Builder<'a, const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> = TxBuilder<
    'a,
    mpl_bubblegum::accounts::MintV1,
    mpl_bubblegum::instruction::MintV1,
    (),
    MAX_DEPTH,
    MAX_BUFFER_SIZE,
>;

pub type BurnBuilder<'a, const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> = TxBuilder<
    'a,
    mpl_bubblegum::accounts::Burn,
    mpl_bubblegum::instruction::Burn,
    (),
    MAX_DEPTH,
    MAX_BUFFER_SIZE,
>;

pub type TransferBuilder<'a, const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> = TxBuilder<
    'a,
    mpl_bubblegum::accounts::Transfer,
    mpl_bubblegum::instruction::Transfer,
    (),
    MAX_DEPTH,
    MAX_BUFFER_SIZE,
>;

pub type DelegateBuilder<'a, const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> = TxBuilder<
    'a,
    mpl_bubblegum::accounts::Delegate,
    mpl_bubblegum::instruction::Delegate,
    (),
    MAX_DEPTH,
    MAX_BUFFER_SIZE,
>;

pub type SetTreeDelegateBuilder<'a, const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> =
    TxBuilder<
        'a,
        mpl_bubblegum::accounts::SetTreeDelegate,
        mpl_bubblegum::instruction::SetTreeDelegate,
        (),
        MAX_DEPTH,
        MAX_BUFFER_SIZE,
    >;

pub type VerifyCreatorBuilder<'a, const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> = TxBuilder<
    'a,
    mpl_bubblegum::accounts::CreatorVerification,
    mpl_bubblegum::instruction::VerifyCreator,
    (),
    MAX_DEPTH,
    MAX_BUFFER_SIZE,
>;

pub type UnverifyCreatorBuilder<'a, const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> =
    TxBuilder<
        'a,
        mpl_bubblegum::accounts::CreatorVerification,
        mpl_bubblegum::instruction::UnverifyCreator,
        (),
        MAX_DEPTH,
        MAX_BUFFER_SIZE,
    >;