use std::env::*;
use sym_cts_rs::ClockTree;
fn main() {
    let mut cns = ClockTree::new("blif_clk_net");
    let cli_arg: Vec<String> = args().collect();
    println!("read in verilog file {:?}", cli_arg[1]);
    println!("read in def file {:?}", cli_arg[2]);

    let _ = cns
        .run(&cli_arg[1], &cli_arg[2], &cli_arg[3], &cli_arg[4])
        .unwrap();
}
