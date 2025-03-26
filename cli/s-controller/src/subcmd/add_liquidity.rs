use clap::{
    builder::{StringValueParser, TypedValueParser},
    Args,
};
use s_cli_utils::{handle_tx_full, pubkey_src_to_box_dyn_signer};
use s_controller_lib::{
    add_liquidity_ix_full, find_lst_state_list_address, find_pool_state_address, try_pool_state,
    AddLiquidityByMintFreeArgs, AddLiquidityIxAmts, AddLiquidityIxFullArgs,
    AddRemoveLiquidityAccountSuffixes, AddRemoveLiquidityExtraAccounts,
};
use sanctum_solana_cli_utils::PubkeySrc;
use solana_readonly_account::sdk::KeyedAccount;
use solana_readonly_account::ReadonlyAccountData;
use solana_sdk::instruction::AccountMeta;
use spl_associated_token_account::get_associated_token_address;

use crate::lst_arg::LstArg;

use super::Subcmd;

#[derive(Args, Debug)]
#[command(long_about = "Add liquidity to the pool")]
pub struct AddLiquidityArgs {
    #[arg(
        long,
        short,
        help = "The liquidity provider wallet. Defaults to config wallet if not set."
    )]
    pub wallet: Option<String>,

    #[arg(
        help = "Mint pubkey of LST to add liquidity.",
        value_parser = StringValueParser::new().try_map(|s| LstArg::parse_arg(&s)),
    )]
    pub mint: LstArg,

    #[arg(long, help = "Amount of LST to add to the pool")]
    pub amount: u64,
}

impl AddLiquidityArgs {
    pub async fn run(args: crate::Args) {
        let Self {
            wallet,
            mint,
            amount,
        } = match args.subcmd {
            Subcmd::AddLiquidity(a) => a,
            _ => unreachable!(),
        };

        let payer = args.config.signer();
        let rpc = args.config.nonblocking_rpc_client();
        let program_id = args.program;

        let wallet_signer =
            wallet.map(|s| pubkey_src_to_box_dyn_signer(PubkeySrc::parse(&s).unwrap()));
        let wallet = wallet_signer.as_ref().unwrap_or(&payer);

        let pool_state_addr = find_pool_state_address(program_id).0;
        let lst_list_addr = find_lst_state_list_address(program_id).0;
        let mut fetched_accs = rpc
            .get_multiple_accounts(&[pool_state_addr, lst_list_addr, mint.mint()])
            .await
            .unwrap();

        let lst_mint_acc = fetched_accs.pop().unwrap().unwrap();
        let lst_list_acc = fetched_accs.pop().unwrap().unwrap();
        let pool_state_acc = fetched_accs.pop().unwrap().unwrap();

        let pool_state = try_pool_state(&pool_state_acc.data()).unwrap();

        let wallet_src_ata = get_associated_token_address(&wallet.pubkey(), &mint.mint());
        let wallet_lp_ata =
            get_associated_token_address(&payer.pubkey(), &pool_state.lp_token_mint);

        println!("wallet_src_ata: {:?}", wallet_src_ata);
        println!("wallet_lp_ata: {:?}", wallet_lp_ata);

        let (keys, lst_index, program_ids) = AddLiquidityByMintFreeArgs {
            lst_mint: KeyedAccount {
                pubkey: mint.mint(),
                account: lst_mint_acc,
            },
            src_lst_acc: wallet_src_ata,
            dst_lp_acc: wallet_lp_ata,
            pool_state: pool_state_acc,
            lst_state_list: lst_list_acc,
            signer: wallet.pubkey(),
        }
        .resolve_for_prog(program_id)
        .unwrap();

        let amts = AddLiquidityIxAmts {
            lst_amount: amount,
            min_lp_out: 0,
        };

        let lst_calculator_accounts = mint.sol_value_calculator_accounts_of().unwrap();
        let pricing_program_accounts = &[AccountMeta {
            pubkey: mint.mint().clone(),
            is_signer: false,
            is_writable: false,
        }];
        let account_suffixes = AddRemoveLiquidityAccountSuffixes {
            lst_calculator_accounts: &lst_calculator_accounts,
            pricing_program_price_lp_accounts: pricing_program_accounts,
        };

        let ix = add_liquidity_ix_full(
            keys,
            AddLiquidityIxFullArgs { lst_index, amts },
            AddRemoveLiquidityExtraAccounts::new(program_ids, account_suffixes),
        )
        .unwrap();

        handle_tx_full(
            &rpc,
            args.fee_limit_cb,
            args.send_mode,
            vec![ix],
            &[],
            &mut [wallet.as_ref()],
        )
        .await;
    }
}
