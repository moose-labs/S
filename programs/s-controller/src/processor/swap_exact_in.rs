use s_controller_interface::{SControllerError, SwapExactInIxArgs};
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult};

pub fn process_swap_exact_in(_accounts: &[AccountInfo], _args: SwapExactInIxArgs) -> ProgramResult {
    Err(SControllerError::FeatureNotSupported.into())
}
