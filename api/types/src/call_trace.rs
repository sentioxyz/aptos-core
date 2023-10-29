use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use move_core_types::call_trace::InternalCallTrace;

/// A call trace
///
/// This is a representation of the debug call trace
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Object)]
pub struct CallTrace {
    pub pc: u16,
    pub module_id: String,
    pub func_name: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub type_args: Vec<String>,
    pub sub_traces: Vec<CallTrace>,
}

impl From<InternalCallTrace> for CallTrace {
    fn from(value: InternalCallTrace) -> Self {
        CallTrace {
            pc: value.pc,
            module_id: value.module_id,
            func_name: value.func_name,
            inputs: value.inputs,
            outputs: value.outputs,
            type_args: value.type_args,
            sub_traces: value.sub_traces.0.into_iter().enumerate().map(|(_, trace)| {
                CallTrace::from(trace)
            }).collect(),
        }
    }
}
