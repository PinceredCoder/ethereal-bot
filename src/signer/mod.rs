use alloy::primitives::{Address, FixedBytes};
use alloy::signers::SignerSync;
use alloy::signers::local::PrivateKeySigner;

mod settings;

use alloy_sol_types::Eip712Domain;
pub(crate) use settings::Config;

use crate::models::{CancelOrder, TradeOrder};

pub(crate) struct Signer {
    pub inner: PrivateKeySigner,
    pub address: Address,
    pub subaccount: FixedBytes<32>,
}

impl Signer {
    pub fn new(config: &Config) -> Self {
        let inner = PrivateKeySigner::from_slice(&config.private_key).unwrap();
        let address = inner.address();
        let subaccount = FixedBytes::<32>::from_slice(&config.subaccount);
        Self {
            inner,
            address,
            subaccount,
        }
    }

    pub fn accound_address(&self) -> &Address {
        &self.address
    }

    pub fn subaccount(&self) -> &FixedBytes<32> {
        &self.subaccount
    }

    pub fn sign_trade_order(
        &self,
        order: &TradeOrder,
        domain: &Eip712Domain,
    ) -> alloy::primitives::Signature {
        self.inner.sign_typed_data_sync(order, domain).unwrap()
    }

    pub fn sign_cancel_order(
        &self,
        nonce: u64,
        domain: &Eip712Domain,
    ) -> (alloy::primitives::Signature, CancelOrder) {
        let msg = CancelOrder {
            sender: self.address,
            subaccount: self.subaccount,
            nonce,
        };

        (self.inner.sign_typed_data_sync(&msg, domain).unwrap(), msg)
    }
}
