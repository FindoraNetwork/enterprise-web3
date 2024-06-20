mod data;
mod utils;

use {
    super::PrecompileResult,
    crate::vm::precompile::REDIS_POOL,
    data::{Address, EvmDataReader, EvmDataWriter},
    ethereum_types::{H160, U256},
    evm::{
        executor::stack::{PrecompileFailure, PrecompileOutput},
        Context, ExitSucceed,
    },
    evm_exporter::{ConnectionType, Getter, RedisGetter, PREFIX},
    evm_runtime::ExitError,
    log::debug,
    slices::u8_slice,
    std::{borrow::Cow, collections::BTreeMap},
    utils::{error, EvmResult, Gasometer, LogsBuilder},
};

/// FRC20 transfer event selector, Keccak256("Transfer(address,address,uint256)")
///
/// event Transfer(address indexed from, address indexed to, uint256 value);
pub const TRANSFER_EVENT_SELECTOR: &[u8; 32] =
    u8_slice!("0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef");

/// FRC20 approval event selector, Keccak256("Approval(address,address,uint256)")
///
/// event Approval(address indexed owner, address indexed spender, uint256 value);
pub const APPROVAL_EVENT_SELECTOR: &[u8; 32] =
    u8_slice!("0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925");

/// b"Findora"
pub const FRC20_NAME: &[u8; 96] = u8_slice!(
    "0x00000000000000000000000000000000000000000000000000000000000000200000000000000\
    00000000000000000000000000000000000000000000000000746696e646f7261000000000000000\
    00000000000000000000000000000000000"
);

/// b"FRA"
pub const FRC20_SYMBOL: &[u8; 96] = u8_slice!(
    "0x00000000000000000000000000000000000000000000000000000000000000200000000000000\
    00000000000000000000000000000000000000000000000000346524100000000000000000000000\
    00000000000000000000000000000000000"
);

// The gas used value is obtained according to the standard erc20 call.
// https://github.com/OpenZeppelin/openzeppelin-contracts/blob/v4.3.2/contracts/token/ERC20/ERC20.sol
const GAS_NAME: u64 = 3283;
const GAS_SYMBOL: u64 = 3437;
const GAS_DECIMALS: u64 = 243;
const GAS_TOTAL_SUPPLY: u64 = 1003;
const GAS_BALANCE_OF: u64 = 1350;
const GAS_TRANSFER: u64 = 23661;
const GAS_ALLOWANCE: u64 = 1624;
const GAS_APPROVE: u64 = 20750;
const GAS_TRANSFER_FROM: u64 = 6610;

pub struct FRC20 {
    height: u32,
    allowance: BTreeMap<(H160, H160), U256>,
    balance: BTreeMap<H160, U256>,
}

#[precompile_utils_macro::generate_function_selector]
#[derive(Debug, PartialEq, Eq, num_enum::TryFromPrimitive, num_enum::IntoPrimitive)]
pub enum Call {
    Name = "name()",
    Symbol = "symbol()",
    Decimals = "decimals()",
    TotalSupply = "totalSupply()",
    BalanceOf = "balanceOf(address)",
    Transfer = "transfer(address,uint256)",
    Allowance = "allowance(address,address)",
    Approve = "approve(address,uint256)",
    TransferFrom = "transferFrom(address,address,uint256)",
}

impl FRC20 {
    pub fn new(height: u32) -> Self {
        Self {
            height,
            allowance: BTreeMap::new(),
            balance: BTreeMap::new(),
        }
    }
    pub fn contract_id() -> u64 {
        0x1000
    }
    fn get_balance(&self, addr: H160) -> EvmResult<U256> {
        let conn = REDIS_POOL
            .get()
            .ok_or_else(|| ExitError::Other(Cow::from("REDIS_POOL get error")))
            .and_then(|redis_pool| {
                redis_pool.get_connection().map_err(|e| {
                    ExitError::Other(Cow::from(format!("redis get connect error:{:?}", e)))
                })
            })?;

        let mut getter: RedisGetter = Getter::new(ConnectionType::Redis(conn), PREFIX.to_string());
        let amount = getter
            .get_balance(self.height, addr)
            .map_err(|e| ExitError::Other(Cow::from(format!("redis get value error:{:?}", e))))?;

        let amount = self.balance.get(&addr).cloned().unwrap_or(amount);
        Ok(amount)
    }

    fn get_allowance(&self, owner: H160, spender: H160) -> EvmResult<U256> {
        let conn = REDIS_POOL
            .get()
            .ok_or_else(|| ExitError::Other(Cow::from("REDIS_POOL get error")))
            .and_then(|redis_pool| {
                redis_pool.get_connection().map_err(|e| {
                    ExitError::Other(Cow::from(format!("redis get connect error:{:?}", e)))
                })
            })?;

        let mut getter: RedisGetter = Getter::new(ConnectionType::Redis(conn), PREFIX.to_string());
        let amount = getter
            .get_allowances(self.height, owner, spender)
            .map_err(|e| ExitError::Other(Cow::from(format!("redis get value error:{:?}", e))))?;

        let amount = self
            .allowance
            .get(&(owner, spender))
            .cloned()
            .unwrap_or(amount);
        Ok(amount)
    }
}

impl FRC20 {
    pub fn execute(
        &mut self,
        input: &[u8],
        gas_limit: Option<u64>,
        context: &Context,
        _is_static: bool,
    ) -> PrecompileResult {
        let addr = context.address;
        if addr != H160::from_low_u64_be(Self::contract_id()) {
            return Err(PrecompileFailure::Error {
                exit_status: error("No delegatecall support"),
            });
        }

        let mut input = EvmDataReader::new(input);
        let selector = match input.read_selector::<Call>() {
            Ok(v) => v,
            Err(e) => {
                return Err(PrecompileFailure::Error { exit_status: e });
            }
        };

        match &selector {
            Call::Name => match Self::name(input, gas_limit) {
                Ok(v) => Ok(v),
                Err(e) => Err(PrecompileFailure::Error { exit_status: e }),
            },
            Call::Symbol => match Self::symbol(input, gas_limit) {
                Ok(v) => Ok(v),
                Err(e) => Err(PrecompileFailure::Error { exit_status: e }),
            },
            Call::Decimals => match Self::decimals(input, gas_limit) {
                Ok(v) => Ok(v),
                Err(e) => Err(PrecompileFailure::Error { exit_status: e }),
            },
            Call::TotalSupply => match self.total_supply(input, gas_limit) {
                Ok(v) => Ok(v),
                Err(e) => Err(PrecompileFailure::Error { exit_status: e }),
            },
            Call::BalanceOf => match self.balance_of(input, gas_limit) {
                Ok(v) => Ok(v),
                Err(e) => Err(PrecompileFailure::Error { exit_status: e }),
            },
            Call::Allowance => match self.allowance(input, gas_limit) {
                Ok(v) => Ok(v),
                Err(e) => Err(PrecompileFailure::Error { exit_status: e }),
            },
            Call::Approve => match self.approve(input, gas_limit, context) {
                Ok(v) => Ok(v),
                Err(e) => Err(PrecompileFailure::Error { exit_status: e }),
            },
            Call::Transfer => match self.transfer(input, gas_limit, context) {
                Ok(v) => Ok(v),
                Err(e) => Err(PrecompileFailure::Error { exit_status: e }),
            },
            Call::TransferFrom => match self.transfer_from(input, gas_limit, context) {
                Ok(v) => Ok(v),
                Err(e) => Err(PrecompileFailure::Error { exit_status: e }),
            },
        }
    }
}

impl FRC20 {
    /// Returns the name of the token.
    fn name(input: EvmDataReader, gas_limit: Option<u64>) -> EvmResult<PrecompileOutput> {
        let mut gasometer = Gasometer::new(gas_limit);
        gasometer.record_cost(GAS_NAME)?;

        input.expect_arguments(0)?;

        debug!(target: "evm", "FRC20#name: Findora");

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output: EvmDataWriter::new().write_raw_bytes(FRC20_NAME).build(),
            logs: vec![],
        })
    }

    /// Returns the symbol of the token, usually a shorter version of the name.
    fn symbol(input: EvmDataReader, gas_limit: Option<u64>) -> EvmResult<PrecompileOutput> {
        let mut gasometer = Gasometer::new(gas_limit);
        gasometer.record_cost(GAS_SYMBOL)?;

        input.expect_arguments(0)?;

        debug!(target: "evm", "FRC20#symbol: FRA");

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output: EvmDataWriter::new().write_raw_bytes(FRC20_SYMBOL).build(),
            logs: vec![],
        })
    }

    /// Returns the number of decimals used to get its user representation.
    /// Tokens usually opt for a value of 18.
    fn decimals(input: EvmDataReader, gas_limit: Option<u64>) -> EvmResult<PrecompileOutput> {
        let mut gasometer = Gasometer::new(gas_limit);
        gasometer.record_cost(GAS_DECIMALS)?;

        input.expect_arguments(0)?;

        debug!(target: "evm", "FRC20#decimals: 18");

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output: EvmDataWriter::new().write(18_u8).build(),
            logs: vec![],
        })
    }

    /// Returns the amount of tokens in existence.
    fn total_supply(
        &self,
        input: EvmDataReader,
        gas_limit: Option<u64>,
    ) -> EvmResult<PrecompileOutput> {
        let mut gasometer = Gasometer::new(gas_limit);
        gasometer.record_cost(GAS_TOTAL_SUPPLY)?;

        input.expect_arguments(0)?;
        let conn = REDIS_POOL
            .get()
            .ok_or_else(|| ExitError::Other(Cow::from("REDIS_POOL get error")))
            .and_then(|redis_pool| {
                redis_pool.get_connection().map_err(|e| {
                    ExitError::Other(Cow::from(format!("redis get connect error:{:?}", e)))
                })
            })?;

        let mut getter: RedisGetter = Getter::new(ConnectionType::Redis(conn), PREFIX.to_string());
        let amount: U256 = getter
            .get_total_issuance(self.height)
            .map_err(|e| ExitError::Other(Cow::from(format!("redis get value error:{:?}", e))))?;
        debug!(target: "evm", "FRC20#total_supply: {:?}", amount);

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output: EvmDataWriter::new().write(amount).build(),
            logs: vec![],
        })
    }

    /// Returns the amount of tokens owned by `owner`.
    fn balance_of(
        &self,
        mut input: EvmDataReader,
        gas_limit: Option<u64>,
    ) -> EvmResult<PrecompileOutput> {
        let mut gasometer = Gasometer::new(gas_limit);
        gasometer.record_cost(GAS_BALANCE_OF)?;

        input.expect_arguments(1)?;

        let owner: H160 = input.read::<Address>()?.into();
        let amount = self.get_balance(owner)?;

        debug!(target: "evm", "FRC20#balance_of: owner: {:?}, amount: {:?} ", owner, amount);

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output: EvmDataWriter::new().write(amount).build(),
            logs: vec![],
        })
    }

    /// Returns the remaining number of tokens that `spender` will be allowed to spend on behalf
    /// of `owner` through {transferFrom}. This is zero by default.
    fn allowance(
        &self,
        mut input: EvmDataReader,
        gas_limit: Option<u64>,
    ) -> EvmResult<PrecompileOutput> {
        let mut gasometer = Gasometer::new(gas_limit);
        gasometer.record_cost(GAS_ALLOWANCE)?;

        input.expect_arguments(2)?;

        let owner: H160 = input.read::<Address>()?.into();
        let spender: H160 = input.read::<Address>()?.into();
        let amount = self.get_allowance(owner, spender)?;
        debug!(target: "evm",
            "FRC20#allowance: owner: {:?}, spender: {:?}, allowance: {:?}",
            owner, spender, amount
        );

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output: EvmDataWriter::new().write(amount).build(),
            logs: vec![],
        })
    }

    /// Sets `amount` as the allowance of `spender` over the caller's tokens.
    ///
    /// Returns a boolean value indicating whether the operation succeeded.
    fn approve(
        &mut self,
        mut input: EvmDataReader,
        gas_limit: Option<u64>,
        context: &Context,
    ) -> EvmResult<PrecompileOutput> {
        let mut gasometer = Gasometer::new(gas_limit);
        gasometer.record_cost(GAS_APPROVE)?;
        gasometer.record_log_costs_manual(3, 32)?;

        input.expect_arguments(2)?;

        let caller = context.caller;
        let spender: H160 = input.read::<Address>()?.into();
        if spender == H160::zero() {
            return Err(error("FRC20: approve to the zero address"));
        }

        let amount: U256 = input.read()?;
        debug!(target: "evm",
            "FRC20#approve: sender: {:?}, spender: {:?}, amount: {:?}",
            context.caller, spender, amount
        );
        self.allowance.insert((caller, spender), amount);

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output: EvmDataWriter::new().write(true).build(),
            logs: LogsBuilder::new(context.address)
                .log3(
                    APPROVAL_EVENT_SELECTOR,
                    context.caller,
                    spender,
                    EvmDataWriter::new().write(amount).build(),
                )
                .build(),
        })
    }

    /// Moves `amount` tokens from the caller's account to `recipient`.
    ///
    /// Returns a boolean value indicating whether the operation succeeded.
    fn transfer(
        &mut self,
        mut input: EvmDataReader,
        target_gas: Option<u64>,
        context: &Context,
    ) -> EvmResult<PrecompileOutput> {
        let mut gasometer = Gasometer::new(target_gas);
        gasometer.record_cost(GAS_TRANSFER)?;
        gasometer.record_log_costs_manual(3, 32)?;

        input.expect_arguments(2)?;

        let caller = context.caller;
        let recipient: H160 = input.read::<Address>()?.into();
        if recipient == H160::zero() {
            return Err(error("FRC20: transfer to the zero address"));
        }
        let amount: U256 = input.read()?;
        debug!(target: "evm",
            "FRC20#transfer: sender: {:?}, to: {:?}, amount: {:?}",
            context.caller, recipient, amount
        );
        let caller_balance = self.get_balance(caller)?;
        let recipient_balance = self.get_balance(recipient)?;

        self.balance
            .insert(caller, caller_balance.saturating_sub(amount));

        self.balance
            .insert(recipient, recipient_balance.saturating_add(amount));

        // C::AccountAsset::transfer(state, &caller, &recipient_id, amount)
        //     .map_err(|e| error(format!("{e:?}")))?;

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output: EvmDataWriter::new().write(true).build(),
            logs: LogsBuilder::new(context.address)
                .log3(
                    TRANSFER_EVENT_SELECTOR,
                    context.caller,
                    recipient,
                    EvmDataWriter::new().write(amount).build(),
                )
                .build(),
        })
    }

    /// Moves `amount` tokens from `sender` to `recipient` using the allowance mechanism.
    /// `amount` is then deducted from the caller's allowance.
    ///
    /// Returns a boolean value indicating whether the operation succeeded.
    fn transfer_from(
        &mut self,
        mut input: EvmDataReader,
        target_gas: Option<u64>,
        context: &Context,
    ) -> EvmResult<PrecompileOutput> {
        let mut gasometer = Gasometer::new(target_gas);
        gasometer.record_cost(GAS_TRANSFER_FROM)?;
        gasometer.record_log_costs_manual(3, 32)?;
        gasometer.record_log_costs_manual(3, 32)?;

        input.expect_arguments(3)?;

        let caller = context.caller;
        let from: H160 = input.read::<Address>()?.into();
        if from == H160::zero() {
            return Err(error("FRC20: transfer from the zero address"));
        }
        let recipient: H160 = input.read::<Address>()?.into();
        if recipient == H160::zero() {
            return Err(error("FRC20: transfer to the zero address"));
        }
        let amount: U256 = input.read()?;

        let allowance = self.get_allowance(from, caller)?;
        if allowance < amount {
            return Err(error("FRC20: transfer amount exceeds allowance"));
        }
        debug!(target: "evm",
            "FRC20#transfer_from: sender: {:?}, from: {:?}, to: {:?}, amount: {:?}",
            context.caller, from, recipient, amount
        );
        let from_balance = self.get_balance(from)?;
        let recipient_balance = self.get_balance(recipient)?;

        self.balance
            .insert(from, from_balance.saturating_sub(amount));

        self.balance
            .insert(recipient, recipient_balance.saturating_add(amount));

        self.allowance
            .insert((from, caller), allowance.saturating_sub(amount));

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output: EvmDataWriter::new().write(true).build(),
            logs: LogsBuilder::new(context.address)
                .log3(
                    TRANSFER_EVENT_SELECTOR,
                    from,
                    recipient,
                    EvmDataWriter::new().write(amount).build(),
                )
                .log3(
                    APPROVAL_EVENT_SELECTOR,
                    from,
                    context.caller,
                    EvmDataWriter::new()
                        .write(allowance.saturating_sub(amount))
                        .build(),
                )
                .build(),
        })
    }
}
