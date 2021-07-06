use cosmwasm_std::{
    entry_point, to_binary, from_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Coin, Uint128, Addr, WasmMsg, CosmosMsg, StdError
};
use cw20_base::state::{TOKEN_INFO, BALANCES};
use cw20_base::contract::{instantiate as cw20_instantiate, execute_mint,query_balance, execute_burn};
use cw20_base;
use cw20::{Cw20ExecuteMsg, MinterResponse};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{State, STATE};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        nativeSupply: Coin{denom:msg.nativeDenom, amount: Uint128(0)},
        tokenAddress: msg.tokenAddress,
        tokenSupply: Uint128(0),
    };
    STATE.save(deps.storage, &state)?;

    cw20_instantiate(deps,_env.clone(),info,cw20_base::msg::InstantiateMsg{name:"liquidity".into(),symbol:"AAAA".into(),decimals:0,initial_balances:vec![],mint:Some(MinterResponse{minter:_env.contract.address.clone().into(), cap: None})})?;

    Ok(Response::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddLiquidity {min_liqudity, max_token} => try_add_liquidity(deps, info, _env, min_liqudity, max_token),
        ExecuteMsg::RemoveLiquidity {amount, min_native, min_token} => try_remove_liquidity(deps, info, _env, amount, min_native, min_token),
    }
}

pub fn try_add_liquidity(deps: DepsMut, info: MessageInfo, _env: Env, min_liqudity: Uint128, max_token: Uint128) -> Result<Response, ContractError> {

    let state = STATE.load(deps.storage).unwrap();

    let token = TOKEN_INFO.load(deps.storage)?;

    let mint_amount = if token.total_supply == Uint128(0) {
        info.funds[0].clone().amount
    } else {
        info.funds[0].clone().amount
            .checked_mul(token.total_supply)
            .map_err(StdError::overflow)?
            .checked_div(state.nativeSupply.amount)
            .map_err(StdError::divide_by_zero)?
    };

    let token_amount= if token.total_supply == Uint128(0) {
        max_token
    } else {
        info.funds[0].clone().amount
            .checked_mul(state.tokenSupply)
            .map_err(StdError::overflow)?
            .checked_div(state.nativeSupply.amount)
            .map_err(StdError::divide_by_zero)?
            .checked_add(Uint128(1))
            .map_err(StdError::overflow)?
    };

    if mint_amount < min_liqudity {
        return Err(ContractError::MinLiquidityError{min_liquidity: min_liqudity, liquidity_available: mint_amount});
    }

    if token_amount > max_token {
        return Err(ContractError::MaxTokenError{max_token: max_token, tokens_required: token_amount});
    }

    // create transfer cw20 msg
    let transfer_cw20_msg = Cw20ExecuteMsg::TransferFrom {
        owner: info.sender.clone().into(),
        recipient: _env.contract.address.clone().into(),
        amount: token_amount,
    };
    let exec_cw20_transfer = WasmMsg::Execute {
        contract_addr: state.tokenAddress.into(),
        msg: to_binary(&transfer_cw20_msg)?,
        send: vec![],
    };
    let cw20_transfer_cosmos_msg: CosmosMsg = exec_cw20_transfer.into();

    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.tokenSupply += token_amount;
        state.nativeSupply.amount += info.funds[0].amount.clone();
        Ok(state)
    })?;

    let sub_info = MessageInfo {
        sender: _env.contract.address.clone(),
        funds: vec![],
    };
    execute_mint(deps, _env, sub_info, info.sender.clone().into(), mint_amount)?;


    Ok(Response {
        messages: vec![cw20_transfer_cosmos_msg],
        submessages: vec![],
        attributes: vec![],
        data: None,
    })
}

pub fn try_remove_liquidity(deps: DepsMut, info: MessageInfo, _env: Env, amount: Uint128, min_native: Uint128, min_token: Uint128) -> Result<Response, ContractError> {
    let balance = BALANCES.load(deps.storage, &info.sender)?;
    let token = TOKEN_INFO.load(deps.storage)?;
    let state = STATE.load(deps.storage)?;

    if amount > balance {
        return Err(ContractError::InsufficientLiquidityError{requested: amount, available: balance});
    }

    let native_amount = amount.checked_mul(state.nativeSupply.amount).map_err(StdError::overflow)?.checked_div(token.total_supply).map_err(StdError::divide_by_zero)?;
    if native_amount < min_native {
        return Err(ContractError::MinNative{requested: min_native, available: native_amount})
    }

    let token_amount = amount.checked_mul(state.tokenSupply).map_err(StdError::overflow)?.checked_div(token.total_supply).map_err(StdError::divide_by_zero)?;
    if token_amount < min_token {
        return Err(ContractError::MinNative{requested: min_token, available: token_amount})
    }

    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.tokenSupply = state.tokenSupply.checked_sub(token_amount).map_err(StdError::overflow)?;
        state.nativeSupply.amount = state.nativeSupply.amount.checked_sub(native_amount).map_err(StdError::overflow)?; 
        Ok(state)
    })?;


    let transfer_bank_msg = cosmwasm_std::BankMsg::Send {
        to_address: info.sender.clone().into(),
        amount: vec!(Coin{denom:state.nativeSupply.denom,amount:native_amount}),
    };

    let transfer_bank_cosmos_msg: CosmosMsg = transfer_bank_msg.into();

      // create transfer cw20 msg
    let transfer_cw20_msg = Cw20ExecuteMsg::Transfer {
        recipient: info.sender.clone().into(),
        amount: token_amount,
    };
    let exec_cw20_transfer = WasmMsg::Execute {
        contract_addr: state.tokenAddress.into(),
        msg: to_binary(&transfer_cw20_msg)?,
        send: vec![],
    };
    let cw20_transfer_cosmos_msg: CosmosMsg = exec_cw20_transfer.into();

    execute_burn(deps, _env, info, amount)?;


    Ok(Response {
        messages: vec![transfer_bank_cosmos_msg, cw20_transfer_cosmos_msg],
        submessages: vec![],
        attributes: vec![],
        data: None,
    })
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance {address} => to_binary(&query_balance(deps, address)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg { nativeDenom: "test".to_string(), tokenAddress: Addr::unchecked("asdf")};
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn add_liqudity() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let msg = InstantiateMsg { nativeDenom: "test".to_string(), tokenAddress: Addr::unchecked("asdf")};
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("anyone", &coins(2, "token"));
        let msg = ExecuteMsg::AddLiquidity {min_liqudity: Uint128(1), max_token: Uint128(1) };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }
}
