const anchor = require('@project-serum/anchor');
const { SystemProgram, PublicKey } = require('@solana/web3.js');
const assert = require('chai').assert;

describe('solana_staking_program', () => {
  const provider = anchor.Provider.local();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolanaStakingProgram;
  let stakingAccount, userAccount, owner, user;

  const stakingParams = {
    apy: 10, // Example APY
    lock_duration: 30 * 24 * 60 * 60, // Lock duration (30 days in seconds)
    start_time: Math.floor(Date.now() / 1000) + 100, // Start after 100 seconds
    end_time: Math.floor(Date.now() / 1000) + 200, // End after 200 seconds
  };

  before(async () => {
    // Initialize accounts and owner
    owner = provider.wallet.payer;
    user = anchor.web3.Keypair.generate();

    // Create and initialize the staking account
    stakingAccount = anchor.web3.Keypair.generate();
    await program.rpc.initialize(stakingParams, {
      accounts: {
        stakingAccount: stakingAccount.publicKey,
        owner: owner.publicKey,
        systemProgram: SystemProgram.programId,
      },
      signers: [stakingAccount],
    });

    // Create the user account and token account for staking (this part may require creating an associated token account for the user)
    userAccount = anchor.web3.Keypair.generate();
    const tokenAccount = await provider.connection.getAccountInfo(userAccount.publicKey);

    // Fund user with tokens for staking (you may need to create an associated token account for the user)
  });

  it('should initialize the staking account correctly', async () => {
    const account = await program.account.stakingAccount.fetch(stakingAccount.publicKey);
    
    assert.equal(account.apy.toString(), stakingParams.apy.toString());
    assert.equal(account.lockDuration.toString(), stakingParams.lock_duration.toString());
    assert.equal(account.startTime.toString(), stakingParams.start_time.toString());
    assert.equal(account.endTime.toString(), stakingParams.end_time.toString());
    assert.equal(account.totalStaked.toString(), "0");
    assert.equal(account.rewardPool.toString(), "0");
  });

  it('should allow staking', async () => {
    const amountToStake = 1000; // Example amount to stake

    // Stake the tokens
    await program.rpc.stake(new anchor.BN(amountToStake), {
      accounts: {
        stakingAccount: stakingAccount.publicKey,
        userAccount: userAccount.publicKey,
        user: user.publicKey,
        tokenAccount: userTokenAccount.publicKey, // Ensure user's token account is used
        stakingAccountToken: stakingAccountTokenAccount.publicKey, // The token account for the staking pool
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      },
      signers: [user],
    });

    // Fetch the updated staking account and user account
    const updatedStakingAccount = await program.account.stakingAccount.fetch(stakingAccount.publicKey);
    const updatedUserAccount = await program.account.userAccount.fetch(userAccount.publicKey);

    assert.equal(updatedStakingAccount.totalStaked.toString(), amountToStake.toString());
    assert.equal(updatedUserAccount.stakedAmount.toString(), amountToStake.toString());
  });

  it('should allow claiming rewards', async () => {
    const rewards = 100; // Example rewards amount

    // Claim the rewards
    await program.rpc.claimRewards({
      accounts: {
        stakingAccount: stakingAccount.publicKey,
        userAccount: userAccount.publicKey,
        stakingAccountToken: stakingAccountTokenAccount.publicKey,
        tokenAccount: userTokenAccount.publicKey,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      },
      signers: [user],
    });

    // Fetch the updated staking and user accounts
    const updatedStakingAccount = await program.account.stakingAccount.fetch(stakingAccount.publicKey);
    const updatedUserAccount = await program.account.userAccount.fetch(userAccount.publicKey);

    assert.equal(updatedStakingAccount.rewardPool.toString(), "0");  // After rewards are claimed
    assert.equal(updatedUserAccount.stakedAmount.toString(), "0"); // After unstaking and claiming
  });

  it('should allow unstaking', async () => {
    const amountToUnstake = 1000;

    // Unstake and claim rewards
    await program.rpc.unstake({
      accounts: {
        stakingAccount: stakingAccount.publicKey,
        userAccount: userAccount.publicKey,
        stakingAccountToken: stakingAccountTokenAccount.publicKey,
        tokenAccount: userTokenAccount.publicKey,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      },
      signers: [user],
    });

    // Fetch the updated staking and user accounts
    const updatedStakingAccount = await program.account.stakingAccount.fetch(stakingAccount.publicKey);
    const updatedUserAccount = await program.account.userAccount.fetch(userAccount.publicKey);

    assert.equal(updatedStakingAccount.totalStaked.toString(), "0"); // Staked amount should be 0 after unstaking
    assert.equal(updatedUserAccount.stakedAmount.toString(), "0");  // User's staked amount should be 0
  });
});
