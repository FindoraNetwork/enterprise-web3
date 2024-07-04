use {
    crate::rpc::debugapi::types::TraceParams,
    boa_engine::{
        object::FunctionObjectBuilder, prelude::JsObject, Context, JsError, JsResult, JsString,
        JsValue, NativeFunction,
    },
    chrono::{DateTime, UTC},
    ethereum_types::{H160, H256, U256},
    evm::Opcode,
    evm_exporter::Getter,
    once_cell::sync::OnceCell,
    ruc::{eg, Result as RucResult},
    std::{str::FromStr, sync::Arc},
};

static GETTER: OnceCell<Arc<dyn Getter + Sync + Send>> = OnceCell::new();

#[inline(always)]
pub fn init_upstream(getter: Arc<dyn Getter + Sync + Send>) -> RucResult<()> {
    GETTER.set(getter).map_err(|_| eg!())
}

pub struct Cfg {
    params: TraceParams,
}
impl Cfg {
    pub fn new(params: TraceParams) -> Self {
        Self { params }
    }
    pub fn to_jsobject(&self, ctx: &mut Context) -> JsResult<JsObject> {
        let obj = JsObject::default();
        obj.set(
            "disable_storage",
            self.params.disable_storage.unwrap_or(false),
            true,
            ctx,
        )?;
        obj.set(
            "disable_memory",
            self.params.disable_memory.unwrap_or(false),
            true,
            ctx,
        )?;
        obj.set(
            "disable_stack",
            self.params.disable_stack.unwrap_or(false),
            true,
            ctx,
        )?;
        obj.set(
            "tracer",
            self.params.tracer.as_ref().cloned().unwrap_or_default(),
            true,
            ctx,
        )?;
        obj.set(
            "timeout",
            JsString::from(self.params.timeout.as_ref().cloned().unwrap_or_default()),
            true,
            ctx,
        )?;
        Ok(obj)
    }
}

pub struct Frame {
    contract_type: String,
    from: H160,
    to: H160,
    input: Vec<u8>,
    gas: U256,
    value: U256,
}
impl Frame {
    pub fn new(
        contract_type: &str,
        from: H160,
        to: H160,
        input: Vec<u8>,
        gas: U256,
        value: U256,
    ) -> Self {
        Self {
            contract_type: contract_type.to_string(),
            from,
            to,
            input,
            gas,
            value,
        }
    }
    fn get_type(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?
            .get("contract_type", ctx)
    }
    fn get_from(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?
            .get("from", ctx)
    }
    fn get_to(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?
            .get("to", ctx)
    }
    fn get_input(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?
            .get("input", ctx)
    }
    fn get_gas(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?
            .get("gas", ctx)
    }
    fn get_value(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?
            .get("value", ctx)
    }
    pub fn to_jsobject(&self, ctx: &mut Context) -> JsResult<JsObject> {
        let get_type =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_type)).build();

        let get_from =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_from)).build();

        let get_to =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_to)).build();

        let get_input =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_input)).build();

        let get_gas =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_gas)).build();

        let get_value =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_value)).build();

        let obj = JsObject::default();
        obj.set(
            "contract_type",
            JsValue::String(JsString::from(self.contract_type.as_str())),
            true,
            ctx,
        )?;
        obj.set(
            "from",
            JsValue::String(JsString::from(format!("{:?}", self.from))),
            true,
            ctx,
        )?;
        obj.set(
            "to",
            JsValue::String(JsString::from(format!("{:?}", self.to))),
            true,
            ctx,
        )?;
        obj.set(
            "input",
            JsValue::String(JsString::from(format!("0x{}", hex::encode(&self.input)))),
            true,
            ctx,
        )?;

        obj.set(
            "gas",
            JsValue::String(JsString::from(self.gas.to_string())),
            true,
            ctx,
        )?;
        obj.set(
            "value",
            JsValue::String(JsString::from(self.value.to_string())),
            true,
            ctx,
        )?;
        obj.set("getType", get_type, true, ctx)?;
        obj.set("getFrom", get_from, true, ctx)?;
        obj.set("getTo", get_to, true, ctx)?;
        obj.set("getInput", get_input, true, ctx)?;
        obj.set("getGas", get_gas, true, ctx)?;
        obj.set("getValue", get_value, true, ctx)?;
        Ok(obj)
    }
}

pub struct FrameResult {
    gas_used: U256,
    output: Vec<u8>,
    err: Option<String>,
}
impl FrameResult {
    pub fn new(gas_used: U256, output: Vec<u8>, err: Option<String>) -> Self {
        Self {
            gas_used,
            output,
            err,
        }
    }

    fn get_gas_used(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?
            .get("gas_used", ctx)
    }

    fn get_output(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?
            .get("output", ctx)
    }

    fn get_error(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?
            .get("err", ctx)
    }

    pub fn to_jsobject(&self, ctx: &mut Context) -> JsResult<JsObject> {
        let get_gas_used =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_gas_used))
                .build();

        let get_output =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_output)).build();

        let get_error =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_error)).build();

        let obj = JsObject::default();
        obj.set(
            "gas_used",
            JsValue::String(JsString::from(self.gas_used.to_string())),
            true,
            ctx,
        )?;
        obj.set(
            "output",
            JsValue::String(JsString::from(format!("0x{}", hex::encode(&self.output)))),
            true,
            ctx,
        )?;
        obj.set(
            "err",
            if let Some(ref e) = self.err {
                JsValue::String(JsString::from(e.as_str()))
            } else {
                JsValue::undefined()
            },
            true,
            ctx,
        )?;
        obj.set("getGasUsed", get_gas_used, true, ctx)?;
        obj.set("getOutput", get_output, true, ctx)?;
        obj.set("getError", get_error, true, ctx)?;
        Ok(obj)
    }
}

struct Op {
    opcode: Opcode,
}
impl Op {
    fn new(opcode: Opcode) -> Self {
        Self { opcode }
    }
    fn to_number(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?
            .get("opcode", ctx)
    }
    fn to_string(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let val = value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?;
        let opcode = Opcode(val.get("opcode", ctx)?.to_i32(ctx)? as u8);

        Ok(JsValue::String(JsString::from(Self::to_str(opcode))))
    }
    fn is_push(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let val = value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?;
        let opcode = Opcode(val.get("opcode", ctx)?.to_i32(ctx)? as u8);
        Ok(JsValue::Boolean(opcode.is_push().is_some()))
    }
    pub fn to_jsobject(&self, ctx: &mut Context) -> JsResult<JsObject> {
        let to_number =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::to_number)).build();

        let to_string =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::to_string)).build();

        let is_push =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::is_push)).build();

        let opcode = JsValue::Integer(self.opcode.as_u8() as i32);

        let obj = JsObject::default();
        obj.set("opcode", opcode, true, ctx)?;
        obj.set("toNumber", to_number, true, ctx)?;
        obj.set("toString", to_string, true, ctx)?;
        obj.set("isPush", is_push, true, ctx)?;
        Ok(obj)
    }
    fn to_str(opcode: Opcode) -> &'static str {
        match opcode {
            Opcode::STOP => "STOP",
            Opcode::ADD => "ADD",
            Opcode::MUL => "MUL",
            Opcode::SUB => "SUB",
            Opcode::DIV => "DIV",
            Opcode::SDIV => "SDIV",
            Opcode::MOD => "MOD",
            Opcode::SMOD => "SMOD",
            Opcode::ADDMOD => "ADDMOD",
            Opcode::MULMOD => "MULMOD",
            Opcode::EXP => "EXP",
            Opcode::SIGNEXTEND => "SIGNEXTEND",
            Opcode::LT => "LT",
            Opcode::GT => "GT",
            Opcode::SLT => "SLT",
            Opcode::SGT => "SGT",
            Opcode::EQ => "EQ",
            Opcode::ISZERO => "ISZERO",
            Opcode::AND => "AND",
            Opcode::OR => "OR",
            Opcode::XOR => "XOR",
            Opcode::NOT => "NOT",
            Opcode::BYTE => "BYTE",
            Opcode::CALLDATALOAD => "CALLDATALOAD",
            Opcode::CALLDATASIZE => "CALLDATASIZE",
            Opcode::CALLDATACOPY => "CALLDATACOPY",
            Opcode::CODESIZE => "CODESIZE",
            Opcode::CODECOPY => "CODECOPY",
            Opcode::SHL => "SHL",
            Opcode::SHR => "SHR",
            Opcode::SAR => "SAR",
            Opcode::POP => "POP",
            Opcode::MLOAD => "MLOAD",
            Opcode::MSTORE => "MSTORE",
            Opcode::MSTORE8 => "MSTORE8",
            Opcode::JUMP => "JUMP",
            Opcode::JUMPI => "JUMPI",
            Opcode::PC => "PC",
            Opcode::MSIZE => "MSIZE",
            Opcode::JUMPDEST => "JUMPDEST",
            Opcode::PUSH1 => "PUSH1",
            Opcode::PUSH2 => "PUSH2",
            Opcode::PUSH3 => "PUSH3",
            Opcode::PUSH4 => "PUSH4",
            Opcode::PUSH5 => "PUSH5",
            Opcode::PUSH6 => "PUSH6",
            Opcode::PUSH7 => "PUSH7",
            Opcode::PUSH8 => "PUSH8",
            Opcode::PUSH9 => "PUSH9",
            Opcode::PUSH10 => "PUSH10",
            Opcode::PUSH11 => "PUSH11",
            Opcode::PUSH12 => "PUSH12",
            Opcode::PUSH13 => "PUSH13",
            Opcode::PUSH14 => "PUSH14",
            Opcode::PUSH15 => "PUSH15",
            Opcode::PUSH16 => "PUSH16",
            Opcode::PUSH17 => "PUSH17",
            Opcode::PUSH18 => "PUSH18",
            Opcode::PUSH19 => "PUSH19",
            Opcode::PUSH20 => "PUSH20",
            Opcode::PUSH21 => "PUSH21",
            Opcode::PUSH22 => "PUSH22",
            Opcode::PUSH23 => "PUSH23",
            Opcode::PUSH24 => "PUSH24",
            Opcode::PUSH25 => "PUSH25",
            Opcode::PUSH26 => "PUSH26",
            Opcode::PUSH27 => "PUSH27",
            Opcode::PUSH28 => "PUSH28",
            Opcode::PUSH29 => "PUSH29",
            Opcode::PUSH30 => "PUSH30",
            Opcode::PUSH31 => "PUSH31",
            Opcode::PUSH32 => "PUSH32",
            Opcode::DUP1 => "DUP1",
            Opcode::DUP2 => "DUP2",
            Opcode::DUP3 => "DUP3",
            Opcode::DUP4 => "DUP4",
            Opcode::DUP5 => "DUP5",
            Opcode::DUP6 => "DUP6",
            Opcode::DUP7 => "DUP7",
            Opcode::DUP8 => "DUP8",
            Opcode::DUP9 => "DUP9",
            Opcode::DUP10 => "DUP10",
            Opcode::DUP11 => "DUP11",
            Opcode::DUP12 => "DUP12",
            Opcode::DUP13 => "DUP13",
            Opcode::DUP14 => "DUP14",
            Opcode::DUP15 => "DUP15",
            Opcode::DUP16 => "DUP16",
            Opcode::SWAP1 => "SWAP1",
            Opcode::SWAP2 => "SWAP2",
            Opcode::SWAP3 => "SWAP3",
            Opcode::SWAP4 => "SWAP4",
            Opcode::SWAP5 => "SWAP5",
            Opcode::SWAP6 => "SWAP6",
            Opcode::SWAP7 => "SWAP7",
            Opcode::SWAP8 => "SWAP8",
            Opcode::SWAP9 => "SWAP9",
            Opcode::SWAP10 => "SWAP10",
            Opcode::SWAP11 => "SWAP11",
            Opcode::SWAP12 => "SWAP12",
            Opcode::SWAP13 => "SWAP13",
            Opcode::SWAP14 => "SWAP14",
            Opcode::SWAP15 => "SWAP15",
            Opcode::SWAP16 => "SWAP16",
            Opcode::RETURN => "RETURN",
            Opcode::REVERT => "REVERT",
            Opcode::INVALID => "INVALID",
            // Opcode::EOFMAGIC => "EOFMAGIC",
            Opcode::SHA3 => "SHA3",
            Opcode::ADDRESS => "ADDRESS",
            Opcode::BALANCE => "BALANCE",
            Opcode::SELFBALANCE => "SELFBALANCE",
            Opcode::BASEFEE => "BASEFEE",
            Opcode::ORIGIN => "ORIGIN",
            Opcode::CALLER => "CALLER",
            Opcode::CALLVALUE => "CALLVALUE",
            Opcode::GASPRICE => "GASPRICE",
            Opcode::EXTCODESIZE => "EXTCODESIZE",
            Opcode::EXTCODECOPY => "EXTCODECOPY",
            Opcode::EXTCODEHASH => "EXTCODEHASH",
            Opcode::RETURNDATASIZE => "RETURNDATASIZE",
            Opcode::RETURNDATACOPY => "RETURNDATACOPY",
            Opcode::BLOCKHASH => "BLOCKHASH",
            Opcode::COINBASE => "COINBASE",
            Opcode::TIMESTAMP => "TIMESTAMP",
            Opcode::NUMBER => "NUMBER",
            Opcode::DIFFICULTY => "DIFFICULTY",
            Opcode::GASLIMIT => "GASLIMIT",
            Opcode::SLOAD => "SLOAD",
            Opcode::SSTORE => "SSTORE",
            Opcode::GAS => "GAS",
            Opcode::LOG0 => "LOG0",
            Opcode::LOG1 => "LOG1",
            Opcode::LOG2 => "LOG2",
            Opcode::LOG3 => "LOG3",
            Opcode::LOG4 => "LOG4",
            Opcode::CREATE => "CREATE",
            Opcode::CREATE2 => "CREATE2",
            Opcode::CALL => "CALL",
            Opcode::CALLCODE => "CALLCODE",
            Opcode::DELEGATECALL => "DELEGATECALL",
            Opcode::STATICCALL => "STATICCALL",
            Opcode::SUICIDE => "SUICIDE",
            Opcode::CHAINID => "CHAINID",
            _ => "UNKNOWN",
        }
    }
}

struct Stack {
    data: Vec<H256>,
}
impl Stack {
    fn new(data: Vec<H256>) -> Self {
        Self { data }
    }
    fn peek(value: &JsValue, params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let index = params
            .get(0)
            .ok_or_else(|| {
                JsError::from_opaque(JsValue::String(JsString::from("peek params is empty")))
            })?
            .to_i32(ctx)?;
        value
            .as_object()
            .ok_or_else(|| {
                JsError::from_opaque(JsValue::String(JsString::from("peek value as_object")))
            })?
            .get(format!("data{}", index), ctx)
    }
    fn length(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::Integer(0)))?
            .get("data_length", ctx)
    }
    pub fn to_jsobject(&self, ctx: &mut Context) -> JsResult<JsObject> {
        let peek = FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::peek)).build();

        let length =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::length)).build();

        let obj = JsObject::default();
        for (index, value) in self.data.iter().enumerate() {
            obj.set(
                format!("data{}", index),
                JsValue::String(JsString::from(format!("{:?}", value))),
                true,
                ctx,
            )?;
        }
        obj.set(
            "data_length",
            JsValue::Integer(self.data.len() as i32),
            true,
            ctx,
        )?;
        obj.set("peek", peek, true, ctx)?;
        obj.set("length", length, true, ctx)?;
        Ok(obj)
    }
}

struct Memory {
    data: Vec<u8>,
}
impl Memory {
    fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
    fn slice(value: &JsValue, params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let start = params
            .get(0)
            .ok_or_else(|| {
                JsError::from_opaque(JsValue::String(JsString::from("slice params 0 is empty")))
            })?
            .to_u32(ctx)? as usize;
        let mut end = params
            .get(1)
            .ok_or_else(|| {
                JsError::from_opaque(JsValue::String(JsString::from("slice params 1 is empty")))
            })?
            .to_u32(ctx)? as usize;

        let length = value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::Integer(0)))?
            .get("data_length", ctx)?
            .to_u32(ctx)? as usize;
        if end > length {
            end = length;
        }
        if start >= end {
            return Ok(JsValue::String(JsString::from("slice start >= end")));
        }
        let data = value
            .as_object()
            .ok_or_else(|| {
                JsError::from_opaque(JsValue::String(JsString::from("slice value as_object")))
            })?
            .get("data", ctx)?
            .to_string(ctx)?
            .to_std_string()
            .map_err(|_| {
                JsError::from_opaque(JsValue::String(JsString::from("to_std_string error")))
            })?;

        let data = hex::decode(data).map_err(|_| {
            JsError::from_opaque(JsValue::String(JsString::from(
                "slice hex::decode(data) error",
            )))
        })?;
        let mut ret_data = vec![];
        for index in start..end {
            if let Some(v) = data.get(index) {
                ret_data.push(*v);
            }
        }
        Ok(JsValue::String(JsString::from(hex::encode(&ret_data))))
    }
    fn get_uint(value: &JsValue, params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let index = params
            .get(0)
            .ok_or_else(|| {
                JsError::from_opaque(JsValue::String(JsString::from(
                    "get_uint params 0 is empty",
                )))
            })?
            .to_u32(ctx)?;
        value
            .as_object()
            .ok_or_else(|| {
                JsError::from_opaque(JsValue::String(JsString::from(
                    "get_uint value as_object 1",
                )))
            })?
            .get("data", ctx)?
            .as_object()
            .ok_or_else(|| {
                JsError::from_opaque(JsValue::String(JsString::from(
                    "get_uint value as_object 2",
                )))
            })?
            .get(index as usize + 32, ctx)
    }
    fn length(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or(JsError::from_opaque(JsValue::Integer(0)))?
            .get("data_length", ctx)
    }
    pub fn to_jsobject(&self, ctx: &mut Context) -> JsResult<JsObject> {
        let slice =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::slice)).build();

        let get_uint =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_uint)).build();

        let length =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::length)).build();

        let obj = JsObject::default();
        obj.set(
            "data",
            JsValue::String(JsString::from(hex::encode(&self.data))),
            true,
            ctx,
        )?;
        obj.set(
            "data_length",
            JsValue::Integer(self.data.len() as i32),
            true,
            ctx,
        )?;
        obj.set("slice", slice, true, ctx)?;
        obj.set("getUint", get_uint, true, ctx)?;
        obj.set("length", length, true, ctx)?;
        Ok(obj)
    }
}

struct Contract {
    caller: H160,
    address: H160,
    value: U256,
    input: Vec<u8>,
}
impl Contract {
    fn new(caller: H160, address: H160, value: U256, input: Vec<u8>) -> Self {
        Self {
            caller,
            address,
            value,
            input,
        }
    }
    fn get_caller(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?
            .get("caller", ctx)
    }
    fn get_address(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?
            .get("address", ctx)
    }
    fn get_value(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?
            .get("value", ctx)
    }
    fn get_input(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?
            .get("input", ctx)
    }
    pub fn to_jsobject(&self, ctx: &mut Context) -> JsResult<JsObject> {
        let get_caller =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_caller)).build();

        let get_address =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_address)).build();

        let get_value =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_value)).build();

        let get_input =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_input)).build();

        let obj = JsObject::default();
        obj.set(
            "caller",
            JsValue::String(JsString::from(format!("{:?}", self.caller))),
            true,
            ctx,
        )?;
        obj.set(
            "address",
            JsValue::String(JsString::from(format!("{:?}", self.address))),
            true,
            ctx,
        )?;
        obj.set(
            "value",
            JsValue::String(JsString::from(self.value.to_string())),
            true,
            ctx,
        )?;
        obj.set(
            "input",
            JsValue::String(JsString::from(format!("0x{}", hex::encode(&self.input)))),
            true,
            ctx,
        )?;

        obj.set("getCaller", get_caller, true, ctx)?;
        obj.set("getAddress", get_address, true, ctx)?;
        obj.set("getValue", get_value, true, ctx)?;
        obj.set("getInput", get_input, true, ctx)?;
        Ok(obj)
    }
}

pub struct Log {
    pc: u64,
    gas: u64,
    cost: u64,
    depth: u64,
    refund: u64,
    err: Option<String>,
    op: Op,
    stack: Option<Stack>,
    memory: Option<Memory>,
    contract: Contract,
}
impl Log {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pc: u64,
        gas: u64,
        cost: u64,
        depth: u64,
        refund: u64,
        err: Option<String>,
        opcode: Opcode,
        stack_data: Option<Vec<H256>>,
        memory_data: Option<Vec<u8>>,
        caller: H160,
        address: H160,
        value: U256,
        input: Vec<u8>,
    ) -> Self {
        Self {
            pc,
            gas,
            cost,
            depth,
            refund,
            err,
            op: Op::new(opcode),
            stack: stack_data.map(Stack::new),
            memory: memory_data.map(Memory::new),
            contract: Contract::new(caller, address, value, input),
        }
    }
    fn get_pc(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?
            .get("pc", ctx)
    }
    fn get_gas(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?
            .get("gas", ctx)
    }
    fn get_cost(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?
            .get("cost", ctx)
    }
    fn get_depth(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?
            .get("depth", ctx)
    }
    fn get_refund(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?
            .get("refund", ctx)
    }
    fn get_error(value: &JsValue, _params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        value
            .as_object()
            .ok_or_else(|| JsError::from_opaque(JsValue::undefined()))?
            .get("err", ctx)
    }
    pub fn to_jsobject(&self, ctx: &mut Context) -> JsResult<JsObject> {
        let get_pc =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_pc)).build();

        let get_gas =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_gas)).build();

        let get_cost =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_cost)).build();

        let get_depth =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_depth)).build();

        let get_refund =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_refund)).build();

        let get_error =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_error)).build();

        let op = self.op.to_jsobject(ctx)?;

        let contract = self.contract.to_jsobject(ctx)?;
        let obj = JsObject::default();
        obj.set("pc", JsValue::Integer(self.pc as i32), true, ctx)?;
        obj.set("gas", JsValue::Integer(self.gas as i32), true, ctx)?;
        obj.set("cost", JsValue::Integer(self.cost as i32), true, ctx)?;
        obj.set("depth", JsValue::Integer(self.depth as i32), true, ctx)?;
        obj.set("refund", JsValue::Integer(self.refund as i32), true, ctx)?;
        obj.set(
            "err",
            if let Some(ref e) = self.err {
                JsValue::String(JsString::from(e.as_str()))
            } else {
                JsValue::undefined()
            },
            true,
            ctx,
        )?;
        obj.set("getPC", get_pc, true, ctx)?;
        obj.set("getGas", get_gas, true, ctx)?;
        obj.set("getCost", get_cost, true, ctx)?;
        obj.set("getDepth", get_depth, true, ctx)?;
        obj.set("getRefund", get_refund, true, ctx)?;
        obj.set("getError", get_error, true, ctx)?;
        obj.set("op", op, true, ctx)?;
        obj.set(
            "stack",
            match self.stack {
                Some(ref s) => JsValue::Object(s.to_jsobject(ctx)?),
                None => JsValue::undefined(),
            },
            true,
            ctx,
        )?;
        obj.set(
            "memory",
            match self.memory {
                Some(ref m) => JsValue::Object(m.to_jsobject(ctx)?),
                None => JsValue::undefined(),
            },
            true,
            ctx,
        )?;
        obj.set("contract", contract, true, ctx)?;
        Ok(obj)
    }
}

pub struct DB {
    height: U256,
}
impl DB {
    pub fn new(height: U256) -> Self {
        Self { height }
    }
    fn get_balance(value: &JsValue, params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let height = value
            .as_object()
            .ok_or_else(|| {
                JsError::from_opaque(JsValue::String(JsString::from(
                    "get_balance value.as_object()",
                )))
            })?
            .get("height", ctx)?
            .to_u32(ctx)?;

        let address = H160::from_str(
            params
                .get(0)
                .ok_or_else(|| {
                    JsError::from_opaque(JsValue::String(JsString::from(
                        "get_balance params is empty",
                    )))
                })?
                .to_string(ctx)?
                .to_std_string()
                .map_err(|_| {
                    JsError::from_opaque(JsValue::String(JsString::from("to_std_string error")))
                })?
                .as_str(),
        )
        .map_err(|_| {
            JsError::from_opaque(JsValue::String(JsString::from(
                "get_balance from_str error",
            )))
        })?;
        if let Some(getter) = GETTER.get() {
            let info = match getter.get_balance(height, address) {
                Ok(b) => b,
                _ => U256::zero(),
            };
            Ok(JsValue::Integer(info.as_usize() as i32))
        } else {
            Ok(JsValue::Integer(0))
        }
    }
    fn get_nonce(value: &JsValue, params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let height = value
            .as_object()
            .ok_or_else(|| {
                JsError::from_opaque(JsValue::String(JsString::from(
                    "get_nonce value.as_object()",
                )))
            })?
            .get("height", ctx)?
            .to_u32(ctx)?;
        let address = H160::from_str(
            params
                .get(0)
                .ok_or_else(|| {
                    JsError::from_opaque(JsValue::String(JsString::from(
                        "get_nonce params is empty",
                    )))
                })?
                .to_string(ctx)?
                .to_std_string()
                .map_err(|_| {
                    JsError::from_opaque(JsValue::String(JsString::from("to_std_string error")))
                })?
                .as_str(),
        )
        .map_err(|_| {
            JsError::from_opaque(JsValue::String(JsString::from("get_nonce from_str error")))
        })?;
        if let Some(getter) = GETTER.get() {
            let info = match getter.get_nonce(height, address) {
                Ok(b) => b,
                _ => U256::zero(),
            };
            Ok(JsValue::Integer(info.as_usize() as i32))
        } else {
            Ok(JsValue::Integer(0))
        }
    }
    fn get_code(value: &JsValue, params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let height = value
            .as_object()
            .ok_or_else(|| {
                JsError::from_opaque(JsValue::String(JsString::from(
                    "get_code value.as_object()",
                )))
            })?
            .get("height", ctx)?
            .to_u32(ctx)?;
        let address = H160::from_str(
            params
                .get(0)
                .ok_or_else(|| {
                    JsError::from_opaque(JsValue::String(JsString::from(
                        "get_code params is empty",
                    )))
                })?
                .to_string(ctx)?
                .to_std_string()
                .map_err(|_| {
                    JsError::from_opaque(JsValue::String(JsString::from("to_std_string error")))
                })?
                .as_str(),
        )
        .map_err(|_| {
            JsError::from_opaque(JsValue::String(JsString::from("get_code from_str error")))
        })?;
        if let Some(getter) = GETTER.get() {
            let info = match getter.get_byte_code(height, address) {
                Ok(b) => b,
                _ => vec![],
            };
            Ok(JsValue::String(JsString::from(format!(
                "0x{}",
                hex::encode(info)
            ))))
        } else {
            Ok(JsValue::String(JsString::from(
                "get_code GETTER.get() error",
            )))
        }
    }
    fn get_state(value: &JsValue, params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let height = value
            .as_object()
            .ok_or_else(|| {
                JsError::from_opaque(JsValue::String(JsString::from(
                    "get_state value.as_object()",
                )))
            })?
            .get("height", ctx)?
            .to_u32(ctx)?;
        let address = H160::from_str(
            params
                .get(0)
                .ok_or_else(|| {
                    JsError::from_opaque(JsValue::String(JsString::from(
                        "get_state params is empty 0",
                    )))
                })?
                .to_string(ctx)?
                .to_std_string()
                .map_err(|_| {
                    JsError::from_opaque(JsValue::String(JsString::from("to_std_string error")))
                })?
                .as_str(),
        )
        .map_err(|_| {
            JsError::from_opaque(JsValue::String(JsString::from(
                "get_state H160 from_str error",
            )))
        })?;
        let index = H256::from_str(
            params
                .get(0)
                .ok_or_else(|| {
                    JsError::from_opaque(JsValue::String(JsString::from(
                        "get_state params is empty 1",
                    )))
                })?
                .to_string(ctx)?
                .to_std_string()
                .map_err(|_| {
                    JsError::from_opaque(JsValue::String(JsString::from("to_std_string error")))
                })?
                .as_str(),
        )
        .map_err(|_| {
            JsError::from_opaque(JsValue::String(JsString::from(
                "get_state H160 from_str error",
            )))
        })?;
        if let Some(getter) = GETTER.get() {
            let info = match getter.get_state(height, address, index) {
                Ok(b) => b,
                _ => H256::zero(),
            };
            Ok(JsValue::String(JsString::from(format!("{:?}", info))))
        } else {
            Ok(JsValue::String(JsString::from(
                "get_state GETTER.get() error",
            )))
        }
    }
    fn exists(value: &JsValue, params: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let height = value
            .as_object()
            .ok_or_else(|| {
                JsError::from_opaque(JsValue::String(JsString::from("exists value.as_object()")))
            })?
            .get("height", ctx)?
            .to_u32(ctx)?;
        let address = H160::from_str(
            params
                .get(0)
                .ok_or_else(|| {
                    JsError::from_opaque(JsValue::String(JsString::from(
                        "get_state params is empty",
                    )))
                })?
                .to_string(ctx)?
                .to_std_string()
                .map_err(|_| {
                    JsError::from_opaque(JsValue::String(JsString::from("to_std_string error")))
                })?
                .as_str(),
        )
        .map_err(|_| {
            JsError::from_opaque(JsValue::String(JsString::from(
                "exists H160 from_str error",
            )))
        })?;
        if let Some(getter) = GETTER.get() {
            let info = match getter.addr_state_exists(height, address) {
                Ok(b) => b,
                _ => false,
            };
            Ok(JsValue::Boolean(info))
        } else {
            Ok(JsValue::Boolean(false))
        }
    }
    pub fn to_jsobject(&self, ctx: &mut Context) -> JsResult<JsObject> {
        let get_balance =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_balance)).build();

        let get_nonce =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_nonce)).build();

        let get_code =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_code)).build();

        let get_state =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::get_state)).build();

        let exists =
            FunctionObjectBuilder::new(ctx, NativeFunction::from_fn_ptr(Self::exists)).build();

        let obj = JsObject::default();
        obj.set(
            "height",
            JsValue::Integer(self.height.as_usize() as i32 + 1),
            true,
            ctx,
        )?;
        obj.set("getBalance", get_balance, true, ctx)?;
        obj.set("getNonce", get_nonce, true, ctx)?;
        obj.set("getCode", get_code, true, ctx)?;
        obj.set("getState", get_state, true, ctx)?;
        obj.set("exists", exists, true, ctx)?;
        Ok(obj)
    }
}

pub struct Ctx {
    block_hash: H256,
    tx_index: U256,
    tx_hash: H256,
    contract_type: String,
    from: H160,
    to: H160,
    input: Vec<u8>,
    gas: U256,
    gas_price: U256,
    value: U256,
    block: U256,
    intrinsic_gas: U256,
    output: Vec<u8>,
    time: DateTime<UTC>,
    gas_used: U256,
    error: Option<String>,
}
impl Ctx {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        block_hash: H256,
        tx_index: U256,
        tx_hash: H256,
        contract_type: String,
        from: H160,
        to: H160,
        input: Vec<u8>,
        gas: U256,
        gas_price: U256,
        value: U256,
        block: U256,
        intrinsic_gas: U256,
        output: Vec<u8>,
        time: DateTime<UTC>,
        gas_used: U256,
        error: Option<String>,
    ) -> Self {
        Self {
            block_hash,
            tx_index,
            tx_hash,
            contract_type,
            from,
            to,
            input,
            gas,
            gas_price,
            value,
            block,
            intrinsic_gas,
            output,
            time,
            gas_used,
            error,
        }
    }
    pub fn to_jsobject(&self, ctx: &mut Context) -> JsResult<JsObject> {
        let obj = JsObject::default();
        obj.set(
            "blockHash",
            JsValue::String(JsString::from(format!("{:?}", &self.block_hash))),
            true,
            ctx,
        )?;
        obj.set(
            "txIndex",
            JsValue::Integer(self.tx_index.as_usize() as i32),
            true,
            ctx,
        )?;
        obj.set(
            "txHash",
            JsValue::String(JsString::from(format!("{:?}", &self.tx_hash))),
            true,
            ctx,
        )?;
        obj.set(
            "type",
            JsValue::String(JsString::from(self.contract_type.as_str())),
            true,
            ctx,
        )?;
        obj.set(
            "from",
            JsValue::String(JsString::from(format!("{:?}", &self.from))),
            true,
            ctx,
        )?;
        obj.set(
            "to",
            JsValue::String(JsString::from(format!("{:?}", &self.to))),
            true,
            ctx,
        )?;
        obj.set(
            "input",
            JsValue::String(JsString::from(format!("0x{}", hex::encode(&self.input)))),
            true,
            ctx,
        )?;
        obj.set(
            "gas",
            JsValue::Integer(self.gas.as_usize() as i32),
            true,
            ctx,
        )?;
        obj.set(
            "gasPrice",
            JsValue::Integer(self.gas_price.as_usize() as i32),
            true,
            ctx,
        )?;
        obj.set(
            "value",
            JsValue::String(JsString::from(self.value.to_string().as_str())),
            true,
            ctx,
        )?;
        obj.set(
            "block",
            JsValue::Integer(self.block.as_usize() as i32),
            true,
            ctx,
        )?;
        obj.set(
            "intrinsicGas",
            JsValue::Integer(self.intrinsic_gas.as_usize() as i32),
            true,
            ctx,
        )?;
        obj.set(
            "output",
            JsValue::String(JsString::from(format!("0x{}", hex::encode(&self.output)))),
            true,
            ctx,
        )?;
        obj.set(
            "time",
            JsValue::String(JsString::from(self.time.to_string())),
            true,
            ctx,
        )?;
        obj.set(
            "gasUsed",
            JsValue::Integer(self.gas_used.as_usize() as i32),
            true,
            ctx,
        )?;
        obj.set(
            "error",
            if let Some(ref e) = self.error {
                JsValue::String(JsString::from(e.as_str()))
            } else {
                JsValue::undefined()
            },
            true,
            ctx,
        )?;
        Ok(obj)
    }
}
