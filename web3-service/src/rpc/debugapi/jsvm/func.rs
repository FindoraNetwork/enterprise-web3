use {
    super::params::{Cfg, Ctx, Frame, FrameResult, Log, DB},
    crate::rpc::debugapi::types::TraceParams,
    boa_engine::{
        prelude::JsObject, Context, JsError, JsResult, JsString, JsValue, NativeFunction, Source,
    },
    chrono::{DateTime, UTC},
    ethereum_types::{H160, H256, U256},
    evm::Opcode,
    jsonrpc_core::{Error, Result, Value},
    std::cell::RefCell,
};

pub struct Func<'a> {
    context: RefCell<Context<'a>>,
    this: JsValue,
    result_func: JsObject,
    fault_func: JsObject,
    step_func: Option<JsObject>,
    enter_func: Option<JsObject>,
    exit_func: Option<JsObject>,
    setup_func: Option<JsObject>,
}
impl<'a> Func<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        context: Context<'a>,
        this: JsValue,
        result_func: JsObject,
        fault_func: JsObject,
        step_func: Option<JsObject>,
        enter_func: Option<JsObject>,
        exit_func: Option<JsObject>,
        setup_func: Option<JsObject>,
    ) -> Self {
        Self {
            context: RefCell::new(context),
            this,
            result_func,
            fault_func,
            step_func,
            enter_func,
            exit_func,
            setup_func,
        }
    }
    pub fn call_setup_func(&self, params: TraceParams) -> Result<()> {
        if let Some(ref func) = self.setup_func {
            let context = &mut self.context.borrow_mut();
            let cfg = Cfg::new(params).to_jsobject(context).map_err(|_| {
                let mut err = Error::internal_error();
                err.message = "call fault function error".to_owned();
                err
            })?;
            func.call(&self.this, &[JsValue::Object(cfg)], context)
                .map(|_| ())
                .map_err(|_| {
                    let mut err = Error::internal_error();
                    err.message = "call setup function error".to_owned();
                    err
                })?;
        }
        Ok(())
    }

    pub fn call_enter_func(
        &self,
        contract_type: &str,
        from: H160,
        to: H160,
        input: Vec<u8>,
        gas: U256,
        value: U256,
    ) -> Result<()> {
        let context = &mut self.context.borrow_mut();
        let frame = Frame::new(contract_type, from, to, input, gas, value)
            .to_jsobject(context)
            .map_err(|_| {
                let mut err = Error::internal_error();
                err.message = "call fault function error".to_owned();
                err
            })?;
        if let Some(ref func) = self.enter_func {
            func.call(&self.this, &[JsValue::Object(frame)], context)
                .map(|_| ())
                .map_err(|_| {
                    let mut err = Error::internal_error();
                    err.message = "call enter function error".to_owned();
                    err
                })?;
        }
        Ok(())
    }

    pub fn call_exit_func(
        &self,
        gas_used: U256,
        output: Vec<u8>,
        err: Option<String>,
    ) -> Result<()> {
        if let Some(ref func) = self.exit_func {
            let context = &mut self.context.borrow_mut();
            let frame_result = FrameResult::new(gas_used, output, err)
                .to_jsobject(context)
                .map_err(|_| {
                    let mut err = Error::internal_error();
                    err.message = "call fault function error".to_owned();
                    err
                })?;
            func.call(&self.this, &[JsValue::Object(frame_result)], context)
                .map(|_| ())
                .map_err(|_| {
                    let mut err = Error::internal_error();
                    err.message = "call exit function error".to_owned();
                    err
                })?;
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn call_step_func(
        &self,
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
        height: U256,
    ) -> Result<()> {
        if let Some(ref func) = self.step_func {
            let context = &mut self.context.borrow_mut();
            let log = Log::new(
                pc,
                gas,
                cost,
                depth,
                refund,
                err,
                opcode,
                stack_data,
                memory_data,
                caller,
                address,
                value,
                input,
            )
            .to_jsobject(context)
            .map_err(|_| {
                let mut err = Error::internal_error();
                err.message = "call fault function error".to_owned();
                err
            })?;
            let db = DB::new(height).to_jsobject(context).map_err(|_| {
                let mut err = Error::internal_error();
                err.message = "call fault function error".to_owned();
                err
            })?;
            func.call(
                &self.this,
                &[JsValue::Object(log), JsValue::Object(db)],
                context,
            )
            .map(|_| ())
            .map_err(|_| {
                let mut err = Error::internal_error();
                err.message = "call step function error".to_owned();
                err
            })?;
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn call_fault_func(
        &self,
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
        height: U256,
    ) -> Result<()> {
        let context = &mut self.context.borrow_mut();
        let log = Log::new(
            pc,
            gas,
            cost,
            depth,
            refund,
            err,
            opcode,
            stack_data,
            memory_data,
            caller,
            address,
            value,
            input,
        )
        .to_jsobject(context)
        .map_err(|_| {
            let mut err = Error::internal_error();
            err.message = "call fault function error".to_owned();
            err
        })?;
        let db = DB::new(height).to_jsobject(context).map_err(|_| {
            let mut err = Error::internal_error();
            err.message = "call fault function error".to_owned();
            err
        })?;
        self.fault_func
            .call(
                &self.this,
                &[JsValue::Object(log), JsValue::Object(db)],
                context,
            )
            .map(|_| ())
            .map_err(|_| {
                let mut err = Error::internal_error();
                err.message = "call fault function error".to_owned();
                err
            })
    }
    #[allow(clippy::too_many_arguments)]
    pub fn call_result_func(
        &self,
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
        height: U256,
    ) -> Result<Value> {
        let context = &mut self.context.borrow_mut();
        let ctx = Ctx::new(
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
        )
        .to_jsobject(context)
        .map_err(|_| {
            let mut err = Error::internal_error();
            err.message = "ctx to_jsobject error ".to_owned();
            err
        })?;
        let db = DB::new(height).to_jsobject(context).map_err(|_| {
            let mut err = Error::internal_error();
            err.message = "db to_jsobject error".to_owned();
            err
        })?;
        let val = self
            .result_func
            .call(
                &self.this,
                &[JsValue::Object(ctx), JsValue::Object(db)],
                context,
            )
            .map_err(|e| {
                let mut err = Error::internal_error();
                err.message = "call result function error".to_owned();
                err
            })?;
        val.to_json(context).map_err(|_| {
            let mut err = Error::internal_error();
            err.message = "val.to_json error".to_owned();
            err
        })
    }
}

fn get_func(name: &str, js_obj: &JsObject, ctx: &mut Context) -> Result<Option<JsObject>> {
    let func = js_obj.get(name, ctx).map_err(|_| {
        let mut err = Error::internal_error();
        err.message = "javascript exec failed".to_owned();
        err
    })?;
    let func = func.as_object().and_then(|obj| {
        if !obj.is_function() {
            None
        } else {
            Some(obj.clone())
        }
    });

    Ok(func)
}

fn js_func(_value: &JsValue, params: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    params
        .get(0)
        .cloned()
        .ok_or_else(|| JsError::from_opaque(JsValue::String(JsString::from("params is empty"))))
}

pub fn parse_tracer<'a>(mut ctx: Context<'a>, tracer: &Option<String>) -> Result<Option<Func<'a>>> {
    let tracer = match tracer {
        Some(t) => t,
        None => return Ok(None),
    };
    if tracer.is_empty() {
        return Ok(None);
    }
    let tracer = format!("({})", tracer);

    let func = NativeFunction::from_fn_ptr(js_func);

    ctx.register_global_callable("toHex", 0, func.clone());
    ctx.register_global_callable("toAddress", 0, func.clone());
    ctx.register_global_callable("bigInt", 0, func);
    let value = ctx.eval(Source::from_bytes(&tracer)).map_err(|_| {
        let mut err = Error::internal_error();
        err.message = "javascript exec failed".to_owned();
        err
    })?;

    let js_obj = value.as_object().ok_or({
        let mut err = Error::internal_error();
        err.message = "javascript exec failed".to_owned();
        err
    })?;
    let result_func = get_func("result", js_obj, &mut ctx)?.ok_or({
        let mut err = Error::internal_error();
        err.message = "trace object must expose a function result()".to_owned();
        err
    })?;
    let fault_func = get_func("fault", js_obj, &mut ctx)?.ok_or({
        let mut err = Error::internal_error();
        err.message = "trace object must expose a function fault()".to_owned();
        err
    })?;
    let step_func = get_func("step", js_obj, &mut ctx)?;
    let enter_func = get_func("enter", js_obj, &mut ctx)?;
    let exit_func = get_func("exit", js_obj, &mut ctx)?;
    if enter_func.is_none() != exit_func.is_none() {
        let mut err = Error::internal_error();
        err.message = "trace object must expose a function fault()".to_owned();
        return Err(err);
    }
    let setup_func = get_func("setup", js_obj, &mut ctx)?;
    Ok(Some(Func::new(
        ctx,
        value,
        result_func,
        fault_func,
        step_func,
        enter_func,
        exit_func,
        setup_func,
    )))
}
