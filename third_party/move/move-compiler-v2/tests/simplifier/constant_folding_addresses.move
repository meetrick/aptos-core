//# publish --print-bytecode
module 0xcafe::Addresses {
    use std::vector;
    public fun test() {
        let addresses = vector[@0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234, @0x1234];
        assert!(vector::length(&addresses) == 1845, 1);
    }
}

//# run 0xcafe::Addresses::test
