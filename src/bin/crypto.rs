use std::env;

use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;

use smokesignal::jose::jwk;

fn main() {
    let mut rng = rand::thread_rng();

    env::args().for_each(|arg| match arg.as_str() {
        "key" => {
            let mut key: [u8; 64] = [0; 64];
            rng.fill_bytes(&mut key);
            let encoded: String = general_purpose::STANDARD_NO_PAD.encode(key);
            println!("{encoded}");
        }
        "jwk" => {
            let ec_jwk = jwk::generate();
            let serialized_value =
                serde_json::to_string_pretty(&ec_jwk).expect("failed to serialize ec jwk");
            println!("{serialized_value}");
        }
        _ => {}
    });
}
