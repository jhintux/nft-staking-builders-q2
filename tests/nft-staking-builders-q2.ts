import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { assert } from "chai";
import { NftStakingBuildersQ2 } from "../target/types/nft_staking_builders_q2";

const MPL_CORE_PROGRAM_ID = new anchor.web3.PublicKey(
  "CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d",
);
const TOKEN_PROGRAM_ID = new anchor.web3.PublicKey(
  "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
);
const ASSOCIATED_TOKEN_PROGRAM_ID = new anchor.web3.PublicKey(
  "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
);

/** Whole-token reward rate: 1 token/sec (6 decimals). */
const REWARD_RATE_PER_SEC = new anchor.BN(1_000_000);
const FREEZE_PERIOD = 0;
const REWARD_ACCRUAL_SECONDS = 5;

type StakingContext = {
  collection: anchor.web3.Keypair;
  updateAuthority: anchor.web3.PublicKey;
  config: anchor.web3.PublicKey;
  rewardsMint: anchor.web3.PublicKey;
  userRewardsTokenAccount: anchor.web3.PublicKey;
};

/** Surfpool cheatcode — absoluteTimestamp is in milliseconds. */
async function surfpoolTimeTravel(
  connection: anchor.web3.Connection,
  unixSeconds: number,
) {
  if (unixSeconds <= 0) {
    throw new Error(`Invalid time travel target: ${unixSeconds}`);
  }

  const res = await fetch(connection.rpcEndpoint, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "surfnet_timeTravel",
      params: [{ absoluteTimestamp: unixSeconds * 1000 }],
    }),
  });
  const json = (await res.json()) as {
    error?: { message: string; data?: string };
  };
  if (json.error) {
    throw new Error(
      `surfnet_timeTravel failed: ${json.error.message}${json.error.data ? ` (${json.error.data})` : ""}`,
    );
  }
}

function parseAttributeTimestamp(data: Buffer, attributeKey: string): number {
  const keyLen = attributeKey.length;

  for (let i = 0; i <= data.length - 8; i++) {
    const len = data.readUInt32LE(i);
    if (len !== keyLen) {
      continue;
    }
    if (
      data.subarray(i + 4, i + 4 + keyLen).toString("utf8") !== attributeKey
    ) {
      continue;
    }

    const valueOffset = i + 4 + keyLen;
    const valueLen = data.readUInt32LE(valueOffset);
    if (valueLen < 1 || valueLen > 13) {
      continue;
    }

    const value = data
      .subarray(valueOffset + 4, valueOffset + 4 + valueLen)
      .toString("utf8");
    const parsed = Number.parseInt(value, 10);
    if (attributeKey === "staked" && parsed === 0) {
      continue;
    }
    if (!Number.isNaN(parsed) && parsed > 0) {
      return parsed;
    }
  }

  throw new Error(`Could not find ${attributeKey} attribute on asset`);
}

async function readAssetAttributeTimestamp(
  connection: anchor.web3.Connection,
  asset: anchor.web3.PublicKey,
  attributeKey: "staked" | "last_claimed_at",
): Promise<number> {
  const accountInfo = await connection.getAccountInfo(asset);
  if (!accountInfo) {
    throw new Error(`Asset account not found: ${asset.toBase58()}`);
  }

  return parseAttributeTimestamp(accountInfo.data, attributeKey);
}

function updateAuthorityPda(
  collection: anchor.web3.PublicKey,
  programId: anchor.web3.PublicKey,
) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("update_authority"), collection.toBuffer()],
    programId,
  )[0];
}

function configPda(
  collection: anchor.web3.PublicKey,
  programId: anchor.web3.PublicKey,
) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("config"), collection.toBuffer()],
    programId,
  )[0];
}

function rewardsMintPda(
  collection: anchor.web3.PublicKey,
  programId: anchor.web3.PublicKey,
) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("rewards_mint"), collection.toBuffer()],
    programId,
  )[0];
}

function userRewardsAta(
  owner: anchor.web3.PublicKey,
  rewardsMint: anchor.web3.PublicKey,
) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [owner.toBuffer(), TOKEN_PROGRAM_ID.toBuffer(), rewardsMint.toBuffer()],
    ASSOCIATED_TOKEN_PROGRAM_ID,
  )[0];
}

async function expectProgramError(
  promise: Promise<unknown>,
  code: string,
): Promise<void> {
  try {
    await promise;
    assert.fail(`Expected ${code} but transaction succeeded`);
  } catch (err) {
    if (err instanceof anchor.AnchorError) {
      assert.equal(err.error.errorCode.code, code);
      return;
    }
    const message = err instanceof Error ? err.message : String(err);
    assert.include(message, code, `Expected ${code} in: ${message}`);
  }
}

describe("nft-staking-builders-q2", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace
    .nftStakingBuildersQ2 as Program<NftStakingBuildersQ2>;

  async function setupInitializedCollection(
    freezePeriod = FREEZE_PERIOD,
    rewardRate = REWARD_RATE_PER_SEC,
  ): Promise<StakingContext> {
    const collection = anchor.web3.Keypair.generate();
    const updateAuthority = updateAuthorityPda(
      collection.publicKey,
      program.programId,
    );
    const config = configPda(collection.publicKey, program.programId);
    const rewardsMint = rewardsMintPda(collection.publicKey, program.programId);
    const userRewardsTokenAccount = userRewardsAta(
      provider.wallet.publicKey,
      rewardsMint,
    );

    await program.methods
      .createCollection("Test Collection", "https://example.com/collection")
      .accountsStrict({
        payer: provider.wallet.publicKey,
        collection: collection.publicKey,
        updateAuthority,
        systemProgram: anchor.web3.SystemProgram.programId,
        mplProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([collection])
      .rpc();

    await program.methods
      .initialize(rewardRate, freezePeriod)
      .accountsStrict({
        payer: provider.wallet.publicKey,
        config,
        rewardsMint,
        collection: collection.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    return {
      collection,
      updateAuthority,
      config,
      rewardsMint,
      userRewardsTokenAccount,
    };
  }

  async function mintAssetInCollection(ctx: StakingContext) {
    const asset = anchor.web3.Keypair.generate();

    await program.methods
      .mintAsset("Test NFT", "https://example.com/nft")
      .accountsStrict({
        payer: provider.wallet.publicKey,
        asset: asset.publicKey,
        collection: ctx.collection.publicKey,
        updateAuthority: ctx.updateAuthority,
        mplProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([asset])
      .rpc();

    return asset;
  }

  function stakeAccounts(ctx: StakingContext, asset: anchor.web3.PublicKey) {
    return {
      owner: provider.wallet.publicKey,
      asset,
      collection: ctx.collection.publicKey,
      updateAuthority: ctx.updateAuthority,
      mplProgram: MPL_CORE_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
    };
  }

  function claimAccounts(ctx: StakingContext, asset: anchor.web3.PublicKey) {
    return {
      owner: provider.wallet.publicKey,
      asset,
      collection: ctx.collection.publicKey,
      config: ctx.config,
      rewardsMint: ctx.rewardsMint,
      userRewardsTokenAccount: ctx.userRewardsTokenAccount,
      updateAuthority: ctx.updateAuthority,
      mplProgram: MPL_CORE_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
    };
  }

  async function stakeAsset(ctx: StakingContext, asset: anchor.web3.Keypair) {
    const stakeSig = await program.methods
      .stake()
      .accountsStrict(stakeAccounts(ctx, asset.publicKey))
      .rpc();

    const stakedAt = await readAssetAttributeTimestamp(
      provider.connection,
      asset.publicKey,
      "staked",
    );

    return { stakeSig, stakedAt };
  }

  it("happy path: create_collection -> initialize -> mint_asset -> stake -> claim_rewards -> unstake", async () => {
    const ctx = await setupInitializedCollection();
    const asset = await mintAssetInCollection(ctx);

    const { stakedAt } = await stakeAsset(ctx, asset);
    await surfpoolTimeTravel(
      provider.connection,
      stakedAt + REWARD_ACCRUAL_SECONDS,
    );

    await program.methods
      .claimRewards()
      .accountsStrict(claimAccounts(ctx, asset.publicKey))
      .rpc();

    const balanceAfterClaim = await provider.connection.getTokenAccountBalance(
      ctx.userRewardsTokenAccount,
    );
    const expectedRewards = REWARD_RATE_PER_SEC.muln(REWARD_ACCRUAL_SECONDS);
    assert.equal(
      balanceAfterClaim.value.amount,
      expectedRewards.toString(),
      "claim_rewards should mint accrued tokens",
    );

    await program.methods
      .unstake()
      .accountsStrict(claimAccounts(ctx, asset.publicKey))
      .rpc();

    const configAccount = await program.account.config.fetch(ctx.config);
    assert.equal(configAccount.freezePeriod, FREEZE_PERIOD);
    assert.equal(
      configAccount.rewardRatePerSec.toString(),
      REWARD_RATE_PER_SEC.toString(),
    );
  });

  it("rejects initialize when reward rate is not a whole token per second", async () => {
    const otherCollection = anchor.web3.Keypair.generate();
    const otherUpdateAuthority = updateAuthorityPda(
      otherCollection.publicKey,
      program.programId,
    );
    const otherConfig = configPda(otherCollection.publicKey, program.programId);
    const otherRewardsMint = rewardsMintPda(
      otherCollection.publicKey,
      program.programId,
    );

    await program.methods
      .createCollection("Other Collection", "https://example.com/other")
      .accountsStrict({
        payer: provider.wallet.publicKey,
        collection: otherCollection.publicKey,
        updateAuthority: otherUpdateAuthority,
        systemProgram: anchor.web3.SystemProgram.programId,
        mplProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([otherCollection])
      .rpc();

    await expectProgramError(
      program.methods
        .initialize(new anchor.BN(500_000), FREEZE_PERIOD)
        .accountsStrict({
          payer: provider.wallet.publicKey,
          config: otherConfig,
          rewardsMint: otherRewardsMint,
          collection: otherCollection.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc(),
      "InvalidRewardRate",
    );
  });

  it("rejects staking an asset that is already staked", async () => {
    const ctx = await setupInitializedCollection();
    const asset = await mintAssetInCollection(ctx);

    await stakeAsset(ctx, asset);

    await expectProgramError(
      program.methods
        .stake()
        .accountsStrict(stakeAccounts(ctx, asset.publicKey))
        .rpc(),
      "AlreadyStaked",
    );
  });

  it("rejects claim and unstake when the asset has never been staked", async () => {
    const ctx = await setupInitializedCollection();
    const asset = await mintAssetInCollection(ctx);

    await expectProgramError(
      program.methods
        .claimRewards()
        .accountsStrict(claimAccounts(ctx, asset.publicKey))
        .rpc(),
      "AttributesPluginNotFound",
    );

    await expectProgramError(
      program.methods
        .unstake()
        .accountsStrict(claimAccounts(ctx, asset.publicKey))
        .rpc(),
      "AttributesPluginNotFound",
    );
  });

  it("rejects claim when no rewards have accrued yet", async () => {
    const ctx = await setupInitializedCollection();
    const asset = await mintAssetInCollection(ctx);

    await stakeAsset(ctx, asset);

    await expectProgramError(
      program.methods
        .claimRewards()
        .accountsStrict(claimAccounts(ctx, asset.publicKey))
        .rpc(),
      "NoRewardsToClaim",
    );
  });

  it("rejects claim and unstake before the freeze period elapses", async () => {
    const freezePeriod = 10;
    const ctx = await setupInitializedCollection(freezePeriod);
    const asset = await mintAssetInCollection(ctx);

    const { stakedAt } = await stakeAsset(ctx, asset);
    await surfpoolTimeTravel(
      provider.connection,
      stakedAt + freezePeriod - 1,
    );

    await expectProgramError(
      program.methods
        .claimRewards()
        .accountsStrict(claimAccounts(ctx, asset.publicKey))
        .rpc(),
      "FreezePeriodNotElapsed",
    );

    await expectProgramError(
      program.methods
        .unstake()
        .accountsStrict(claimAccounts(ctx, asset.publicKey))
        .rpc(),
      "FreezePeriodNotElapsed",
    );
  });

  it("accrues rewards across multiple claims without double-paying", async () => {
    const ctx = await setupInitializedCollection();
    const asset = await mintAssetInCollection(ctx);

    const { stakedAt } = await stakeAsset(ctx, asset);

    await surfpoolTimeTravel(provider.connection, stakedAt + 3);
    await program.methods
      .claimRewards()
      .accountsStrict(claimAccounts(ctx, asset.publicKey))
      .rpc();

    const balanceAfterFirstClaim =
      await provider.connection.getTokenAccountBalance(
        ctx.userRewardsTokenAccount,
      );
    assert.equal(
      balanceAfterFirstClaim.value.amount,
      REWARD_RATE_PER_SEC.muln(3).toString(),
    );

    await surfpoolTimeTravel(provider.connection, stakedAt + 5);
    await program.methods
      .claimRewards()
      .accountsStrict(claimAccounts(ctx, asset.publicKey))
      .rpc();

    const balanceAfterSecondClaim =
      await provider.connection.getTokenAccountBalance(
        ctx.userRewardsTokenAccount,
      );
    assert.equal(
      balanceAfterSecondClaim.value.amount,
      REWARD_RATE_PER_SEC.muln(5).toString(),
      "second claim should only mint rewards since last claim",
    );
  });

  it("unstake mints pending rewards without a separate claim", async () => {
    const ctx = await setupInitializedCollection();
    const asset = await mintAssetInCollection(ctx);

    const { stakedAt } = await stakeAsset(ctx, asset);
    await surfpoolTimeTravel(
      provider.connection,
      stakedAt + REWARD_ACCRUAL_SECONDS,
    );

    await program.methods
      .unstake()
      .accountsStrict(claimAccounts(ctx, asset.publicKey))
      .rpc();

    const balanceAfterUnstake = await provider.connection.getTokenAccountBalance(
      ctx.userRewardsTokenAccount,
    );
    assert.equal(
      balanceAfterUnstake.value.amount,
      REWARD_RATE_PER_SEC.muln(REWARD_ACCRUAL_SECONDS).toString(),
      "unstake should auto-claim accrued rewards",
    );
  });
});
