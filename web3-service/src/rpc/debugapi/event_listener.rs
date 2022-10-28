use evm::{Capture, ExitReason};

use {
    super::types::RawStepLog,
    ethereum_types::{H160, H256},
    evm::Opcode,
    evm_runtime::tracing::{Event as RuntimeEvent, EventListener},
    std::collections::BTreeMap,
};

#[derive(Debug)]
struct Step {
    opcode: Opcode,
    depth: usize,
    gas: u64,
    gas_cost: u64,
    position: usize,
    memory: Option<Vec<u8>>,
    stack: Option<Vec<H256>>,
}

#[derive(Debug)]
struct Context {
    storage_cache: BTreeMap<H256, H256>,
    address: H160,
    current_step: Option<Step>,
    global_storage_changes: BTreeMap<H160, BTreeMap<H256, H256>>,
}
pub struct DebugEventListener {
    disable_storage: bool,
    disable_memory: bool,
    disable_stack: bool,

    new_context: bool,
    context_stack: Vec<Context>,

    pub step_logs: Vec<RawStepLog>,
    pub return_value: Vec<u8>,
}
impl DebugEventListener {
    pub fn new(disable_storage: bool, disable_memory: bool, disable_stack: bool) -> Self {
        Self {
            disable_storage,
            disable_memory,
            disable_stack,

            step_logs: vec![],
            return_value: vec![],

            new_context: true,
            context_stack: vec![],
        }
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
                    Err(Capture::Exit(reason)) => {
                        if let Some(mut context) = self.context_stack.pop() {
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

pub enum CallType {
    Call,
    CallCode,
    DelegateCall,
    StaticCall,
}

pub enum ContextType {
    Call(CallType),
    Create,
}

impl ContextType {
    pub fn from(opcode: Opcode) -> Option<Self> {
        match opcode {
            Opcode::CREATE | Opcode::CREATE2 => Some(ContextType::Create),
            Opcode::CALL => Some(ContextType::Call(CallType::Call)),
            Opcode::CALLCODE => Some(ContextType::Call(CallType::CallCode)),
            Opcode::DELEGATECALL => Some(ContextType::Call(CallType::DelegateCall)),
            Opcode::STATICCALL => Some(ContextType::Call(CallType::StaticCall)),
            _ => None,
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
