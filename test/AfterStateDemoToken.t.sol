// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {AfterStateDemoToken} from "../contracts/AfterStateDemoToken.sol";

interface VmDemoToken {
    function addr(uint256 privateKey) external returns (address);
    function prank(address caller) external;
}

contract AfterStateDemoTokenTest {
    VmDemoToken private constant vm =
        VmDemoToken(address(uint160(uint256(keccak256("hevm cheat code")))));

    uint256 private constant OWNER_KEY = 0xA57A7E;
    address private owner;
    address private payer = address(0xBEEF);
    address private recipient = address(0xCAFE);
    AfterStateDemoToken private token;

    function setUp() public {
        owner = vm.addr(OWNER_KEY);
        token = new AfterStateDemoToken(owner);
    }

    function testOwnerMintAndExactTransferFrom() public {
        vm.prank(owner);
        token.mint(payer, 2_000_000);

        vm.prank(payer);
        token.approve(address(this), 1_250_000);
        token.transferFrom(payer, recipient, 1_250_000);

        require(token.balanceOf(recipient) == 1_250_000, "wrong recipient balance");
        require(token.balanceOf(payer) == 750_000, "wrong payer balance");
        require(token.allowance(payer, address(this)) == 0, "allowance not consumed");
        require(token.decimals() == 6, "unexpected decimals");
    }

    function testNonOwnerCannotMint() public {
        (bool ok,) = address(token).call(
            abi.encodeCall(AfterStateDemoToken.mint, (recipient, 1))
        );
        require(!ok, "non-owner minted demo tokens");
    }
}
