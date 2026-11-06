pub mod comet {
    soroban_sdk::contractimport!(file = "./src/dependencies/comet.wasm");
}

#[allow(clippy::too_many_arguments)]
pub mod backstop {
    soroban_sdk::contractimport!(file = "./src/dependencies/backstop.wasm");
}
