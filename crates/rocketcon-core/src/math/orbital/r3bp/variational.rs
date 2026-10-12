use super::equations::{cr3bp_acceleration, effective_potential_hessian};
use super::types::SynodicState;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Variational42State {
    pub state: SynodicState,
    pub stm: [f64; 36],
}

impl Variational42State {
    pub fn new(state: SynodicState) -> Self {
        let mut stm = [0.0; 36];
        for i in 0..6 {
            stm[i * 6 + i] = 1.0;
        }
        Self { state, stm }
    }

    pub fn stm_matrix(&self) -> [[f64; 6]; 6] {
        let mut mat = [[0.0; 6]; 6];
        for i in 0..6 {
            for j in 0..6 {
                mat[i][j] = self.stm[i * 6 + j];
            }
        }
        mat
    }
}

pub fn variational_42_derivative(var_state: &Variational42State, mu: f64) -> Variational42State {
    let pos = var_state.state.position;
    let vel = var_state.state.velocity;
    let acc = cr3bp_acceleration(pos, vel, mu);

    let h = effective_potential_hessian(pos, mu);

    let mut a_mat = [[0.0; 6]; 6];
    a_mat[0][3] = 1.0;
    a_mat[1][4] = 1.0;
    a_mat[2][5] = 1.0;

    a_mat[3][0] = h[0][0];
    a_mat[3][1] = h[0][1];
    a_mat[3][2] = h[0][2];
    a_mat[3][4] = 2.0;

    a_mat[4][0] = h[1][0];
    a_mat[4][1] = h[1][1];
    a_mat[4][2] = h[1][2];
    a_mat[4][3] = -2.0;

    a_mat[5][0] = h[2][0];
    a_mat[5][1] = h[2][1];
    a_mat[5][2] = h[2][2];

    let mut d_stm = [0.0; 36];
    for i in 0..6 {
        for j in 0..6 {
            let mut sum = 0.0;
            for k in 0..6 {
                sum += a_mat[i][k] * var_state.stm[k * 6 + j];
            }
            d_stm[i * 6 + j] = sum;
        }
    }

    Variational42State {
        state: SynodicState::new(vel, acc),
        stm: d_stm,
    }
}

pub fn variational_rk4_step(state: &Variational42State, mu: f64, dt: f64) -> Variational42State {
    let add_scaled = |s: &Variational42State, ds: &Variational42State, scale: f64| -> Variational42State {
        let p = s.state.position + ds.state.position * scale;
        let v = s.state.velocity + ds.state.velocity * scale;
        let mut stm = [0.0; 36];
        for i in 0..36 {
            stm[i] = s.stm[i] + ds.stm[i] * scale;
        }
        Variational42State {
            state: SynodicState::new(p, v),
            stm,
        }
    };

    let k1 = variational_42_derivative(state, mu);
    let s2 = add_scaled(state, &k1, 0.5 * dt);
    let k2 = variational_42_derivative(&s2, mu);
    let s3 = add_scaled(state, &k2, 0.5 * dt);
    let k3 = variational_42_derivative(&s3, mu);
    let s4 = add_scaled(state, &k3, dt);
    let k4 = variational_42_derivative(&s4, mu);

    let sixth = dt / 6.0;
    let p_new = state.state.position + (k1.state.position + k2.state.position * 2.0 + k3.state.position * 2.0 + k4.state.position) * sixth;
    let v_new = state.state.velocity + (k1.state.velocity + k2.state.velocity * 2.0 + k3.state.velocity * 2.0 + k4.state.velocity) * sixth;

    let mut stm_new = [0.0; 36];
    for i in 0..36 {
        stm_new[i] = state.stm[i] + (k1.stm[i] + k2.stm[i] * 2.0 + k3.stm[i] * 2.0 + k4.stm[i]) * sixth;
    }

    Variational42State {
        state: SynodicState::new(p_new, v_new),
        stm: stm_new,
    }
}