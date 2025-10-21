import * as cbor from "cbor-x";
import * as ed from "@noble/ed25519";
import { sha512 } from "@noble/hashes/sha2.js";
ed.hashes.sha512 = sha512;

export function unpack({ verificationKey, tag, cheque }) {
  const vkey = Buffer.from(verificationKey, "hex");
  const l = cheque.length;
  const sigL = 64 * 2;
  const bodyHex = cheque.slice(2, l - (sigL + 6));
  const sigHex = cheque.slice(l - (sigL + 6), l - 2);
  const msg = Buffer.from(tag + bodyHex, "hex");
  const sig = Buffer.from(sigHex.slice(4), "hex");
  const isSigned = ed.verify(sig, msg, vkey);
  // FIXME :: OTHER VERIFICATION STEPS REQUIRED
  if (isSigned) {
    const [index, amount, timeout, lock] = cbor.decode(
      Buffer.from(bodyHex, "hex"),
    );
    console.log("UNPACKED", { index, amount, timeout, lock });
    return { index, amount, timeout, lock };
  }
}

// let serializedAsBuffer = cbor.encode(value);
//
// // Sync methods can be used now:
// const { secretKey, publicKey } = ed.keygen();
// // const publicKey = ed.getPublicKey(secretKey);
// const sig = ed.sign(msg, secretKey);
