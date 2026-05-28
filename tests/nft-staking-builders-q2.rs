use anchor_litesvm::{AnchorContext, AnchorLiteSVM};
use litesvm_utils::TestHelpers;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL, pubkey, signature::{Keypair, read_keypair_file}, signer::Signer
};

anchor_lang::declare_program!(nft_staking_builders_q2);

const MPL_CORE_PROGRAM_ID: pubkey::Pubkey = pubkey!("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");

fn setup_smv() -> AnchorContext {
    let program_keypair = read_keypair_file("target/deploy/nft_staking_builders_q2-keypair.json")
        .expect("Failed to read program keypair");

    AnchorLiteSVM::build_with_program(
        program_keypair.pubkey(),
        include_bytes!("../target/deploy/nft_staking_builders_q2.so"),
    )
}

#[test]
fn create_collection() {
    let mut ctx = setup_smv();

    let user = ctx.svm.create_funded_account(LAMPORTS_PER_SOL * 100).unwrap();
    let collection = Keypair::new();
    let update_authority = ctx.svm.get_pda(
        &[b"update_authority", collection.pubkey().as_ref()],
        &ctx.program_id,
    );

    let ix = ctx
        .program()
        .accounts(
            nft_staking_builders_q2::client::accounts::CreateCollection {
                payer: user.pubkey(),
                collection: collection.pubkey(),
                update_authority,
                system_program: anchor_lang::system_program::ID,
                mpl_program: MPL_CORE_PROGRAM_ID,
            },
        )
        .args(nft_staking_builders_q2::client::args::CreateCollection {
            collection_name: "Test Collection".to_string(),
            collection_uri: "https://test.com".to_string(),
        })
        .instruction()
        .unwrap();

    let res = ctx
        .execute_instruction(ix, &[&user, &collection])
        .unwrap();
    res.print_logs();
}
