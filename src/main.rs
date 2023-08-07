mod crypto;
pub use crypto::ed25519;

fn main() {
    println!("Hello, world!");
    ed25519::hello();
}
