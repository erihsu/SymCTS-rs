use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
pub struct GenTopologyCfg {
    pub max_branch: usize,
}

#[derive(Serialize, Deserialize)]
pub struct BufferingCfg {
    pub buffer_list: Vec<String>,
    pub input_slew: f32,
    pub max_slew: f32,
    pub rho_matrix_path: String,
    pub pop_size: usize,
    pub elite_size: usize,
    pub mutation_rate: f32,
    pub generations: usize,
}

#[derive(Serialize, Deserialize)]
pub struct CtsCfg {
    pub name: String, // clock net name
    pub stage1_cfg: GenTopologyCfg,
    pub stage2_cfg: BufferingCfg,
}
