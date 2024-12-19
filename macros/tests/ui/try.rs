use proc_macro_post::my_proc_macro;

#[derive(RpcMessage)]

fn main() {
    my_proc_macro!(43);
}
