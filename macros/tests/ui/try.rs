use proc_macro_post::my_proc_macro;

#[derive(RpcMessage)]
pub struct TheseParams {
    field: String,
}

fn main() {
    my_proc_macro!(43);
}
