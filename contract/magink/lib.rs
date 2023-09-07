#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub use self::magink::MaginkRef;

#[ink::contract]
pub mod magink {
    use crate::ensure;
    use ink::{
        env::{
            call::{build_call, ExecutionInput, Selector},
            DefaultEnvironment,
        },
        prelude::string::String,
        storage::Mapping,
    };
    use openbrush::contracts::psp34::Id;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        TooEarlyToClaim,
        UserNotFound,
        NotEnoughBadges,
        NftAlreadyClaimed,
        MintError,
    }

    #[ink(storage)]
    pub struct Magink {
        user: Mapping<AccountId, Profile>,
        wizard_account: AccountId,
    }
    #[derive(
        Debug, PartialEq, Eq, PartialOrd, Ord, Clone, scale::Encode, scale::Decode,
    )]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Profile {
        // duration in blocks until next claim
        claim_era: u8,
        // block number of last claim
        start_block: u32,
        // number of badges claimed
        badges_claimed: u8,
    }

    impl Magink {
        /// Creates a new Magink smart contract.
        #[ink(constructor)]
        pub fn new(wizard_account: AccountId) -> Self {
            Self {
                user: Mapping::new(),
                wizard_account,
            }
        }

        /// (Re)Start the Magink the claiming era for the caller.
        #[ink(message)]
        pub fn start(&mut self, era: u8) {
            let profile = Profile {
                claim_era: era,
                start_block: self.env().block_number(),
                badges_claimed: 0,
            };
            self.user.insert(self.env().caller(), &profile);
        }

        /// Claim the badge after the era.
        #[ink(message)]
        pub fn claim(&mut self) -> Result<(), Error> {
            ensure!(self.get_remaining() == 0, Error::TooEarlyToClaim);

            // update profile
            let mut profile = self.get_profile().ok_or(Error::UserNotFound)?;
            profile.badges_claimed += 1;
            profile.start_block = self.env().block_number();
            self.user.insert(self.env().caller(), &profile);

            Ok(())
        }

        #[ink(message)]
        pub fn mint_wizard(&mut self) -> Result<(), Error> {
            let user = self.get_profile().ok_or(Error::UserNotFound)?;

            if user.badges_claimed < 9 {
                return Err(Error::NotEnoughBadges);
            }

            if self.get_is_already_minted(self.env().caller()) {
                return Err(Error::NftAlreadyClaimed);
            }

            let next_id = self.get_next_id();

            let mint_result = build_call::<DefaultEnvironment>()
                .call(self.wizard_account)
                .gas_limit(5000000000)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!(
                        "PSP34Mintable::mint"
                    )))
                    .push_arg(self.env().caller())
                    .push_arg(Id::U128(next_id)),
                )
                .returns::<()>()
                .try_invoke()
                .expect("Mint call error");

            if mint_result.is_err() {
                return Err(Error::MintError);
            }

            Ok(())
        }

        /// Returns the remaining blocks in the era.
        #[ink(message)]
        pub fn get_remaining(&self) -> u8 {
            let current_block = self.env().block_number();
            let caller = self.env().caller();
            self.user.get(&caller).map_or(0, |profile| {
                if current_block - profile.start_block >= profile.claim_era as u32 {
                    return 0;
                }
                profile.claim_era - (current_block - profile.start_block) as u8
            })
        }

        /// Returns the remaining blocks in the era for the given account.
        #[ink(message)]
        pub fn get_remaining_for(&self, account: AccountId) -> u8 {
            let current_block = self.env().block_number();
            self.user.get(&account).map_or(0, |profile| {
                if current_block - profile.start_block >= profile.claim_era as u32 {
                    return 0;
                }
                profile.claim_era - (current_block - profile.start_block) as u8
            })
        }

        /// Returns the profile of the given account.
        #[ink(message)]
        pub fn get_account_profile(&self, account: AccountId) -> Option<Profile> {
            self.user.get(&account)
        }

        /// Returns the profile of the caller.
        #[ink(message)]
        pub fn get_profile(&self) -> Option<Profile> {
            let caller = self.env().caller();
            self.user.get(&caller)
        }

        /// Returns the badge of the caller.
        #[ink(message)]
        pub fn get_badges(&self) -> u8 {
            self.get_profile()
                .map_or(0, |profile| profile.badges_claimed)
        }

        /// Returns the badge count of the given account.
        #[ink(message)]
        pub fn get_badges_for(&self, account: AccountId) -> u8 {
            self.get_account_profile(account)
                .map_or(0, |profile| profile.badges_claimed)
        }

        /// Use token total supply for the next id
        #[ink(message)]
        pub fn get_next_id(&self) -> u128 {
            build_call::<DefaultEnvironment>()
                .call(self.wizard_account)
                .gas_limit(5000000000)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!(
                        "PSP34::total_supply"
                    )))
                    .push_arg(self.env().caller()),
                )
                .returns::<u128>()
                .try_invoke()
                .expect("total_supply call error")
                .expect("total_supply error")
        }

        /// For frontend access
        #[ink(message)]
        pub fn get_is_already_minted(&self, account: AccountId) -> bool {
            let balance_result = build_call::<DefaultEnvironment>()
                .call(self.wizard_account)
                .gas_limit(5000000000)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!(
                        "PSP34::balance_of"
                    )))
                    .push_arg(account),
                )
                .returns::<u32>()
                .try_invoke()
                .expect("balance_of call error");

            balance_result.expect("balance_of error") >= 1
        }

        /// For frontend access
        #[ink(message)]
        pub fn get_token_image(&self) -> String {
            let collection_id =
                Id::Bytes(<_ as AsRef<[u8; 32]>>::as_ref(&self.wizard_account).to_vec());

            build_call::<DefaultEnvironment>()
                .call(self.wizard_account)
                .gas_limit(5000000000)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!(
                        "PSP34Metadata::get_attribute"
                    )))
                    .push_arg(collection_id)
                    .push_arg(String::from("image")),
                )
                .returns::<Option<String>>()
                .try_invoke()
                .expect("get_attribute call error")
                .expect("get_attribute error")
                .expect("no metadata")
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn start_works() {
            let mut magink = Magink::new(AccountId::from([0x42; 32]));
            println!("get {:?}", magink.get_remaining());
            magink.start(10);
            assert_eq!(10, magink.get_remaining());
            advance_block();
            assert_eq!(9, magink.get_remaining());
        }

        #[ink::test]
        fn claim_works() {
            const ERA: u32 = 10;
            let accounts = default_accounts();
            let mut magink = Magink::new(AccountId::from([0x42; 32]));
            magink.start(ERA as u8);
            advance_n_blocks(ERA - 1);
            assert_eq!(1, magink.get_remaining());

            // claim fails, too early
            assert_eq!(Err(Error::TooEarlyToClaim), magink.claim());

            // claim succeeds
            advance_block();
            assert_eq!(Ok(()), magink.claim());
            assert_eq!(1, magink.get_badges());
            assert_eq!(1, magink.get_badges_for(accounts.alice));
            assert_eq!(1, magink.get_badges());
            assert_eq!(10, magink.get_remaining());

            // claim fails, too early
            assert_eq!(Err(Error::TooEarlyToClaim), magink.claim());
            advance_block();
            assert_eq!(9, magink.get_remaining());
            assert_eq!(Err(Error::TooEarlyToClaim), magink.claim());
        }

        fn default_accounts(
        ) -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<Environment>()
        }

        // fn set_sender(sender: AccountId) {
        //     ink::env::test::set_caller::<Environment>(sender);
        // }
        fn advance_n_blocks(n: u32) {
            for _ in 0..n {
                advance_block();
            }
        }
        fn advance_block() {
            ink::env::test::advance_block::<ink::env::DefaultEnvironment>();
        }
    }

    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        use super::*;
        use ink_e2e::build_message;
        use openbrush::contracts::ownable::ownable_external::Ownable;
        use wizard::WizardRef;

        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        #[ink_e2e::test(additional_contracts = "magink/Cargo.toml wizard/Cargo.toml")]
        async fn e2e_minting_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            // Upload and instantiate Wizard contract
            let wizard_constructor = WizardRef::new();
            let wizard_account = client
                .instantiate("wizard", &ink_e2e::alice(), wizard_constructor, 0, None)
                .await
                .expect("Wizard instantiate failed")
                .account_id;

            // Upload and instantiate Magink contract
            let magink_constructor = MaginkRef::new(wizard_account);
            let magink_account = client
                .instantiate("magink", &ink_e2e::alice(), magink_constructor, 0, None)
                .await
                .expect("Magink instantiate failed")
                .account_id;

            // Set Magink contract to be the owner of Wizard contract
            let change_owner = build_message::<WizardRef>(wizard_account.clone())
                .call(|p| p.transfer_ownership(magink_account));
            client
                .call(&ink_e2e::alice(), change_owner, 0, None)
                .await
                .expect("calling `transfer_ownership` failed");

            // Verify that Magink is the Wizard contract owner
            let owner =
                build_message::<WizardRef>(wizard_account.clone()).call(|p| p.owner());
            let owner_result = client
                .call(&ink_e2e::alice(), owner, 0, None)
                .await
                .expect("calling 'owner'")
                .return_value()
                .expect("No owner found");
            assert_eq!(owner_result, magink_account);

            // Start the game for Bob
            let start_call =
                build_message::<MaginkRef>(magink_account.clone()).call(|p| p.start(0));
            client
                .call(&ink_e2e::bob(), start_call, 0, None)
                .await
                .expect("calling `start` failed");

            // Collect 9 badges
            for i in 0..9 {
                // Verify badges are being collected
                let get_badges_call = build_message::<MaginkRef>(magink_account.clone())
                    .call(|p| p.get_badges());
                let get_badges_result = client
                    .call(&ink_e2e::bob(), get_badges_call, 0, None)
                    .await
                    .expect("calling `get_badges` failed")
                    .return_value();
                assert_eq!(i, get_badges_result);

                // Check there is not enough badges to mint the NFT
                let mint_call = build_message::<MaginkRef>(magink_account.clone())
                    .call(|p| p.mint_wizard());
                let mint_result = client
                    .call_dry_run(&ink_e2e::bob(), &mint_call, 0, None)
                    .await
                    .return_value();

                assert_eq!(Err(Error::NotEnoughBadges), mint_result);

                // Claim badge
                let claim_call = build_message::<MaginkRef>(magink_account.clone())
                    .call(|p| p.claim());
                let claim_result = client
                    .call(&ink_e2e::bob(), claim_call, 0, None)
                    .await
                    .expect("calling `claim` failed")
                    .return_value();
                assert_eq!(Ok(()), claim_result);

                ink::env::test::advance_block::<ink::env::DefaultEnvironment>();
            }
            // Check that 9 badges have been collected
            let get_badges_call = build_message::<MaginkRef>(magink_account.clone())
                .call(|p| p.get_badges());
            let get_badges_result = client
                .call(&ink_e2e::bob(), get_badges_call, 0, None)
                .await
                .expect("calling `get_badges` failed")
                .return_value();
            assert_eq!(9, get_badges_result);

            // No NFT has been minted yet so the ID should be 0
            let next_id_call = build_message::<MaginkRef>(magink_account.clone())
                .call(|p| p.get_next_id());
            let next_id_result = client
                .call(&ink_e2e::bob(), next_id_call, 0, None)
                .await
                .expect("calling `get_next_id` failed")
                .return_value();
            assert_eq!(0, next_id_result);

            // NFT is now ready to be minted
            let mint_call = build_message::<MaginkRef>(magink_account.clone())
                .call(|p| p.mint_wizard());
            let mint_result = client
                .call(&ink_e2e::bob(), mint_call, 0, None)
                .await
                .expect("calling `mint_wizard` failed")
                .return_value();
            assert_eq!(Ok(()), mint_result);

            // Thus, next id should be now 1
            let next_id_call = build_message::<MaginkRef>(magink_account.clone())
                .call(|p| p.get_next_id());
            let next_id_result = client
                .call(&ink_e2e::bob(), next_id_call, 0, None)
                .await
                .expect("calling `get_next_id` failed")
                .return_value();
            assert_eq!(1, next_id_result);

            // Check the user can't collect again the NFT
            let mint_call = build_message::<MaginkRef>(magink_account.clone())
                .call(|p| p.mint_wizard());
            let mint_result = client
                .call_dry_run(&ink_e2e::bob(), &mint_call, 0, None)
                .await
                .return_value();
            assert_eq!(Err(Error::NftAlreadyClaimed), mint_result);

            // As the previous call didn't mint anything, the id should remain one
            let next_id_call = build_message::<MaginkRef>(magink_account.clone())
                .call(|p| p.get_next_id());
            let next_id_result = client
                .call(&ink_e2e::bob(), next_id_call, 0, None)
                .await
                .expect("calling `get_next_id` failed")
                .return_value();
            assert_eq!(1, next_id_result);

            // Check token image
            let token_image_call = build_message::<MaginkRef>(magink_account.clone())
                .call(|p| p.get_token_image());
            let token_image = client
                .call(&ink_e2e::bob(), token_image_call, 0, None)
                .await
                .expect("calling `get_token_image` failed")
                .return_value();
            assert_eq!(String::from("https://bafkreihgob3knpzzmhiw66grmkuq3qa2ukvdseksbaxkxuiehwkhuniyfy.ipfs.nftstorage.link"), token_image);

            Ok(())
        }
    }
}

/// Evaluate `$x:expr` and if not true return `Err($y:expr)`.
///
/// Used as `ensure!(expression_to_ensure, expression_to_return_on_false)`.
#[macro_export]
macro_rules! ensure {
    ( $x:expr, $y:expr $(,)? ) => {{
        if !$x {
            return Err($y.into());
        }
    }};
}
