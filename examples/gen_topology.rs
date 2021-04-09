use sym_cts_rs::ClockTree;

fn main() {
    let mut cns = ClockTree::new("blif_clk_net");
    let _ = cns.run("s1238_placed.def").unwrap();
}
