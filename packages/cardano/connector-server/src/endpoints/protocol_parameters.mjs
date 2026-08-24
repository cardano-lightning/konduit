export async function endpointProtocolParameters(ctx) {
  return ctx.endpoint(undefined, async () => {
    const [parameters, tip] = await Promise.all([
      ctx.koios("/cli_protocol_params"),
      ctx.koios("/tip"),
    ]);
    const marker = Array.isArray(tip) ? tip[0] : tip;
    return {
      era: parameters.protocol_major >= 10 ? "conway" : "babbage",
      epoch: marker.epoch_no,
      slot: marker.abs_slot,
      payload: parameters,
    };
  });
}
