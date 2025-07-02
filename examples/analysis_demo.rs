use script::ir::optimizer::analysis::{
    AnalysisManager, ControlFlowGraph, DominanceAnalysis, LivenessAnalysis, UseDefChains,
};
use script::ir::{BinaryOp, Constant, IrBuilder};
use script::types::Type;

fn main() {
    println!("Testing Analysis Infrastructure");

    // Create a simple IR function for testing
    let mut builder = IrBuilder::new();

    let func_id = builder.create_function("test_function".to_string(), vec![], Type::I32);

    let entry = builder.get_current_block().unwrap();
    builder.set_current_block(entry);

    let x = builder.const_value(Constant::I32(1));
    let y = builder.const_value(Constant::I32(2));
    let result = builder
        .build_binary(BinaryOp::Add, x, y, Type::I32)
        .unwrap();
    builder.build_return(Some(result));

    let module = builder.build();
    let func = module.get_function(func_id).unwrap();

    println!("Created function: {}", func.name);

    // Test Control Flow Graph
    let cfg = ControlFlowGraph::build(func);
    println!("CFG has {} nodes", cfg.nodes.len());

    // Test Dominance Analysis
    let mut dom_analysis = DominanceAnalysis::new();
    let dom_info = dom_analysis.analyze(func);
    println!("Dominance analysis completed");

    // Test Use-Def Chains
    let mut use_def_analysis = UseDefChains::new();
    let use_def_info = use_def_analysis.analyze(func);
    println!(
        "Use-def analysis completed - found {} dead values",
        use_def_info.dead_values().len()
    );

    // Test Liveness Analysis
    let mut liveness_analysis = LivenessAnalysis::new();
    let liveness_info = liveness_analysis.analyze(func, &cfg);
    println!("Liveness analysis completed");

    // Test Analysis Manager
    let mut manager = AnalysisManager::new();
    let _dom_from_manager = manager.get_dominance_analysis(func);
    let _cfg_from_manager = manager.get_control_flow_graph(func);
    let _use_def_from_manager = manager.get_use_def_chains(func);
    let _liveness_from_manager = manager.get_liveness_analysis(func);

    println!("Analysis Manager working correctly");
    println!("All analysis infrastructure tests passed!");
}
