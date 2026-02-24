use std::str::FromStr;

use alloy::network::EthereumWallet;
use alloy::primitives::{Address, FixedBytes, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::signers::local::PrivateKeySigner;

use crate::tests::{EXCHANGE, IERC20, IExchange, IWUSDE, RPC_URL, TOKEN};

async fn setup() -> (impl Provider, Address) {
    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY required");
    let signer = PrivateKeySigner::from_str(&private_key).unwrap();
    let address = signer.address();
    let wallet = EthereumWallet::new(signer);
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect_http(RPC_URL.parse().unwrap());
    (provider, address)
}

#[tokio::test]
async fn check_balances() {
    let (provider, address) = setup().await;

    let native = provider.get_balance(address).await.unwrap();
    println!("native USDe = {native}");

    let token = IERC20::new(TOKEN, &provider);
    let wusde = token.balanceOf(address).call().await.unwrap();
    println!("WUSDe = {wusde}");
}

#[tokio::test]
#[ignore]
async fn wrap_usde() {
    let decimals = U256::from(10u128).pow(U256::from(18u128));

    let (provider, _address) = setup().await;

    let amount = U256::from(10u128) * decimals;

    let wusde = IWUSDE::new(TOKEN, &provider);
    let tx = wusde
        .deposit()
        .value(amount)
        .send()
        .await
        .unwrap()
        .watch()
        .await
        .unwrap();
    println!("wrap tx: {tx}");
}

#[tokio::test]
#[ignore]
async fn create_subaccount() {
    let decimals = U256::from(10u128).pow(U256::from(18u128));

    let (provider, _address) = setup().await;
    let amount = U256::from(10u128) * decimals;

    let token = IERC20::new(TOKEN, &provider);

    let approve_tx = token
        .approve(EXCHANGE, amount)
        .send()
        .await
        .unwrap()
        .watch()
        .await
        .unwrap();
    println!("approve tx: {approve_tx}");

    let exchange = IExchange::new(EXCHANGE, &provider);
    let subaccount = FixedBytes::<32>::from_slice(
        &hex::decode("7072696d61727900000000000000000000000000000000000000000000000000").unwrap(),
    );

    let deposit_tx = exchange
        .deposit(subaccount, TOKEN, amount, FixedBytes::<32>::ZERO)
        .send()
        .await
        .unwrap()
        .watch()
        .await
        .unwrap();
    println!("deposit tx: {deposit_tx}");
}
