use alloy::primitives::{Address, address};
use alloy::sol;

mod init;
mod latency;

const RPC_URL: &str = "https://rpc.etherealtest.net";
const EXCHANGE: Address = address!("1F0327A80e43FEF1Cd872DC5d38dCe4A165c0643");
const TOKEN: Address = address!("b7ae43711d85c23dc862c85b9c95a64dc6351f90");

sol! {
    #[sol(rpc)]
    interface IERC20 {
        function approve(address spender, uint256 amount) external returns (bool);
        function balanceOf(address account) external view returns (uint256);
    }

    #[sol(rpc)]
    interface IWUSDE {
        function deposit() external payable;
    }

    #[sol(rpc)]
    interface IExchange {
        function deposit(
            bytes32 subaccount,
            address depositToken,
            uint256 amount,
            bytes32 referralCode
        ) external;
    }
}
