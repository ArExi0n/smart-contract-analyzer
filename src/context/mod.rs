use std::path::Path;

use solang_parser::pt::{self, Loc};

use crate::ast::ParsedSource;

#[derive(Debug, Clone)]
pub struct ContractInfo {
    pub name: String,
    pub loc: Loc,
    pub kind: ContractKind,
    pub events: Vec<String>,
    pub functions: Vec<usize>,
    pub state_variables: Vec<usize>,
    pub bases: Vec<String>,
    pub modifiers: Vec<String>,
    pub source_idx: usize,
    pub has_fallback: bool,
    pub has_receive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractKind {
    Contract,
    Lib,
    Abstract,
    Interface,
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub loc: Loc,
    pub mutability: Mutability,
    pub visibility: Visibility,
    pub contract_idx: usize,
    pub modifiers: Vec<String>,
    pub returns: Vec<ParamsInfo>,
    pub params: Vec<ParamsInfo>,
    pub body_source: String,
    pub is_constructor: bool,
    pub is_fallback: bool,
    pub is_receive: bool,
    pub external_calls: Vec<ExternalCallInfo>,
    pub state_writes: Vec<StateWriteInfo>,
    pub has_loop: bool,
    pub source_idx: usize,
    pub state_reads: Vec<String>,
    pub emits: Vec<EmitInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    Public,
    External,
    Internal,
    Private,
    Default,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mutability {
    Pure,
    View,
    Payable,
    NonPayable,
}

#[derive(Debug, Clone)]
pub struct ParamsInfo {
    pub name: String,
    pub type_name: String,
}

/// Info about external calls within a function
#[derive(Debug, Clone)]
pub struct ExternalCallInfo {
    pub loc: Loc,
    pub target: String,
    pub call_type: ExternalCallType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalCallType {
    Call,
    DelegateCall,
    StaticCall,
    Send,
    Transfer,
    HighLevel,
}

/// Info about state variable write.
#[derive(Debug, Clone)]
pub struct StateWriteInfo {
    pub loc: Loc,
    pub variable_name: String,
}

/// Info about an emit statement
#[derive(Debug, Clone)]
pub struct EmitInfo {
    pub loc: Loc,
    pub event_name: String,
}

/// Extracted state variable information
#[derive(Debug, Clone)]
pub struct StateVarInfo {
    pub name: String,
    pub type_name: String,
    pub loc: Loc,
    pub contract_idx: usize,
    pub visibility: Visibility,
    pub is_constant: bool,
    pub is_immutable: bool,
    pub is_initialized: bool,
    pub source_idx: usize,
}

/// The central indexed AST database
pub struct WorkspaceContext {
    pub contracts: Vec<ContractInfo>,
    pub sources: Vec<ParsedSource>,
    pub functions: Vec<FunctionInfo>,
    pub state_vars: Vec<StateVarInfo>,
}

impl WorkspaceContext {
    pub fn from_parsed_sources(sources: &[ParsedSource]) -> Self {
        let mut ctx = WorkspaceContext {
            contracts: Vec::new(),
            sources: Vec::new(),
            functions: Vec::new(),
            state_vars: Vec::new(),
        };

        for (source_idx, parsed) in sources.iter().enumerate() {
            for part in &parsed.tree.0 {
                if let pt::SourceUnitPart::ContractDefinition(def) = part {
                    ctx.extract_contract(def, &parsed.source, &parsed.path, source_idx);
                }
            }
        }
        ctx
    }

    fn extract_contract(
        &mut self,
        def: &pt::ContractDefinition,
        source: &str,
        path: &Path,
        source_idx: usize,
    ) {
        let contract_idx = self.contracts.len();
        let name = def
            .name
            .as_ref()
            .map(|s| s.name.clone())
            .unwrap_or_default();
        let kind = match def.ty {
            pt::ContractTy::Contract(_) => ContractKind::Contract,
            pt::ContractTy::Library(_) => ContractKind::Lib,
            pt::ContractTy::Abstract(_) => ContractKind::Abstract,
            pt::ContractTy::Interface(_) => ContractKind::Interface,
        };

        let bases: Vec<String> = def
            .base
            .iter()
            .map(|b| {
                b.name
                    .identifiers
                    .iter()
                    .map(|i| i.name.clone())
                    .collect::<Vec<_>>()
                    .join(".")
            })
            .collect();

        let mut contract_info = ContractInfo {
            name,
            loc: def.loc,
            kind,
            events: Vec::new(),
            functions: Vec::new(),
            state_variables: Vec::new(),
            bases,
            modifiers: Vec::new(),
            source_idx,
            has_fallback: false,
            has_receive: false,
        };

        // Extract all parts of the contract
        for part in &def.parts {
            match part {
                pt::ContractPart::FunctionDefinition(func) => {
                    let func_idx = self.functions.len();
                    contract_info.functions.push(func_idx);
                    self.extract_function(func, source, path, contract_idx, source_idx);
                }
                pt::ContractPart::VariableDefinition(var) => {
                    let var_idx = self.state_vars.len();
                    contract_info.state_variables.push(var_idx);
                    self.extract_state_var(var, source, path, contract_idx, source_idx);
                }
                pt::ContractPart::EventDefinition(ev) => {
                    if let Some(name) = &ev.name {
                        contract_info.events.push(name.name.clone());
                    }
                }
                _ => {}
            }
        }

        for func in &self.functions[contract_info.functions[0]..] {
            if func.is_fallback {
                contract_info.has_fallback = true;
            }
            if func.is_receive {
                contract_info.has_receive = true;
            }
        }

        self.contracts.push(contract_info);
    }

    fn extract_function(
        &mut self,
        _func: &pt::FunctionDefinition,
        _source: &str,
        _path: &Path,
        _contract_idx: usize,
        _source_idx: usize,
    ) {
    }

    fn extract_state_var(
        &mut self,
        _var: &pt::VariableDefinition,
        _source: &str,
        _path: &Path,
        _contract_idx: usize,
        _source_idx: usize,
    ) {
    }
}
