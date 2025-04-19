use std::collections::HashSet;
use move_core_types::value::MoveValue;
use crate::errors::VMError;

const CALL_STACK_SIZE_LIMIT: usize = 1024;

/// A call trace
///
/// This is a representation of the debug call trace
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InternalCallTrace {
    pub pc: u16,
    pub from_module_id: String,
    pub module_id: String,
    pub func_name: String,
    pub inputs: Vec<MoveValue>,
    pub outputs: Vec<MoveValue>,
    pub type_args: Vec<String>,
    pub sub_traces: CallTraces,
    pub fdef_idx: u16,
    pub gas_info: GasInfo,
    pub error: Option<VMError>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GasInfo {
    start_balance: u64,
    end_balance: u64,
}

impl GasInfo {
    pub fn make_frame(start_balance: u64) -> Self {
        Self {
            start_balance,
            end_balance: 0,
        }
    }

    pub fn close_frame(&mut self, end_balance: u64) {
        self.end_balance = end_balance;
    }

    pub fn gas_used(&self) -> u64 {
        self.start_balance - self.end_balance
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallTraces(pub Vec<InternalCallTrace>, pub HashSet<String>);

impl CallTraces {
    pub fn new() -> Self {
        CallTraces(vec![], HashSet::new())
    }

    pub fn push(&mut self, trace: InternalCallTrace) -> Result<(), InternalCallTrace> {
        if self.0.len() < CALL_STACK_SIZE_LIMIT {
            self.0.push(trace);
            let account = self.0[self.0.len() - 1].module_id.split("::").next().unwrap().to_string();
            self.1.insert(account);
            Ok(())
        } else {
            Err(trace)
        }
    }

    pub fn pop(&mut self) -> Option<InternalCallTrace> {
        self.0.pop()
    }

    pub fn set_outputs(&mut self, outputs: Vec<MoveValue>) {
        let length = self.0.len();
        self.0[length - 1].outputs = outputs
    }

    pub fn set_error(&mut self, error: VMError) {
        let length = self.0.len();
        self.0[length - 1].error = Some(error)
    }

    pub fn push_call_trace(&mut self, call_trace: InternalCallTrace) {
        let length = self.0.len();
        self.0[length - 1].sub_traces.push(call_trace).expect("exceed the call trace limit");
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn root(&mut self) -> Option<InternalCallTrace> {
        self.0.pop()
    }

    pub fn set_gas_end(&mut self, balance: u64) {
        let length = self.0.len();
        self.0[length - 1].gas_info.close_frame(balance);
    }

    pub fn set_root_gas(&mut self, start_balance: u64, end_balance: u64) {
        let length = self.0.len();
        self.0[length - 1].gas_info = GasInfo {
            start_balance,
            end_balance,
        };
    }
}
