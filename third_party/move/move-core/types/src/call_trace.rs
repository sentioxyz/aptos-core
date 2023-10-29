use serde::{Deserialize, Serialize};

const CALL_STACK_SIZE_LIMIT: usize = 1024;

/// A call trace
///
/// This is a representation of the debug call trace
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InternalCallTrace {
    pub pc: u16,
    pub module_id: String,
    pub func_name: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub type_args: Vec<String>,
    pub sub_traces: CallTraces,
    pub fdef_idx: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CallTraces(pub Vec<InternalCallTrace>);

impl CallTraces {
    pub fn new() -> Self {
        CallTraces(vec![])
    }

    pub fn push(&mut self, trace: InternalCallTrace) -> Result<(), InternalCallTrace> {
        if self.0.len() < CALL_STACK_SIZE_LIMIT {
            self.0.push(trace);
            Ok(())
        } else {
            Err(trace)
        }
    }

    pub fn pop(&mut self) -> Option<InternalCallTrace> {
        self.0.pop()
    }

    pub fn set_outputs(&mut self, outputs: Vec<String>) {
        let length = self.0.len();
        self.0[length - 1].outputs = outputs
    }

    pub fn push_call_trace(&mut self, call_trace: InternalCallTrace) {
        let length = self.0.len();
        self.0[length - 1].sub_traces.push(call_trace);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn root(&mut self) -> Option<InternalCallTrace> {
        self.0.pop()
    }
}
