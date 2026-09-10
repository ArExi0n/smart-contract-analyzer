use std::path::Path;

use solang_parser::helpers::CodeLocation;
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
    pub source_idx: usize,
    pub has_loops: bool,
    pub has_assembly: bool,
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
        func: &pt::FunctionDefinition,
        source: &str,
        path: &std::path::Path,
        contract_idx: usize,
        source_idx: usize,
    ) {
        let name = func
            .name
            .as_ref()
            .map(|n| n.name.clone())
            .unwrap_or_default();

        let (is_constructor, is_receive, is_fallback) = match &func.ty {
            pt::FunctionTy::Constructor => (true, false, false),
            pt::FunctionTy::Receive => (false, true, false),
            pt::FunctionTy::Fallback => (false, false, true),
            pt::FunctionTy::Function => (false, false, false),
            pt::FunctionTy::Modifier => (false, false, false),
        };

        let visibility = func
            .attributes
            .iter()
            .find_map(|attr| match attr {
                pt::FunctionAttribute::Visibility(v) => Some(match v {
                    pt::Visibility::Public(_) => Visibility::Public,
                    pt::Visibility::External(_) => Visibility::External,
                    pt::Visibility::Internal(_) => Visibility::Internal,
                    pt::Visibility::Private(_) => Visibility::Private,
                }),
                _ => None,
            })
            .unwrap_or(Visibility::Default);

        let mutability = func
            .attributes
            .iter()
            .find_map(|attr| match attr {
                pt::FunctionAttribute::Mutability(m) => Some(match m {
                    pt::Mutability::Pure(_) => Mutability::Pure,
                    pt::Mutability::View(_) => Mutability::View,
                    pt::Mutability::Payable(_) => Mutability::Payable,
                    pt::Mutability::Constant(_) => Mutability::View,
                }),
                _ => None,
            })
            .unwrap_or(Mutability::NonPayable);

        let modifiers: Vec<String> = func
            .attributes
            .iter()
            .filter_map(|attr| {
                if let pt::FunctionAttribute::BaseOrModifier(_, base) = attr {
                    Some(
                        base.name
                            .identifiers
                            .iter()
                            .map(|i| i.name.clone())
                            .collect::<Vec<_>>()
                            .join("."),
                    )
                } else {
                    None
                }
            })
            .collect();

        let params = extract_params(&func.params);
        let returns = extract_params(&func.returns);

        // Extract body source for pattern analysis
        let body_source = if let Some(body) = &func.body {
            match body.loc() {
                pt::Loc::File(_, start, end) => source.get(start..end).unwrap_or("").to_string(),
                _ => String::new(),
            }
        } else {
            String::new()
        };

        // Analyze body for external calls, state writes, loops, state reads, and emits
        let external_calls = find_external_calls(&body_source, source, path);
        let state_writes = find_state_writes(&body_source, source, path);
        let state_reads = find_state_reads(&body_source);
        let emits = find_emits(&body_source, source, path);
        let has_loops = body_source.contains("for (")
            || body_source.contains("for(")
            || body_source.contains("while (")
            || body_source.contains("while(")
            || body_source.contains("do {");
        let has_assembly = body_source.contains("assembly {") || body_source.contains("assembly{");

        self.functions.push(FunctionInfo {
            name,
            loc: func.loc,
            contract_idx,
            visibility,
            mutability,
            modifiers,
            params,
            returns,
            is_constructor,
            is_receive,
            is_fallback,
            body_source,
            external_calls,
            state_writes,
            state_reads,
            has_loops,
            has_assembly,
            emits,
            source_idx,
        });
    }

    fn extract_state_var(
        &mut self,
        var: &pt::VariableDefinition,
        source: &str,
        path: &std::path::Path,
        contract_idx: usize,
        source_idx: usize,
    ) {
        let name = var
            .name
            .as_ref()
            .map(|n| n.name.clone())
            .unwrap_or_default();
        let type_name = format_type(&var.ty);

        let visibility = var
            .attrs
            .iter()
            .find_map(|attr| match attr {
                pt::VariableAttribute::Visibility(v) => Some(match v {
                    pt::Visibility::Public(_) => Visibility::Public,
                    pt::Visibility::External(_) => Visibility::External,
                    pt::Visibility::Internal(_) => Visibility::Internal,
                    pt::Visibility::Private(_) => Visibility::Private,
                }),
                _ => None,
            })
            .unwrap_or(Visibility::Internal);

        let is_constant = var
            .attrs
            .iter()
            .any(|a| matches!(a, pt::VariableAttribute::Constant(_)));
        let is_immutable = var
            .attrs
            .iter()
            .any(|a| matches!(a, pt::VariableAttribute::Immutable(_)));
        let is_initialized = var.initializer.is_some();

        self.state_vars.push(StateVarInfo {
            name,
            type_name,
            loc: var.loc,
            contract_idx,
            visibility,
            is_constant,
            is_immutable,
            is_initialized,
            source_idx,
        });
    }
}
