function hexToAddress(hex) {
  return Address.encode(Address.create({bytes: hexToBytes(hex)})).finish();
}
function hexToBytes(hex) {
    for (var bytes = [], c = 0; c < hex.length; c += 2)
    bytes.push(parseInt(hex.substr(c, 2), 16));
    return new Buffer(bytes);
}

// Convert a byte array to a hex string
function bytesToHex(bytes) {
    for (var hex = [], i = 0; i < bytes.length; i++) {
        hex.push((bytes[i] >>> 4).toString(16));
        hex.push((bytes[i] & 0xF).toString(16));
    }
    return hex.join("");
}

function toBytesInt32 (num) {
    arr = new ArrayBuffer(4);
    view = new DataView(arr);
    view.setUint32(0, num);
    return new Uint8Array(arr);
}
function fromBytesInt32 (buffer) {
    arr = new ArrayBuffer(4);
    view = new DataView(arr);
    buffer.forEach((value, index) => view.setUint8(index, value));
    return view.getUint32(0);
}


module.exports = {
  fromBytesInt32,
  toBytesInt32,
  hexToBytes,
  hexToAddress,
  bytesToHex,
}
