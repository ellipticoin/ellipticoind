const BaseContractClient = require('./BaseContractClient');
const MEMORY = [
  Object.freeze("Balances"),
  Object.freeze("Allowances"),
]

class BaseTokenClient extends BaseContractClient {
  constructor(params) {
    super(params)

    this.namespaces = [
      "Balances",
      "Allowances",
    ];
  }

  balanceOf(address) {
    return this.getLong("Balances", address)
  }
}

module.exports = BaseTokenClient;
