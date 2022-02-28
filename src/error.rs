use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Percentage needs to be between 5% and 100%")]
    WrongPercentageInput {},
    
    #[error("A Deposit Already Exists")]
    MaxPooled {},

    #[error("No Deposits to Withdraw")]
    NoDeposit {},

    #[error("No Balance to Withdraw")]
    NoBalance {},

    #[error("Only the charity address can withdraw angel pool")]
    NotAngelAddr {},

    #[error("Wrong coin input")]
    Payment(#[from] PaymentError),
}

#[derive(Error, Debug, PartialEq)]
pub enum PaymentError {
    #[error("Must send reserve token '{0}'")]
    MissingDenom(String),

    #[error("Received unsupported denom '{0}'")]
    ExtraDenom(String),

    #[error("Sent more than one denomination")]
    MultipleDenoms {},

    #[error("No funds sent")]
    NoFunds {},

    #[error("This message does no accept funds")]
    NonPayable {},
}