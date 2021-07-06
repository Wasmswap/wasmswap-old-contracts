#![cfg(test)]

use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{
    coins, from_binary, to_binary, Addr, BalanceResponse, BankQuery, Coin, Empty, Uint128,
};
use cw20::{Cw20Coin, Cw20Contract, Cw20ExecuteMsg};
use cw_multi_test::{App, Contract, ContractWrapper, SimpleBank};

use crate::msg::{ExecuteMsg, InstantiateMsg, ReceiveMsg};

fn mock_app() -> App {
    let env = mock_env();
    let api = Box::new(MockApi::default());
    let bank = SimpleBank {};

    App::new(api, env.block, bank, || Box::new(MockStorage::new()))
}

pub fn contract_amm() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

pub fn contract_cw20() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

#[test]
// receive cw20 tokens and release upon approval
fn sale_happy_path() {
    let mut router = mock_app();

    const NATIVE_TOKEN_DENOM: &str = "token";

    let owner = Addr::unchecked("owner");

    // set up cw20 contract with some tokens
    let cw20_id = router.store_code(contract_cw20());
    let msg = cw20_base::msg::InstantiateMsg {
        name: "Cash Money".to_string(),
        symbol: "CASH".to_string(),
        decimals: 2,
        initial_balances: vec![Cw20Coin {
            address: owner.to_string(),
            amount: Uint128(5000),
        }],
        mint: None,
    };
    let cash_addr = router
        .instantiate_contract(cw20_id, owner.clone(), &msg, &[], "CASH")
        .unwrap();

    // set up sale contract
    let amm_id = router.store_code(contract_amm());
    let msg = InstantiateMsg {
        count: 1,
        nativeDenom: NATIVE_TOKEN_DENOM.to_string(),
        tokenAddress: cash_addr.clone(),
    };
    let sale_addr = router
        .instantiate_contract(amm_id, owner.clone(), &msg, &[], "amm")
        .unwrap();

    assert_ne!(cash_addr, sale_addr);

    // set up cw20 helpers
    let cash = Cw20Contract(cash_addr.clone());

    // check initial balances
    let owner_balance = cash.balance(&router, owner.clone()).unwrap();
    assert_eq!(owner_balance, Uint128(5000));

    // send tokens to contract address
    let send_msg = Cw20ExecuteMsg::Send {
        contract: sale_addr.to_string(),
        amount: Uint128::from(100u128),
        msg: Some(to_binary(&ReceiveMsg::AddLiquidity {}).unwrap()),
    };
    let res = router
        .execute_contract(owner.clone(), cash_addr.clone(), &send_msg, &[])
        .unwrap();
    println!("{:?}", res.attributes);
    assert_eq!(4, res.attributes.len());

    // ensure balances updated
    let owner_balance = cash.balance(&router, owner.clone()).unwrap();
    assert_eq!(owner_balance, Uint128(4900));
    let sale_balance = cash.balance(&router, sale_addr.clone()).unwrap();
    assert_eq!(sale_balance, Uint128(100));
}
