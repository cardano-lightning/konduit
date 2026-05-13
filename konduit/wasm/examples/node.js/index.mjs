import konduit from "./konduit-wasm/konduit_wasm.js";

// ----------------------------------------------------------------------- DEBUG

// Enable some better debugging when working with the WASM bundle.
process.on("uncaughtException", (e) => {
  if (e instanceof konduit.StrError) {
    console.log(`Error: ${e.toString()}`);
    if (e.stack) {
      console.log(e.stack);
    }
  } else {
    throw e;
  }
});

konduit.enableLogs(konduit.LogLevel.Debug);

// ---------------------------------------------------------------------- CONFIG

const consumerSigningKey = Buffer.from(
  process.env.KONDUIT_CONSUMER_SIGNING_KEY,
  "hex",
);

const consumerVerificationKey = konduit.toVerificationKey(consumerSigningKey);

const adaptorVerificationKey = Buffer.from(
  process.env.KONDUIT_ADAPTOR_VERIFICATION_KEY,
  "hex",
);

// ----------------------------------------------------------------------- SETUP

const connector = await konduit.CardanoConnector.new(
  process.env.KONDUIT_CARDANO_CONNECTOR,
);

// ------------------------------------------------------------------------ OPEN

const open = await konduit.open(
  // Cardano's connector backend
  connector,
  // tag: An (ideally) unique tag to discriminate channels and allow reuse of keys between them.
  Buffer.from("konduit-tag-001"),
  // consumer: Consumer's verification key, allowed to *add* funds.
  consumerVerificationKey,
  // adaptor: Adaptor's verification key, allowed to *sub* funds
  adaptorVerificationKey,
  // close_period: Minimum time from `close` to `elapse`, in seconds.
  24n * 3600n,
  // deposit: Quantity of Lovelace to deposit into the channel
  2000000n,
);

await connector.signAndSubmit(open, consumerSigningKey);

// ----------------------------------------------------------------------- CLOSE

console.log(open.toString());
