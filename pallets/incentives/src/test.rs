#![cfg(test)]

use frame_support::{assert_noop, assert_ok};
use orml_rewards::PoolInfo;
use sp_runtime::{traits::BadOrigin};

use mock::{*};

use super::*;

#[test]
fn update_loans_incentive_rewards_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			IncentivesModule::update_loans_incentive_rewards(Origin::signed(ALICE), vec![]),
			BadOrigin
		);
		assert_eq!(IncentivesModule::loans_incentive_rewards(BTC), 0);
		assert_eq!(IncentivesModule::loans_incentive_rewards(DOT), 0);

		assert_ok!(IncentivesModule::update_loans_incentive_rewards(
			Origin::signed(4),
			vec![(BTC, 200), (DOT, 1000),],
		));
		assert_eq!(IncentivesModule::loans_incentive_rewards(BTC), 200);
		assert_eq!(IncentivesModule::loans_incentive_rewards(DOT), 1000);

		assert_ok!(IncentivesModule::update_loans_incentive_rewards(
			Origin::signed(4),
			vec![(BTC, 100), (BTC, 300), (BTC, 500),],
		));
		assert_eq!(IncentivesModule::loans_incentive_rewards(BTC), 500);
	});
}

#[test]
fn on_update_loan_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			RewardsModule::pools(BTC),
			PoolInfo {
				total_shares: 0,
				total_rewards: 0,
				total_withdrawn_rewards: 0,
			}
		);
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(BTC, ALICE),
			(0, 0)
		);
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(BTC, BOB),
			(0, 0)
		);

		OnUpdateLoan::<Runtime>::happened(&(ALICE, BTC, 100, 0));
		assert_eq!(
			RewardsModule::pools(BTC),
			PoolInfo {
				total_shares: 100,
				total_rewards: 0,
				total_withdrawn_rewards: 0,
			}
		);
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(BTC, ALICE),
			(100, 0)
		);
	});
}

#[test]
fn accumulate_reward_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(IncentivesModule::update_loans_incentive_rewards(
			Origin::signed(4),
			vec![(BTC, 1000), (DOT, 2000),],
		));
		assert_eq!(IncentivesModule::accumulate_reward(10, |_, _| {}), vec![]);

		RewardsModule::add_share(&ALICE, BTC, 1);
		assert_eq!(IncentivesModule::accumulate_reward(20, |_, _| {}), vec![(MB, 1000)]);
	});
}
