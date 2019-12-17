const WasmRPC = require('wasm-rpc').default;
const { execFileSync } = require('child_process');
var Long = require("long");
const cbor = require('cbor');
const { StringDecoder } = require('string_decoder');

class BaseContractClient extends WasmRPC {
  constructor(params) {
    super({
      exports: {
        rust_begin_unwind: () => null,
        __sender: () => {
          return this.writePointer(params.defaultSender);
        },
        __block_winner: () => {
          return this.writePointer(null);
        },
        __block_number: () => {
          return this.writePointer(0);
        },
        __get_memory: (keyPtr) => {
          var key = this.readPointer(keyPtr);
          if(this.getMemory(key)) {
            return this.writePointer(this.memory[key]);
          } else {
            return this.writePointer(new Uint8Array([]));
          }
        },
        __set_memory: (keyPtr, valuePtr) => {
          var key = this.readPointer(keyPtr);
          var value = this.readPointer(valuePtr);

          this.memory[key] = value;
          this.setMemory(key, value);
        },
        throw: console.log,
        rust_oom: () => null,
        __udivti3: () => null,
        __multi3: () => null,
        _call: (codePtr, methodPtr, paramsPtr, memoryContextPtr) => {
          const decoder = new StringDecoder('utf8');
          var code = this.readPointer(codePtr);
          var method = decoder.write(Buffer.from(this.readPointer(methodPtr)));
          var params = cbor.decode(Buffer.from(this.readPointer(paramsPtr)));
          var storageContext = this.readPointer(storageContextPtr);
          return this.writePointer(this._call(code, method, params, storageContext));
        },
      }
    })

    this.memory = this.memory || {};
  }

  getMemory(key) {
    // console.log(`${key} -> ${this.memory[key]}`)
    return this.memory[key]
  }

  setMemory(key, value) {
    // console.log(key);
    // console.log(`${key} = ${value}`)
    this.memory[key] = value;
  }

  getLong(namespace, key) {
    let value = this.get(namespace, key);

    if (value) {
      return Long.fromBytesLE(value);
    } else {
      return Long.fromNumber(0);
    }
  }

  get(namespace, key) {
    let fullKey = new Uint8Array(1 + key.length);
    fullKey.set(new Uint8Array([this.namespaces.indexOf(namespace)]));
    fullKey.set(key, 1);
    return this.getMemory(fullKey);
  }

  reset() {
    this.memory = {};
  }
  _call(code, method, params, storageContext) {
    var runWasmCode = `
      const cbor = require('cbor');
      const WasmRPC = require('/Users/masonf/src/wasm-rpc');
      const code = Buffer.from("${Buffer.from(code).toString("hex")}", "hex");
      const storageContext = Buffer.from("${Buffer.from(storageContext).toString("hex")}", "hex");
      const params = cbor.decode(Buffer.from("${Buffer.from(cbor.encode(params).toString("hex"))}", "hex"));
      const method = "${method}";
      function hexToBytes(hex) {
          for (var bytes = [], c = 0; c < hex.length; c += 2)
          bytes.push(parseInt(hex.substr(c, 2), 16));
          return Buffer.from(bytes);
      }

      async function run() {
        const wasm = new WasmRPC({
          exports: {
            sender: () => {
              return wasm.writePointer(hexToBytes("0000000000000000000000000000000000000001"));
            },
            read: (keyPtr) => {
              this.memory = this.memory || {};
              var key = wasm.readPointer(keyPtr);
              if(this.memory[key]) {
                return wasm.writePointer(this.memory[key]);
              } else {
                return wasm.writePointer(new Uint8Array([]));
              }
            },
            write: (keyPtr, valuePtr) => {
              var key = wasm.readPointer(keyPtr);
              var value = wasm.readPointer(valuePtr);

              this.memory = this.memory || {};
              this.memory[key] = value;
            },
            _call: () => null,
            rust_begin_unwind: () => null,
          }
        });
        await wasm.load(code);
        const result = wasm.call(method, ...params);
        process.stdout.write(Buffer.concat([
          Buffer.from([0,0,0,0]),
          Buffer.from(cbor.encode(result)),
        ]));
      };
      run();`;

    return execFileSync('node', ['-e', runWasmCode]);
  }
}

module.exports = BaseContractClient;
