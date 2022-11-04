use {
    super::{
        jsvm::func::Func,
        types::{ContextType, RawStepLog, TransactionTrace},
    },
    chrono::{DateTime, UTC},
    ethereum_types::{H160, H256, U256},
    evm::ExitError,
    evm::{Capture, ExitReason, Opcode},
    evm_runtime::tracing::{Event as RuntimeEvent, EventListener},
    jsonrpc_core::{Error, Result, Value},
    std::cell::RefCell,
    std::collections::BTreeMap,
};

#[derive(Clone)]
struct Step {
    opcode: Opcode,
    depth: usize,
    gas: u64,
    gas_cost: u64,
    position: usize,
    memory: Option<Vec<u8>>,
    stack: Option<Vec<H256>>,
}

#[derive(Clone)]
struct Context {
    storage_cache: BTreeMap<H256, H256>,
    address: H160,
    current_step: Option<Step>,
    global_storage_changes: BTreeMap<H160, BTreeMap<H256, H256>>,
}
#[derive(Clone)]
pub struct ContractInfo {
    pub block: U256,
    pub block_hash: H256,
    pub tx_index: U256,
    pub tx_hash: H256,
    pub contract_type: String,
    pub from: H160,
    pub to: H160,
    pub gas: U256,
    pub gas_price: U256,
    pub input: Vec<u8>,
    pub value: U256,
}
pub struct DebugEventListener {
    disable_storage: bool,
    disable_memory: bool,
    disable_stack: bool,

    new_context: bool,
    context_stack: Vec<Context>,

    step_logs: Vec<RawStepLog>,
    return_value: Vec<u8>,
    pub func: Option<Func>,
    func_exec_result: Option<Error>,
    error: RefCell<Option<String>>,
    info: ContractInfo,
    height: U256,
}
impl DebugEventListener {
    pub fn new(
        disable_storage: bool,
        disable_memory: bool,
        disable_stack: bool,
        func: Option<Func>,
        info: ContractInfo,
        height: U256,
    ) -> Self {
        Self {
            disable_storage,
            disable_memory,
            disable_stack,

            step_logs: vec![],
            return_value: vec![],

            new_context: true,
            context_stack: vec![],
            func,
            func_exec_result: None,
            error: RefCell::new(None),
            info,
            height,
        }
    }

    pub fn get_result(
        &self,
        gas_used: U256,
        time: DateTime<UTC>,
        output: Vec<u8>,
    ) -> Result<Value> {
        if let Some(ref err) = self.func_exec_result {
            return Err(err.clone());
        }
        if let Some(ref func) = self.func {
            let value = func.call_result_func(
                self.info.block_hash.clone(),
                self.info.tx_index.clone(),
                self.info.tx_hash.clone(),
                self.info.contract_type.clone(),
                self.info.from.clone(),
                self.info.to.clone(),
                self.info.input.clone(),
                self.info.gas.clone(),
                self.info.gas_price.clone(),
                self.info.value.clone(),
                self.info.block.clone(),
                self.info.gas_price.saturating_sub(gas_used.clone()),
                self.return_value.clone(),
                time,
                gas_used,
                self.error
                    .borrow()
                    .as_ref()
                    .and_then(|val| Some(val.clone())),
                self.height.clone(),
            );
            println!("value:{:?}", serde_json::to_string(&value).unwrap());
            return value;
        }
        serde_json::to_value(TransactionTrace {
            gas: gas_used,
            return_value: output,
            step_logs: self.step_logs.iter().map(|val| val.clone()).collect(),
        })
        .map_err(|_| {
            let mut err = Error::internal_error();
            err.message = "serde_json::to_value error".to_owned();
            err
        })
    }
}

impl EventListener for DebugEventListener {
    fn event(&mut self, event: RuntimeEvent) {
        match event {
            RuntimeEvent::Step {
                context,
                opcode,
                position,
                stack,
                memory,
            } => {
                if self.new_context {
                    self.new_context = false;

                    self.context_stack.push(Context {
                        storage_cache: BTreeMap::new(),
                        address: context.address,
                        current_step: None,
                        global_storage_changes: BTreeMap::new(),
                    });
                }

                let depth = self.context_stack.len();

                if let Some(context) = self.context_stack.last_mut() {
                    context.current_step = Some(Step {
                        opcode,
                        depth,
                        gas: 0,
                        gas_cost: 0,
                        position: *position.as_ref().unwrap_or(&0) as usize,
                        memory: if self.disable_memory {
                            None
                        } else {
                            Some(memory.data().clone())
                        },
                        stack: if self.disable_stack {
                            None
                        } else {
                            Some(stack.data().clone())
                        },
                    });
                }
            }
            RuntimeEvent::StepResult {
                result,
                return_value,
            } => {
                let mut memory_data = Default::default();
                if let Some(context) = self.context_stack.last_mut() {
                    if let Some(current_step) = context.current_step.take() {
                        let Step {
                            opcode,
                            depth,
                            gas,
                            gas_cost,
                            position,
                            memory,
                            stack,
                        } = current_step;
                        memory_data = memory.as_ref().and_then(|val| Some(val.clone()));
                        let memory = memory.map(convert_memory);

                        let storage = if self.disable_storage {
                            None
                        } else {
                            Some(context.storage_cache.clone())
                        };

                        self.step_logs.push(RawStepLog {
                            depth: depth.into(),
                            gas: gas.into(),
                            gas_cost: gas_cost.into(),
                            memory,
                            op: opcode.0,
                            pc: position.into(),
                            stack,
                            storage,
                        });
                    }
                }
                match result {
                    Ok(_) => {
                        if let Some(ref func) = self.func {
                            self.step_logs.last().map(|log| {
                                let stack_data = log.stack.as_ref().and_then(|val| {
                                    Some(val.iter().map(|val| val.clone()).collect::<Vec<H256>>())
                                });
                                if let Err(e) = func.call_step_func(
                                    log.pc.as_u64(),
                                    log.gas.as_u64(),
                                    log.gas_cost.as_u64(),
                                    log.depth.as_u64(),
                                    Default::default(),
                                    None,
                                    Opcode(log.op),
                                    stack_data,
                                    memory_data,
                                    self.info.from.clone(),
                                    self.info.to.clone(),
                                    self.info.value.clone(),
                                    self.info.input.clone(),
                                    self.height.clone(),
                                ) {
                                    if self.func_exec_result.is_none() {
                                        self.func_exec_result = Some(e);
                                    }
                                };
                            });
                        }
                    }
                    Err(Capture::Exit(reason)) => {
                        match reason.clone() {
                            ExitReason::Error(val) => {
                                let mut err = self.error.borrow_mut();
                                if val == ExitError::OutOfGas {
                                    *err = Some("out of gas".to_string());
                                } else {
                                    *err = Some(format!("evm error: {:?}", val));
                                }
                            }
                            ExitReason::Revert(_) => {
                                let mut err = self.error.borrow_mut();

                                let mut message =
                                    "VM Exception while processing transaction: revert".to_string();

                                if return_value.len() > 68 {
                                    let message_len = return_value[36..68].iter().sum::<u8>();
                                    let body: &[u8] = &return_value[68..68 + message_len as usize];
                                    if let Ok(reason) = std::str::from_utf8(body) {
                                        message = format!("{} {}", message, reason);
                                    }
                                }

                                *err = Some(message)
                            }
                            ExitReason::Fatal(val) => {
                                let mut err = self.error.borrow_mut();
                                *err = Some(format!("evm fatal: {:?}", val))
                            }
                            _ => {}
                        }
                        if let Some(mut context) = self.context_stack.pop() {
                            if let Some(ref func) = self.func {
                                self.step_logs.last().map(|log| {
                                    let stack_data = log.stack.as_ref().and_then(|val| {
                                        Some(
                                            val.iter()
                                                .map(|val| val.clone())
                                                .collect::<Vec<H256>>(),
                                        )
                                    });
                                    if let Err(e) = func.call_fault_func(
                                        log.pc.as_u64(),
                                        log.gas.as_u64(),
                                        log.gas_cost.as_u64(),
                                        log.depth.as_u64(),
                                        Default::default(),
                                        self.error
                                            .borrow()
                                            .as_ref()
                                            .and_then(|val| Some(val.clone())),
                                        Opcode(log.op),
                                        stack_data,
                                        memory_data,
                                        self.info.from.clone(),
                                        self.info.to.clone(),
                                        self.info.value.clone(),
                                        self.info.input.clone(),
                                        self.height.clone(),
                                    ) {
                                        if self.func_exec_result.is_none() {
                                            self.func_exec_result = Some(e);
                                        }
                                    };
                                });
                            }
                            if self.context_stack.is_empty() {
                                self.return_value = return_value.to_vec();
                            }

                            if !self.disable_storage && matches!(reason, ExitReason::Succeed(_)) {
                                if let Some(parent_context) = self.context_stack.last_mut() {
                                    context
                                        .global_storage_changes
                                        .insert(context.address, context.storage_cache);

                                    for (address, mut storage) in
                                        context.global_storage_changes.into_iter()
                                    {
                                        if parent_context.address == address {
                                            for (cached_key, cached_value) in
                                                parent_context.storage_cache.iter_mut()
                                            {
                                                if let Some(value) = storage.remove(cached_key) {
                                                    *cached_value = value;
                                                }
                                            }
                                        } else {
                                            parent_context
                                                .global_storage_changes
                                                .entry(address)
                                                .or_insert_with(BTreeMap::new)
                                                .append(&mut storage);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(Capture::Trap(opcode)) if ContextType::from(*opcode).is_some() => {
                        self.new_context = true;
                    }
                    _ => (),
                }
            }
            RuntimeEvent::SLoad {
                address: _,
                index,
                value,
            }
            | RuntimeEvent::SStore {
                address: _,
                index,
                value,
            } => {
                if let Some(context) = self.context_stack.last_mut() {
                    if !self.disable_storage {
                        context.storage_cache.insert(index, value);
                    }
                }
            }
        }
    }
}

pub fn convert_memory(memory: Vec<u8>) -> Vec<H256> {
    let size = 32;
    memory
        .chunks(size)
        .map(|c| {
            let mut msg = [0u8; 32];
            let chunk = c.len();
            if chunk < size {
                let left = size - chunk;
                let remainder = vec![0; left];
                msg[0..left].copy_from_slice(&remainder[..]);
                msg[left..size].copy_from_slice(c);
            } else {
                msg[0..size].copy_from_slice(c)
            }
            H256::from_slice(&msg[..])
        })
        .collect()
}
