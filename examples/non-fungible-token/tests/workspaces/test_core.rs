use crate::utils::{init, helper_mint, TOKEN_ID};
use near_contract_standards::non_fungible_token::Token;
use near_primitives::views::FinalExecutionStatus;
use near_sdk::json_types::U128;
use near_sdk::ONE_YOCTO;

#[tokio::test]
async fn simulate_simple_transfer() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let (nft_contract, alice, _, _) = init(&worker).await?;

    let token = nft_contract
        .call(&worker, "nft_token")
        .args_json((TOKEN_ID,))?
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), nft_contract.id().to_string());

    let res = nft_contract
        .call(&worker, "nft_transfer")
        .args_json((
            alice.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("simple transfer".to_string()),
        ))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    // TODO: Check the logs once workspaces starts exposing them
    // Prints no logs
    // assert_eq!(res.logs().len(), 0);

    let token = nft_contract
        .call(&worker, "nft_token")
        .args_json((TOKEN_ID,))?
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), alice.id().to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_fast_return_to_sender() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let (nft_contract, _, token_receiver_contract, _) = init(&worker).await?;

    let res = nft_contract
        .call(&worker, "nft_transfer_call")
        .args_json((
            token_receiver_contract.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("transfer & call"),
            "return-it-now",
        ))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    let token = nft_contract
        .call(&worker, "nft_token")
        .args_json((TOKEN_ID,))?
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), nft_contract.id().to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_slow_return_to_sender() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let (nft_contract, _, token_receiver_contract, _) = init(&worker).await?;

    let res = nft_contract
        .call(&worker, "nft_transfer_call")
        .args_json((
            token_receiver_contract.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("transfer & call"),
            "return-it-later",
        ))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    let token = nft_contract
        .call(&worker, "nft_token")
        .args_json((TOKEN_ID,))?
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), nft_contract.id().to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_fast_keep_with_sender() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let (nft_contract, _, token_receiver_contract, _) = init(&worker).await?;

    let res = nft_contract
        .call(&worker, "nft_transfer_call")
        .args_json((
            token_receiver_contract.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("transfer & call"),
            "keep-it-now",
        ))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    // TODO: Check the logs once workspaces starts exposing them
    // Prints no logs
    // assert_eq!(res.logs().len(), 0);

    let token = nft_contract
        .call(&worker, "nft_token")
        .args_json((TOKEN_ID,))?
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), token_receiver_contract.id().to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_slow_keep_with_sender() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let (nft_contract, _, token_receiver_contract, _) = init(&worker).await?;

    let res = nft_contract
        .call(&worker, "nft_transfer_call")
        .args_json((
            token_receiver_contract.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("transfer & call"),
            "keep-it-later",
        ))?
        .gas(300_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    let token = nft_contract
        .call(&worker, "nft_token")
        .args_json((TOKEN_ID,))?
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), token_receiver_contract.id().to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_receiver_panics() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let (nft_contract, _, token_receiver_contract, _) = init(&worker).await?;

    let res = nft_contract
        .call(&worker, "nft_transfer_call")
        .args_json((
            token_receiver_contract.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("transfer & call"),
            "incorrect message",
        ))?
        .gas(35_000_000_000_000 + 1)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::SuccessValue(_)));

    // TODO: Check the logs once workspaces starts exposing them
    // Prints final log
    // assert_eq!(res.logs().len(), 1);

    let token = nft_contract
        .call(&worker, "nft_token")
        .args_json((TOKEN_ID,))?
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), nft_contract.id().to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_transfer_call_receiver_panics_and_nft_resolve_transfer_produces_no_log_if_not_enough_gas(
) -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let (nft_contract, _, token_receiver_contract, _) = init(&worker).await?;

    let res = nft_contract
        .call(&worker, "nft_transfer_call")
        .args_json((
            token_receiver_contract.id(),
            TOKEN_ID,
            Option::<u64>::None,
            Some("transfer & call"),
            "incorrect message",
        ))?
        .gas(30_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::Failure(_)));

    // TODO: Check the logs once workspaces starts exposing them
    // Prints no logs
    // assert_eq!(res.logs().len(), 0);

    let token = nft_contract
        .call(&worker, "nft_token")
        .args_json((TOKEN_ID,))?
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), nft_contract.id().to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_simple_transfer_no_logs_on_failure() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let (nft_contract, _, _, _) = init(&worker).await?;

    let res = nft_contract
        .call(&worker, "nft_transfer")
        // transfer to the current owner should fail and not print log
        .args_json((nft_contract.id(), TOKEN_ID, Option::<u64>::None, Some("simple transfer")))?
        .gas(200_000_000_000_000)
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(matches!(res.status, FinalExecutionStatus::Failure(_)));

    // TODO: Check the logs once workspaces starts exposing them
    // Prints no logs
    // assert_eq!(res.logs().len(), 0);

    let token = nft_contract
        .call(&worker, "nft_token")
        .args_json((TOKEN_ID,))?
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), nft_contract.id().to_string());

    Ok(())
}

#[tokio::test]
async fn simulate_mass_mint() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let (nft_contract, _, _, _) = init(&worker).await?;

    pub const TOTAL_NFT_TO_MINT: u128 = 750;

    for n in 1..TOTAL_NFT_TO_MINT {
        helper_mint(
            &nft_contract,
            &worker,
            n.to_string(),
            "Name".to_string(),
            "Description".to_string(),
        ).await?;
    }

    let token = nft_contract
        .call(&worker, "nft_token")
        .args_json(((TOTAL_NFT_TO_MINT - 1).to_string(), ))?
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), nft_contract.id().to_string());

    let total_supply: U128 = nft_contract.call(&worker, "nft_total_supply").view().await?.json()?;
    assert_eq!(total_supply, U128::from(TOTAL_NFT_TO_MINT));

    // read 3 tokens somewhere from the end
    let tokens: Vec<Token> = nft_contract.call(&worker, "nft_tokens")
        .args_json((Some(U128::from(700)), Some(3u64)))?
        .view().await?.json()?;

    assert_eq!(tokens.len(), 3);
    assert_eq!(tokens.get(0).unwrap().token_id, "700".to_string());
    assert_eq!(tokens.get(1).unwrap().token_id, "701".to_string());
    assert_eq!(tokens.get(2).unwrap().token_id, "702".to_string());

    Ok(())
}