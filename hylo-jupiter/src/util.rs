use anchor_lang::prelude::{AccountDeserialize, Pubkey};
use anyhow::{anyhow, Context, Result};
use fix::num_traits::FromPrimitive;
use fix::prelude::UFix64;
use fix::typenum::Integer;
use hylo_core::idl::tokens::TokenMint;
use jupiter_amm_interface::{
  AccountMap, ClockRef, Quote, SwapMode, SwapParams,
};
use rust_decimal::Decimal;
use solana_program_pack::{IsInitialized, Pack};

use crate::quotes::{
  token_operation::{OperationOutput, TokenOperation, TokenOperationExt},
  ProtocolState,
};

/// Computes fee percentage as `Decimal`.
///
/// # Errors
/// * Conversions
/// * Arithmetic
pub fn fee_pct_decimal<Exp>(
  fees_extracted: UFix64<Exp>,
  fee_base: UFix64<Exp>,
) -> Result<Decimal> {
  if fee_base == UFix64::new(0) {
    Ok(Decimal::ZERO)
  } else {
    Decimal::from_u64(fees_extracted.bits)
      .zip(Decimal::from_u64(fee_base.bits))
      .and_then(|(num, denom)| num.checked_div(denom))
      .context("Arithmetic error in `fee_pct_decimal`")
  }
}

/// Converts [`OperationOutput`] to Jupiter [`Quote`].
///
/// # Errors
/// * Fee decimal conversion
pub fn operation_to_quote<InExp, OutExp, FeeExp>(
  op: OperationOutput<InExp, OutExp, FeeExp>,
) -> Result<Quote>
where
  InExp: Integer,
  OutExp: Integer,
  FeeExp: Integer,
{
  let fee_pct = fee_pct_decimal(op.fee_amount, op.fee_base)?;
  Ok(Quote {
    in_amount: op.in_amount.bits,
    out_amount: op.out_amount.bits,
    fee_amount: op.fee_amount.bits,
    fee_mint: op.fee_mint,
    fee_pct,
  })
}

/// Generic Jupiter quote for any `IN -> OUT` pair.
///
/// # Errors
/// * Quote math
/// * Fee decimal conversion
pub fn quote<IN, OUT>(
  state: &ProtocolState<ClockRef>,
  amount: u64,
) -> Result<Quote>
where
  IN: TokenMint,
  OUT: TokenMint,
  ProtocolState<ClockRef>: TokenOperation<IN, OUT>,
  <ProtocolState<ClockRef> as TokenOperation<IN, OUT>>::FeeExp: Integer,
{
  let op = state.output::<IN, OUT>(UFix64::new(amount))?;
  operation_to_quote(op)
}

/// Finds and deserializes an account in Jupiter's `AccountMap`.
///
/// # Errors
/// * Account not found in map
/// * Deserialization to `A` fails
pub fn account_map_get<A: AccountDeserialize>(
  account_map: &AccountMap,
  key: &Pubkey,
) -> Result<A> {
  let account = account_map
    .get(key)
    .ok_or(anyhow!("Account not found {key}"))?;
  let mut bytes = account.data.as_slice();
  let out = A::try_deserialize(&mut bytes)?;
  Ok(out)
}

pub fn account_spl_get<A: Pack + IsInitialized>(
  account_map: &AccountMap,
  key: &Pubkey,
) -> Result<A> {
  let account = account_map
    .get(key)
    .ok_or(anyhow!("Account not found {key}"))?;
  let mut bytes = account.data.as_slice();
  let out = A::unpack(&mut bytes)?;
  Ok(out)
}

/// Validates Jupiter swap parameters for Hylo compatibility.
///
/// # Errors
/// * `ExactOut` mode
/// * Dynamic accounts
pub fn validate_swap_params<'a>(
  params: &'a SwapParams<'a, 'a>,
) -> Result<&'a SwapParams<'a, 'a>> {
  if params.swap_mode == SwapMode::ExactOut {
    Err(anyhow!("ExactOut not supported"))
  } else if params.missing_dynamic_accounts_as_default {
    Err(anyhow!("Dynamic accounts replacement not supported"))
  } else {
    Ok(params)
  }
}
