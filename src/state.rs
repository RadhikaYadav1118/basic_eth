use crate::ecdsa::EcdsaPublicKey;
use crate::{EcdsaKeyName, EthereumNetwork, InitArg};
use evm_rpc_canister_types::{EthMainnetService, EthSepoliaService, RpcServices};
use ic_cdk::api::management_canister::ecdsa::EcdsaKeyId;
use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
};
use crate::{
    ed25519::{get_ed25519_public_key, Ed25519ExtendedPublicKey},
    Ed25519KeyName, InitArg, SolanaNetwork,
};
use candid::Principal;
use sol_rpc_types::CommitmentLevel;

thread_local! {
    pub static STATE: RefCell<State> = RefCell::default();
}

pub fn init_state(init_arg: InitArg) {
    STATE.with(|s| *s.borrow_mut() = State::from(init_arg));
}

pub fn read_state<R>(f: impl FnOnce(&State) -> R) -> R {
    STATE.with(|s| f(s.borrow().deref()))
}

pub fn mutate_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut State) -> R,
{
    STATE.with(|s| f(s.borrow_mut().deref_mut()))
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct State {
    ethereum_network: EthereumNetwork,
    ecdsa_key_name: EcdsaKeyName,
    ecdsa_public_key: Option<EcdsaPublicKey>,
    sol_rpc_canister_id: Option<Principal>,
    solana_network: SolanaNetwork,
    solana_commitment_level: CommitmentLevel,
    ed25519_public_key: Option<Ed25519ExtendedPublicKey>,
    ed25519_key_name: Ed25519KeyName,
}

impl State {
    pub fn ecdsa_key_id(&self) -> EcdsaKeyId {
        EcdsaKeyId::from(&self.ecdsa_key_name)
    }

    pub fn ethereum_network(&self) -> EthereumNetwork {
        self.ethereum_network
    }

    pub fn evm_rpc_services(&self) -> RpcServices {
        match self.ethereum_network {
            EthereumNetwork::Mainnet => RpcServices::EthMainnet(None),
            EthereumNetwork::Sepolia => RpcServices::EthSepolia(None),
        }
    }

    pub fn single_evm_rpc_service(&self) -> RpcServices {
        match self.ethereum_network {
            EthereumNetwork::Mainnet => {
                RpcServices::EthMainnet(Some(vec![EthMainnetService::PublicNode]))
            }
            EthereumNetwork::Sepolia => {
                RpcServices::EthSepolia(Some(vec![EthSepoliaService::PublicNode]))
            }
        }
    }
        pub fn ed25519_key_name(&self) -> Ed25519KeyName {
        self.ed25519_key_name
    }

    pub fn solana_network(&self) -> &SolanaNetwork {
        &self.solana_network
    }

    pub fn solana_commitment_level(&self) -> CommitmentLevel {
        self.solana_commitment_level.clone()
    }

    pub fn sol_rpc_canister_id(&self) -> Option<Principal> {
        self.sol_rpc_canister_id
    }
}

impl From<InitArg> for State {
    fn from(init_arg: InitArg) -> Self {
        State {
            ethereum_network: init_arg.ethereum_network.unwrap_or_default(),
            ecdsa_key_name: init_arg.ecdsa_key_name.unwrap_or_default(),
            ..Default::default(),
            sol_rpc_canister_id: init_arg.sol_rpc_canister_id,
            solana_network: init_arg.solana_network.unwrap_or_default(),
            solana_commitment_level: init_arg.solana_commitment_level.unwrap_or_default(),
            ed25519_public_key: None,
            ed25519_key_name: init_arg.ed25519_key_name.unwrap_or_default(),

        }
    }
}

pub async fn lazy_call_ecdsa_public_key() -> EcdsaPublicKey {
    use ic_cdk::api::management_canister::ecdsa::{ecdsa_public_key, EcdsaPublicKeyArgument};

    if let Some(ecdsa_pk) = read_state(|s| s.ecdsa_public_key.clone()) {
        return ecdsa_pk;
    }
    let public_key =
        get_ed25519_public_key(read_state(|s| s.ed25519_key_name()), &Default::default()).await;
    mutate_state(|s| s.ed25519_public_key = Some(public_key.clone()));
    public_key
    let key_id = read_state(|s| s.ecdsa_key_id());
    let (response,) = ecdsa_public_key(EcdsaPublicKeyArgument {
        canister_id: None,
        derivation_path: vec![],
        key_id,
    })
    .await
    .unwrap_or_else(|(error_code, message)| {
        ic_cdk::trap(&format!(
            "failed to get canister's public key: {} (error code = {:?})",
            message, error_code,
        ))
    });
    let pk = EcdsaPublicKey::from(response);
    mutate_state(|s| s.ecdsa_public_key = Some(pk.clone()));
    pk
}
