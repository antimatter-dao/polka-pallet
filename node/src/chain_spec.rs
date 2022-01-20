use hex_literal::hex;
use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::UncheckedInto};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::FixedPointNumber;

use asset_pool::{AssetPoolInfo, InterestInfo};
use model::{BTC, DOT, DOT_BTC_PAIR, DOT_ETH_PAIR, DOT_FIL_PAIR, ETH, FIL};
use model::Ratio;
use antimatter_network_runtime::{
	AccountId, AssetPoolConfig, AuraConfig, BalancesConfig, DEXConfig,
	GenesisConfig, GrandpaConfig, IncentivesModuleConfig, antimatterOracleConfig, OperatorMembershipantimatterConfig,
	SudoConfig, SystemConfig, TokensConfig,
	WASM_BINARY,
};

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

pub fn development_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		// Name
		"Development",
		// ID
		"dev",
		ChainType::Development,
		move || testnet_genesis(
			wasm_binary,
			// Initial PoA authorities
			vec![
				(hex!["1cd5e0eaff2ff154de522ee3d3d500a7a5996be4e9134ba30697ed973a437920"].unchecked_into(),
				 hex!["8a7867ceece3bf570bb0457462c6da34cfd4807ec94e3a47b7311ca54fa289f0"].unchecked_into()
				)
			],
			// Sudo account
			hex!["1cd5e0eaff2ff154de522ee3d3d500a7a5996be4e9134ba30697ed973a437920"].into(),
			// Pre-funded accounts,
			vec![
				hex!["1cd5e0eaff2ff154de522ee3d3d500a7a5996be4e9134ba30697ed973a437920"].into(),
				hex!["a6a2594caa968af319a705a59c93da73f1ef8c92b114f284a3e761a202f88811"].into(),
			],
			true,
		),
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		None,
		// Extensions
		None,
	))
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		// Name
		"antimatter Testnet",
		// ID
		"antimatter_testnet",
		ChainType::Local,
		move || testnet_genesis(
			wasm_binary,
			// Initial PoA authorities
			vec![
				(hex!["1cd5e0eaff2ff154de522ee3d3d500a7a5996be4e9134ba30697ed973a437920"].unchecked_into(),
				 hex!["8a7867ceece3bf570bb0457462c6da34cfd4807ec94e3a47b7311ca54fa289f0"].unchecked_into()
				)
			],
			// Sudo account
			hex!["1cd5e0eaff2ff154de522ee3d3d500a7a5996be4e9134ba30697ed973a437920"].into(),
			// Pre-funded accounts
			vec![
				hex!["1cd5e0eaff2ff154de522ee3d3d500a7a5996be4e9134ba30697ed973a437920"].into(),
				hex!["a6a2594caa968af319a705a59c93da73f1ef8c92b114f284a3e761a202f88811"].into(),
			],
			true,
		),
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		None,
		// Extensions
		None,
	))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AuraId, GrandpaId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	_enable_println: bool,
) -> GenesisConfig {
	GenesisConfig {
		frame_system: Some(SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
			changes_trie_config: Default::default(),
		}),
		pallet_balances: Some(BalancesConfig {
			// Configure endowed accounts with initial balance of 1 << 60.
			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 60)).collect(),
		}),
		pallet_aura: Some(AuraConfig {
			authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
		}),
		pallet_grandpa: Some(GrandpaConfig {
			authorities: initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect(),
		}),
		pallet_sudo: Some(SudoConfig { key: root_key.clone() }),
		orml_tokens: Some(TokensConfig {
			endowed_accounts: endowed_accounts
				.iter()
				.flat_map(|x| {
					vec![
						(x.clone(), DOT, 10u128.pow(20)),
						(x.clone(), BTC, 10u128.pow(20)),
						(x.clone(), ETH, 10u128.pow(20)),
						(x.clone(), FIL, 10u128.pow(20)),
					]
				})
				.collect(),
		}),
		orml_oracle_Instance1: Some(antimatterOracleConfig {
			members: Default::default(), // initialized by OperatorMembership
			phantom: Default::default(),
		}),
		pallet_membership_Instance1: Some(OperatorMembershipantimatterConfig {
			members: vec![root_key.clone()],
			phantom: Default::default(),
		}),
		incentives: Some(IncentivesModuleConfig {
			loans_incentive_rewards_params: vec![
				(DOT, 10u128.pow(10)),
				(ETH, 10u128.pow(10)),
				(BTC, 10u128.pow(10)),
				(FIL, 10u128.pow(10)),
			]
		}),
		dex: Some(DEXConfig {
			initial_preparing_trading_pairs: vec![
				(
					DOT_ETH_PAIR,
					(5_000_000_000_000u128, 1_000_000_000_000u128),
					(5_000_000_000_000_000u128, 1_000_000_000_000_000u128),
					10,
				),
				(
					DOT_BTC_PAIR,
					(20_000_000_000_000u128, 1_000_000_000u128),
					(20_000_000_000_000_000u128, 1_000_000_000_000u128),
					10,
				),
				(
					DOT_FIL_PAIR,
					(4_000_000_000_000u128, 1_000_000_000u128),
					(4_000_000_000_000_000u128, 1_000_000_000_000u128),
					20,
				),
			],
			initial_enabled_trading_pairs: vec![DOT_ETH_PAIR, DOT_BTC_PAIR, DOT_FIL_PAIR],
			initial_liquidity_pools: vec![(
				root_key,
				vec![
					(DOT_ETH_PAIR, (1_000_000_000u128, 2_000_000_000u128)),
					(DOT_BTC_PAIR, (1_000_000_000u128, 2_000_000_000u128)),
					(DOT_FIL_PAIR, (1_000_000_000u128, 2_000_000_000u128)),
				],
			)],
		}),
		asset_pool: Some(AssetPoolConfig {
			asset_pool_params: vec![
				(DOT, AssetPoolInfo {
					maximum_total_debit_ratio: Ratio::saturating_from_rational(90, 100),
					minimum_deposit: 10u128.pow(1),
					minimum_debit: 10u128.pow(1),
					health_ratio: Ratio::saturating_from_rational(75, 100),
					total_deposit: 0,
					total_debit: 0,
					deposit_rate: Ratio::saturating_from_rational(100, 100),
					debit_rate: Ratio::saturating_from_rational(100, 100),
					deposit_apy: Ratio::saturating_from_rational(0, 100),
					debit_apy: Ratio::saturating_from_rational(0, 100),
					reserve_factor: Ratio::saturating_from_rational(90, 100),
					interest_info: InterestInfo {
						critical_point: Ratio::saturating_from_rational(90, 100),
						base: Ratio::saturating_from_rational(0, 100),
						slope_1: Ratio::saturating_from_rational(4, 100),
						slope_2: Ratio::saturating_from_rational(100, 100),
					},
				}),
				(ETH, AssetPoolInfo {
					maximum_total_debit_ratio: Ratio::saturating_from_rational(90, 100),
					minimum_deposit: 10u128.pow(1),
					minimum_debit: 10u128.pow(1),
					health_ratio: Ratio::saturating_from_rational(75, 100),
					total_deposit: 0,
					total_debit: 0,
					deposit_rate: Ratio::saturating_from_rational(100, 100),
					debit_rate: Ratio::saturating_from_rational(100, 100),
					deposit_apy: Ratio::saturating_from_rational(0, 100),
					debit_apy: Ratio::saturating_from_rational(0, 100),
					reserve_factor: Ratio::saturating_from_rational(90, 100),
					interest_info: InterestInfo {
						critical_point: Ratio::saturating_from_rational(90, 100),
						base: Ratio::saturating_from_rational(0, 100),
						slope_1: Ratio::saturating_from_rational(4, 100),
						slope_2: Ratio::saturating_from_rational(100, 100),
					},
				}),
				(BTC, AssetPoolInfo {
					maximum_total_debit_ratio: Ratio::saturating_from_rational(90, 100),
					minimum_deposit: 10u128.pow(1),
					minimum_debit: 10u128.pow(1),
					health_ratio: Ratio::saturating_from_rational(75, 100),
					total_deposit: 0,
					total_debit: 0,
					deposit_rate: Ratio::saturating_from_rational(100, 100),
					debit_rate: Ratio::saturating_from_rational(100, 100),
					deposit_apy: Ratio::saturating_from_rational(0, 100),
					debit_apy: Ratio::saturating_from_rational(0, 100),
					reserve_factor: Ratio::saturating_from_rational(90, 100),
					interest_info: InterestInfo {
						critical_point: Ratio::saturating_from_rational(90, 100),
						base: Ratio::saturating_from_rational(0, 100),
						slope_1: Ratio::saturating_from_rational(4, 100),
						slope_2: Ratio::saturating_from_rational(100, 100),
					},
				}),
				(FIL, AssetPoolInfo {
					maximum_total_debit_ratio: Ratio::saturating_from_rational(90, 100),
					minimum_deposit: 10u128.pow(1),
					minimum_debit: 10u128.pow(1),
					health_ratio: Ratio::saturating_from_rational(75, 100),
					total_deposit: 0,
					total_debit: 0,
					deposit_rate: Ratio::saturating_from_rational(100, 100),
					debit_rate: Ratio::saturating_from_rational(100, 100),
					deposit_apy: Ratio::saturating_from_rational(0, 100),
					debit_apy: Ratio::saturating_from_rational(0, 100),
					reserve_factor: Ratio::saturating_from_rational(90, 100),
					interest_info: InterestInfo {
						critical_point: Ratio::saturating_from_rational(90, 100),
						base: Ratio::saturating_from_rational(0, 100),
						slope_1: Ratio::saturating_from_rational(4, 100),
						slope_2: Ratio::saturating_from_rational(100, 100),
					},
				}),
			]
		}),
	}
}
