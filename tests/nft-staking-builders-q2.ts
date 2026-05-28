import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { NftStakingBuildersQ2 } from "../target/types/nft_staking_builders_q2";

describe("nft-staking-builders-q2", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const MPL_CORE_PROGRAM_ID = new anchor.web3.PublicKey("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");

  const program = anchor.workspace
    .nftStakingBuildersQ2 as Program<NftStakingBuildersQ2>;

  it("Create a collection", async () => {
    const collection = anchor.web3.Keypair.generate();
    const updateAuthority = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("update_authority"), collection.publicKey.toBuffer()],
      program.programId,
    );

    const tx = await program.methods
      .createCollection("test", "https://test.com")
      .accountsStrict({
        payer: provider.wallet.publicKey,
        collection: collection.publicKey,
        updateAuthority: updateAuthority[0],
        systemProgram: anchor.web3.SystemProgram.programId,
        mplProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([provider.wallet.payer, collection])
      .rpc();
    console.log("Your transaction signature", tx);
  });
});
