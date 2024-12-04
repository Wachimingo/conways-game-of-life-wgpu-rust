use wgpu_game_of_life_rust::run;

fn main() {
    pollster::block_on(run());
}
