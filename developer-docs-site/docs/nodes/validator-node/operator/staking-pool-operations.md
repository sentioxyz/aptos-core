---
title: "Staking Pool Operations"
slug: "staking-pool-operations"
---

# Staking Pool Operations

This document describes how to perform [staking](../../../concepts/staking.md) pool operations. Note that a staking pool can only accept stake from the stake pool owner. You can stake only when you meet the minimum staking requirement. See also the related [Delegation Pool Operations](./delegation-pool-operations.md) instructions to accept stake from multiple delegators in order to reach the minimum. 

:::tip Minimum staking requirement
The current required minimum for staking is 1 million APT.
:::

:::danger
Important There is no upgrade mechanism for the staking contract from staking pool to delegation pool. A new delegation pool would have to be created.
:::


## Initialize a staking pool

Before initializing a staking pool, ensure that there is an existing owner account with 1M APT.

1. Initialize [Aptos CLI](../../../tools/aptos-cli/index.md) with a private key from an existing account, such as a wallet, or create a new account.

```bash
aptos init --profile mainnet-owner \
  --network mainnet
```

You can either enter the private key from an existing wallet, or create new wallet address.

2. Run the following command to initialize the staking pool:

```bash
aptos stake create-staking-contract \
--operator <operator-address> \
--voter <voter-address> \
--amount 100000000000000 \
--commission-percentage 10 \
--profile mainnet-owner
```

3. Once the staking pool has been initialized, you can proceed to [connect to the Aptos Network](./connect-to-aptos-network.md).

## Perform pool owner operations

:::tip Testnet vs Mainnet
The below Aptos CLI command examples use mainnet. Change the `--network` value for testnet and devnet. View the values in [Aptos Blockchain Deployments](../../deployments.md) to see how profiles can be configured based on the network.
:::

### Transfer coin between accounts

```bash
aptos account transfer \
  --account <operator-address> \
  --amount <amount> \
  --profile mainnet-owner
```

### Switch operator

```bash
aptos stake set-operator \
  --operator-address <new-operator-address> \ 
  --profile mainnet-owner
```

### Switch voter

```bash
aptos stake set-delegated-voter \
  --voter-address <new-voter-address> \ 
  --profile mainnet-owner
```

### Add stake

```bash
aptos stake add-stake \
  --amount <amount> \
  --profile mainnet-owner
```

### Increase stake lockup

```bash
aptos stake increase-lockup --profile mainnet-owner
```

### Unlock stake

```bash
aptos stake unlock-stake \
  --amount <amount> \
  --profile mainnet-owner
```

### Withdraw stake

```bash
aptos stake withdraw-stake \
  --amount <amount> \
  --profile mainnet-owner
```

### Update commission
```bash
aptos move run --function-id 0x1::staking_contract::update_commision \
--args address:<operator_address> u64:<commission_percentage> \
--profile mainnet-owner
```


## Checking your stake pool information

:::tip How validation works
Before you proceed, see [Validation on the Aptos blockchain](../../../concepts/staking.md#validation-on-the-aptos-blockchain) for a brief overview.
:::

To check the details of your stake pool, run the below CLI command with the `get-stake-pool` option by providing the `--owner-address` and `--url` fields. 

The below command is for an example owner address `e7be097a90c18f6bdd53efe0e74bf34393cac2f0ae941523ea196a47b6859edb`. 

:::tip
For testnet or devnet `--url` field values, see [Aptos Blockchain Deployments](../../deployments.md).
:::

```bash
aptos node get-stake-pool \
  --owner-address e7be097a90c18f6bdd53efe0e74bf34393cac2f0ae941523ea196a47b6859edb \
  --profile mainnet-operator
```

Example output:

```json
{
  "Result": [
    {
      "state": "Active", 
      "pool_address": "25c3482850a188d8aa6edc5751846e1226a27863643f5ebc52be4f7d822264e3",
      "operator_address": "3bec5a529b023449dfc86e9a6b5b51bf75cec4a62bf21c15bbbef08a75f7038f",
      "voter_address": "3bec5a529b023449dfc86e9a6b5b51bf75cec4a62bf21c15bbbef08a75f7038f",
      "pool_type": "StakingContract",
      "total_stake": 100525929489123,
      "commission_percentage": 10,
      "commission_not_yet_unlocked": 15949746439,
      "lockup_expiration_utc_time": "2022-10-07T07:12:55Z",
      "consensus_public_key": "0xb3a7ac1491b0165f08f136c2b02739846b6610084984d5298c2983c4f8e5553284bffca2e3fe2b99167da82717501732",
      "validator_network_addresses": [
        "/ip4/35.91.145.164/tcp/6180/noise-ik/0xeddf05470520af91b847f353dd804a04399e1213d130a4260e813527f2c49262/handshake/0"
      ],
      "fullnode_network_addresses": [],
      "epoch_info": {
        "epoch": 594,
        "epoch_interval_secs": 3600,
        "current_epoch_start_time": {
          "unix_time": 1665087178789891,
          "utc_time": "2022-10-06T20:12:58.789891Z"
        },
        "next_epoch_start_time": {
          "unix_time": 1665090778789891,
          "utc_time": "2022-10-06T21:12:58.789891Z"
        }
      }
    }
  ]
}
```

### Description of output fields

**state**
- "Active": Validator is already in the validator set and proposing.
- "Pending_active": Validator will be added to the validator set in the next epoch. **Do not try to join the validator set again before the arrival of next epoch, or else you will receive an error. **

**pool_address**
- Use this "pool_address" (not the operator address) in you `validator.yaml` file. If you mistakenly used the operator address, you will receive the message: "Validator not in validator set". 

**commission_percentage**
- This can be set only by the stake pool owner. Operator receives the "commission_percentage" of the generated staking rewards. If you request the commission (you can do so by running the command `aptos stake request-commission`), then at the end of the `lockup_expiration_utc_time` the commission part of the rewards will go to the operator address while the rest will stay in the stake pool and belong to the owner. Here "the commission part of the rewards" means the value of **commission_not_yet_unlocked**. 

  For example, in a scenario with a lock up of one month, you call `aptos stake request-commission` every month. This will pay out the commission that was accrued during the previous month but only when unlocked at the end of the previous month. Regardless of how often you run `aptos stake request-commission` during the month, the commission is only paid out upon the completion of `lockup_expiration_utc_time`.

  :::tip Compounding
  Note that if you do not request commission for multiple months, your commission will accrue more due to compounding of the **commission_percentage** during these months.
  :::


**commission_not_yet_unlocked**
- The amount of commission (amount of APT) that is not yet unlocked. It will be unlocked at the `lockup_expiration_utc_time`. This is the total commission amount available to the operator, i.e., the staking rewards **only** to the operator. This does not include the staking rewards to the owner.

**lockup_expiration_utc_time**
- The date when the commission will unlock. However, this unlocked commission will not be auto-disbursed. It will only disburse when the command `aptos stake request-commission` is called again.

**epoch_info**
- Use the [Epoch Converter](https://www.epochconverter.com/) to convert the `unix_time` into human readable time. 

## Requesting commission

Either an owner or an operator can request commission. You can request commission at the end of a lockup period, i.e., at the end of **lockup_expiration_utc_time**, by running the `aptos stake request-commission` command. Make sure to provide the operator and the owner addresses. See an example command below:

```bash
aptos stake request-commission \
  --operator-address 0x3bec5a529b023449dfc86e9a6b5b51bf75cec4a62bf21c15bbbef08a75f7038f \
  --owner-address 0xe7be097a90c18f6bdd53efe0e74bf34393cac2f0ae941523ea196a47b6859edb \
  --profile mainnet-operator
```

If you run the `aptos stake request-commission` command before the end of the lockup expiration, the command will initiate unlock for any locked commission earned up until that moment in time.

See example below:

Month 1 Day 29, you call the command, it would initiate unlock for 29 days worth of commission.

Month 2, Day 29, if you call the command again, it would disburse the fully unlocked commission from previous month (29 days worth), and initiate commission unlock for Month 1 Day 30 + Month 2 Day 1-29 (30 days worth).

Month 3, Day 29, if you call the commission again, 30 days of commission would be disbursed, and the a new batch of commission would initiate unlock.

You can call the command multiple times, and the amount you receive depends on the day when you requested commission unlock previously.


Commission is unlocked when `request-commission` is called, the staker unlocks stake, or the staker switches operator. The commission will not be withdrawable until the end of the lockup period. Unlocked commission will continue to earn rewards until the lockup period expires.


## Checking your validator performance

To see your validator performance in the current and past epochs and the rewards earned, run the below command. The output will show the validator's performance in block proposals, and in governance voting and governance proposals. Default values are used in the below command. Type `aptos node get-performance --help` to see default values used.

```bash
aptos node get-performance \ 
  --pool-address <pool address> \
  --profile mainnet-operator
```

Example output:

```json
{
  "Result": {
    "current_epoch_successful_proposals": 56,
    "current_epoch_failed_proposals": 0,
    "previous_epoch_rewards": [
      "12312716242",
      "12272043711",
      "12312912674",
      "12313011054",
      "12313109435",
      "12180092056",
      "12313305136",
      "12313403519",
      "12313501903",
      "12313600288"
    ],
    "epoch_info": {
      "epoch": 68,
      "epoch_interval": 3600000000,
      "last_epoch_start_time": {
        "unix_time": 1665074662417326,
        "utc_time": "2022-10-06T16:44:22.417326Z",
        "local_time": "Thu Oct  6 16:44:22 2022"
      },
      "next_epoch_start_time": {
        "unix_time": 1665078262417326,
        "utc_time": "2022-10-06T17:44:22.417326Z",
        "local_time": "Thu Oct  6 17:44:22 2022"
      }
    }
  }
}
```

#### Description of fields

**current_epoch_successful_proposals**
- Successful leader-validator proposals during the current epoch. Also see [Validation on the Aptos blockchain](../../../concepts/staking.md#validation-on-the-aptos-blockchain) for the distinction between leader-validator and the voter-validator.

**previous_epoch_rewards**
- An ordered list of rewards earned (APT amounts) for the previous 10 epochs, starting with the 10 epoch in the past. In the above example, a reward of 12312716242 APT was earned 10 epochs past and a reward of 12313600288 APT was earned in the most recent epoch. If a reward is 0 for any epoch, then:
  - Either the validator was not part of the validator set in that epoch (could have been in either inactive or pending_active validator state), or
  - The validator missed all the leader proposals.

### Checking the performance for all epochs

To check the performance of all the epochs since the genesis, run the below command. You can filter the results for your pool address with `grep`, as shown below:

```bash
aptos node analyze-validator-performance \
  --analyze-mode detailed-epoch-table \
  --profile mainnet-operator \
  --start-epoch 0 | grep <pool address>
```

## Tracking rewards

`DistributeEvent` is emitted when there is a transfer from staking_contract to the operator or staker (owner). Rewards can be tracked either by listening to `DistributeEvent` or by using the [View function](../../../integration/aptos-apis.md#reading-state-with-the-view-function) to call `staking_contract_amounts`. This will return `accumulated_rewards` and `commission_amount`.
