const BaseTokenClient = require('./support/BaseTokenClient');
const assert = require('assert');
const _ = require('lodash');
const UNKNOWN_ADDRESS = Buffer.from("0000000000000000000000000000000000000000", "hex");
const SENDER = Buffer.from("0000000000000000000000000000000000000001", "hex");
const RECEIVER = Buffer.from("0000000000000000000000000000000000000002", "hex");
const ERROR_INSUFFICIENT_FUNDS = 1;
const ERROR_CODES = {
  INSUFFIENT_FUNDS: 1,
};

describe('BaseToken', function() {
  var token;

  beforeEach(async () => {
    token = new BaseTokenClient({
      defaultSender: SENDER,
    });

    await token.loadFile("../target/wasm32-unknown-unknown/debug/base_token.wasm");
  });

  afterEach(() => token.reset());

  describe('constructor', function() {
    it('should initalize the sender with 100 tokens', async function() {
      await token.call('constructor', 100);
      var result = token.balanceOf(SENDER);

      assert.equal(result, 100);
    });
  });

  describe('balanceOf', function() {
    it('should return your balance', async function() {
      await token.call('constructor', 100);
      var result = await token.balanceOf(SENDER);

      assert.equal(result, 100);
    });

    it('should return 0 for unknown addresses', async function() {
      await token.call('constructor', 100);
      var result = await token.balanceOf(UNKNOWN_ADDRESS);

      assert.equal(result, 0);
    });
  });

  describe('transfer', function() {
    it('decreases the senders balance by the amount specified', async function() {
      await token.call('constructor', 100);
      await token.call('transfer', RECEIVER, 20);

      var result = await token.balanceOf(SENDER);

      assert.equal(result, 80);
    });

    it('increases the receivers balance by the amount specified', async function() {
      await token.call('constructor', 100);
      await token.call('transfer', RECEIVER, 20);

      var result = await token.balanceOf(RECEIVER);

      assert.equal(result, 20);
    });

    it('returns an error if you try to send more tokens than you have', async function() {
      await token.call("transfer", RECEIVER, 120).catch((error) => {
        assert.equal(error.message, "insufficient funds")
        assert.equal(error.code, ERROR_CODES.INSUFFIENT_FUNDS)
      });
    });
  });
});
