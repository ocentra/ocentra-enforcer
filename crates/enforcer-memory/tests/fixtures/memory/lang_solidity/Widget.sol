// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "./IWidget.sol";
using SafeMath for uint256;

type WidgetId is uint256;

interface IWidget {
    function draw() external view returns (string memory);
}

contract Widget is IWidget {
    string public name;
    uint256 public count;

    event WidgetDrawn(string name);

    constructor(string memory initialName) {
        name = initialName;
    }

    function draw() external view returns (string memory) {
        return name;
    }

    function increment(uint256 amount) public returns (uint256) {
        if (amount > 0) {
            count += amount;
        } else {
            count += 1;
        }
        for (uint256 i = 0; i < amount; i++) {
            emit WidgetDrawn(name);
        }
        return count;
    }

    function registerHelper(address helper) public {
        Helper h = new Helper(helper);
        h.register();
    }
}

contract Helper {
    address public target;

    constructor(address target_) {
        target = target_;
    }

    function register() public {}
}
