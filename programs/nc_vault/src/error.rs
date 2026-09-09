use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("Base and position mints must differ")]
    SameMint,
    #[msg("Deposits are only priced while the vault holds no position")]
    VaultDeployed,
    #[msg("Not enough shares")]
    InsufficientShares,
    #[msg("Vault does not hold enough liquid base to deploy that much")]
    InsufficientLiquidity,
}
