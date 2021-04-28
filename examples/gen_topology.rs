use sym_cts_rs::ClockTree;

fn main() {
    let mut cns = ClockTree::new("clk_i");
    let _ = cns.run("mem_ctrl.def").unwrap();
}
