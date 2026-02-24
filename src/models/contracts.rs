alloy::sol! {
    #[derive(Debug)]
    struct TradeOrder {
        address sender;
        bytes32 subaccount;
        uint128 quantity;
        uint128 price;
        bool reduceOnly;
        uint8 side;
        uint8 engineType;
        uint32 productId;
        uint64 nonce;
        uint64 signedAt;
    }
}

alloy::sol! {
    #[derive(Debug)]
    struct CancelOrder {
        address sender;
        bytes32 subaccount;
        uint64 nonce;
    }
}
