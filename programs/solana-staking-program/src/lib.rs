use anchor_lang::prelude::*;
use anchor_spl::{token::{self, Token, TokenAccount, Transfer}};

// Declare the program ID (unique identifier for this program)
declare_id!("FDxReJFEFS2kWBW2vRZ6kkGAi3CueyqKBzSQceN6Jyy");

#[program]
pub mod solana_staking_program {
    use super::*;

    // Initialize the staking account with the provided parameters
    pub fn initialize(ctx: Context<Initialize>, staking_params: StakingParams) -> Result<()> {
        let staking_account = &mut ctx.accounts.staking_account;
        // Set staking account owner and parameters (APY, lock duration, etc.)
        staking_account.owner = ctx.accounts.owner.key();
        staking_account.apy = staking_params.apy;
        staking_account.lock_duration = staking_params.lock_duration;
        staking_account.start_time = staking_params.start_time;
        staking_account.end_time = staking_params.end_time;
        staking_account.total_staked = 0; // Initialize total staked to 0
        staking_account.reward_pool = 0; // Initialize reward pool to 0
        Ok(())
    }

    // Stake tokens into the staking pool
    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        let clock = Clock::get()?; // Get the current blockchain clock
        let user = &mut ctx.accounts.user_account; // Reference to the user account
        let staking_account = &mut ctx.accounts.staking_account; // Reference to the staking account

        // Ensure staking is within the valid time range
        require!(
            clock.unix_timestamp >= staking_account.start_time,
            CustomError::StakingNotStarted
        );
        require!(
            clock.unix_timestamp <= staking_account.end_time,
            CustomError::StakingEnded
        );

        // Check if the user has already staked
        require!(user.staked_amount == 0, CustomError::AlreadyStaked);

        // Update user's staking details
        user.staked_amount = amount;
        user.stake_start_time = clock.unix_timestamp;
        user.reward_start_time = clock.unix_timestamp;
        user.lock_duration = staking_account.lock_duration;
        user.apy = staking_account.apy;

        // Update the total staked in the staking account
        staking_account.total_staked += amount;

        // Transfer tokens from the user's account to the staking pool
        let cpi_accounts = Transfer {
            from: ctx.accounts.token_account.to_account_info(),
            to: ctx.accounts.staking_account_token.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_context = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_context, amount)?; // Perform the token transfer
        Ok(())
    }

    // Claim rewards earned by the user
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        let clock = Clock::get()?; // Get the current blockchain clock
        let staking_account_info = ctx.accounts.staking_account.to_account_info(); // Immutable reference for authority
        let staking_account = &mut ctx.accounts.staking_account; // Mutable reference to staking account

        let user = &mut ctx.accounts.user_account; // Mutable reference to user account

        // Calculate the rewards based on staked amount, APY, and time elapsed
        let rewards = calculate_rewards(
            user.staked_amount,
            staking_account.apy,
            user.reward_start_time,
            clock.unix_timestamp,
        );

        // Transfer rewards from the staking pool to the user's token account
        let cpi_accounts = Transfer {
            from: ctx.accounts.staking_account_token.to_account_info(),
            to: ctx.accounts.token_account.to_account_info(),
            authority: staking_account_info,
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_context = CpiContext::new(cpi_program, cpi_accounts);

        token::transfer(cpi_context, rewards)?; // Perform the token transfer

        // Update the reward pool and user's reward start time
        staking_account.reward_pool -= rewards;
        user.reward_start_time = clock.unix_timestamp;

        Ok(())
    }

    // Unstake tokens and claim rewards
    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        let clock = Clock::get()?; // Get the current blockchain clock
        let user = &mut ctx.accounts.user_account; // Reference to the user account

        // Create an immutable reference to the staking account for later use
        let staking_account_info = ctx.accounts.staking_account.to_account_info();

        // Mutable borrow of staking account for updates
        let staking_account = &mut ctx.accounts.staking_account;

        // Ensure the user has staked tokens
        require!(user.staked_amount > 0, CustomError::NothingStaked);

        // Ensure the lock period is over
        require!(
            clock.unix_timestamp >= user.stake_start_time + user.lock_duration,
            CustomError::LockPeriodNotOver
        );

        // Calculate rewards
        let rewards = calculate_rewards(
            user.staked_amount,
            user.apy,
            user.reward_start_time,
            clock.unix_timestamp,
        );

        // Ensure the staking account has enough rewards in the pool
        require!(
            staking_account.reward_pool >= rewards,
            CustomError::InsufficientRewardPool
        );

        // Transfer staked amount and rewards back to the user's account
        let cpi_accounts = Transfer {
            from: ctx.accounts.staking_account_token.to_account_info(),
            to: ctx.accounts.token_account.to_account_info(),
            authority: staking_account_info,
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_context = CpiContext::new(cpi_program, cpi_accounts);

        token::transfer(cpi_context, user.staked_amount + rewards)?; // Perform the token transfer

        // Update staking account and user account details
        staking_account.total_staked -= user.staked_amount;
        staking_account.reward_pool -= rewards;
        user.staked_amount = 0;
        user.reward_start_time = 0;

        Ok(())
    }

    // Update staking parameters (APY, lock duration, etc.)
    pub fn update_staking_params(
        ctx: Context<UpdateStakingParams>,
        staking_params: StakingParams,
    ) -> Result<()> {
        let staking_account = &mut ctx.accounts.staking_account;
        staking_account.apy = staking_params.apy;
        staking_account.lock_duration = staking_params.lock_duration;
        staking_account.start_time = staking_params.start_time;
        staking_account.end_time = staking_params.end_time;
        Ok(())
    }

    // Add rewards to the staking pool
    pub fn add_rewards(ctx: Context<AddRewards>, amount: u64) -> Result<()> {
        let cpi_accounts = Transfer {
            from: ctx.accounts.source_token_account.to_account_info(),
            to: ctx.accounts.staking_account_token.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_context = CpiContext::new(cpi_program, cpi_accounts);

        token::transfer(cpi_context, amount)?; // Transfer tokens to the reward pool

        Ok(())
    }
}

// Helper function to calculate rewards
fn calculate_rewards(amount: u64, apy: u64, start: i64, end: i64) -> u64 {
    let duration = end - start; // Time elapsed in seconds
    let yearly_rewards = (amount as u128 * apy as u128) / 100; // Calculate annual rewards
    let rewards = yearly_rewards * duration as u128 / 31_536_000; // Convert to rewards for elapsed time
    rewards as u64
}

// Data structures and accounts
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct StakingParams {
    pub apy: u64,
    pub lock_duration: i64,
    pub start_time: i64,
    pub end_time: i64,
}

#[account]
pub struct StakingAccount {
    pub owner: Pubkey,
    pub apy: u64,
    pub lock_duration: i64,
    pub start_time: i64,
    pub end_time: i64,
    pub total_staked: u64,
    pub reward_pool: u64,
}

#[account]
pub struct UserAccount {
    pub user: Pubkey,
    pub staked_amount: u64,
    pub stake_start_time: i64,
    pub reward_start_time: i64,
    pub lock_duration: i64,
    pub apy: u64,
}

#[error_code]
pub enum CustomError {
    #[msg("Staking has not started yet.")]
    StakingNotStarted,
    #[msg("Staking period has ended.")]
    StakingEnded,
    #[msg("Lock period has not ended yet.")]
    LockPeriodNotOver,
    #[msg("Insufficient reward pool.")]
    InsufficientRewardPool,
    #[msg("Nothing is staked.")]
    NothingStaked,
    #[msg("User already has an active stake.")]
    AlreadyStaked,
}

// Contexts for instructions
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = owner, space = 8 + 128)]
    pub staking_account: Account<'info, StakingAccount>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub staking_account: Account<'info, StakingAccount>,
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub staking_account_token: Account<'info, TokenAccount>,
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub staking_account: Account<'info, StakingAccount>,
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
    #[account(mut)]
    pub staking_account_token: Account<'info, TokenAccount>,
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub staking_account: Account<'info, StakingAccount>,
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
    #[account(mut)]
    pub staking_account_token: Account<'info, TokenAccount>,
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct UpdateStakingParams<'info> {
    #[account(mut)]
    pub staking_account: Account<'info, StakingAccount>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct AddRewards<'info> {
    #[account(mut)]
    pub staking_account: Account<'info, StakingAccount>,
    #[account(mut)]
    pub source_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub staking_account_token: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub owner: Signer<'info>,
}

