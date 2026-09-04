use solang_parser::pt::Loc;

#[derive(Debug, Clone)]
pub struct ContractInfo{
    pub name: String,
    pub loc: Loc,
    pub kind: ContractKind,
    pub functions: Vec<usize>,
    pub events: Vec<String>,
    pub bases: Vec<String>,
    pub modifiers: Vec<String>,
    pub state-variables: Vec<usize>,
    pub has_fallback: bool,
    pub has_recive: bool,
    pub source_idx: usize,
}

#[derive(Debug, Clone)]
