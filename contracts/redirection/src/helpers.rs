use crate::{
    error::PaymentError,
    msg::Cw20HookMsg
};
use cosmwasm_std::{to_binary, MessageInfo, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;

pub fn get_convert_to_ust(
    anchor_market_address: String,
    aust_token_address: String,
    aust_amount: u64
) -> WasmMsg {
    return WasmMsg::Execute {
        contract_addr: aust_token_address, // Should not hardcode this! Move to config.
        msg: to_binary(&Cw20ExecuteMsg::Send {
            contract: anchor_market_address, // Should not hardcode this! Move to config.
            msg: to_binary(&Cw20HookMsg::RedeemStable {}).unwrap(),
            amount: Uint128::new(aust_amount.into()),
        })
        .unwrap(),
        funds: Vec::new(),
    };
}

/// Requires exactly one native coin sent, which matches UUSD.
/// Returns the amount if only one denom and non-zero amount. Errors otherwise.
pub fn check_funds(info: &MessageInfo) -> Result<Uint128, PaymentError> {
    // check if only one coin was sent
    match info.funds.len() {
        0 => Err(PaymentError::NoFunds {}),
        1 => {
            let coin = info.funds[0].clone();
            // check that we rcv'd uusd
            if coin.denom != "uusd" {
                return Err(PaymentError::MissingDenom(coin.denom.to_string()));
            }
            // check amount is gte 0
            if coin.amount.is_zero() {
                return Err(PaymentError::NoFunds {});
            }
            Ok(coin.amount)
        }
        _ => Err(PaymentError::MultipleDenoms {}),
    }
}