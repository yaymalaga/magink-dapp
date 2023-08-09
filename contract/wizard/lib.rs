#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[openbrush::implementation(PSP34, Ownable, PSP34Mintable, PSP34Metadata)]
#[openbrush::contract]
pub mod wizard {
    use ink::codegen::{EmitEvent, Env};
    use openbrush::traits::Storage;

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        id: Id,
    }

    #[ink(storage)]
    #[derive(Default, Storage)]
    pub struct Wizard {
        #[storage_field]
        psp34: psp34::Data,
        #[storage_field]
        ownable: ownable::Data,
        #[storage_field]
		metadata: metadata::Data
    }

    #[overrider(PSP34Mintable)]
    #[openbrush::modifiers(only_owner)]
    fn mint(&mut self, account: AccountId, id: Id) -> Result<(), PSP34Error> {
        psp34::InternalImpl::_mint_to(self, account, id)
    }

    #[overrider(psp34::Internal)]
    fn _emit_transfer_event(&self, from: Option<AccountId>, to: Option<AccountId>, id: Id) {
        self.env().emit_event(Transfer { from, to, id });
    }

    impl Wizard {
        #[ink(constructor)]
        pub fn new() -> Self {
            let mut _instance = Self::default();
            ownable::Internal::_init_with_owner(&mut _instance, Self::env().caller());
            let collection_id = psp34::PSP34Impl::collection_id(&_instance);
			metadata::Internal::_set_attribute(&mut _instance, collection_id.clone(), String::from("name"), String::from("Wizard"));
			metadata::Internal::_set_attribute(&mut _instance, collection_id, String::from("symbol"), String::from("WZD"));
            _instance
        }
    }
}
