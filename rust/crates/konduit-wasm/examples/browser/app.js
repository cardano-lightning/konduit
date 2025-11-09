const wasm = wasm_bindgen;
wasm().then(async () => {
  wasm.enableLogs(wasm.LogLevel.Debug);

  const app = document.getElementById("app");
  let state = localStorage.getItem("state") ?? "no_user";
  let userSigningKey = localStorage.getItem("userSigningKey") ?? null;
  let userVerificationKey = localStorage.getItem("userVerificationKey") ?? null;
  let userBalance = null;
  let channel = localStorage.getItem("channel") ?? null;
  if (channel !== null) {
    const json = JSON.parse(channel);
    channel = {
      ...json,
      amount: BigInt(json.amount),
      closePeriod: BigInt(json.closePeriod),
      adaptor: asBuffer(json.adaptor),
      tag: asBuffer(json.tag),
    };
  }

  if (userSigningKey === null || userVerificationKey === null) {
    state = "no_user";
  } else {
    userSigningKey = asBuffer(userSigningKey);
    userVerificationKey = asBuffer(userVerificationKey);
  }

  let connector;
  // Try a local backend first, otherwise, prompt for one.
  try {
    connector = await wasm.CardanoConnector.new("http://localhost:8787");
  } catch (e) {
    const url = prompt("Please enter a connector URL");
    connector = await wasm.CardanoConnector.new(url);
  }

  await render();

  async function render() {
    app.innerHTML = "";
    if (state === "no_user") return renderLogin();
    if (state === "not_opened") return renderNotOpened();
    if (state === "opening") return renderOpening();
    if (state === "opened") return renderOpened();
  }

  // -------------------- STATES -------------------- //

  function renderLogin() {
    const input = el("input", {
      placeholder: "Consumer's signing key (64-hex)",
      id: "cred",
    });
    const btn = el("button", { text: "Login" });
    btn.onclick = async () => {
      const val = input.value.trim();
      if (!/^[0-9a-fA-F]{64}$/.test(val)) {
        alert("invalid credential: please specify 64 hex digits");
        return;
      }
      userSigningKey = asBuffer(val);
      userVerificationKey = wasm.toVerificationKey(userSigningKey);
      localStorage.setItem("state", "not_opened");
      localStorage.setItem("userSigningKey", asHexString(userSigningKey));
      localStorage.setItem(
        "userVerificationKey",
        asHexString(userVerificationKey),
      );
      userBalance = await fetchBalance(connector, userVerificationKey);
      state = "not_opened";
      render();
    };
    app.append(el("h2", { text: "Connect Wallet" }), input, btn);
  }

  async function renderNotOpened() {
    const header = await renderHeader();
    const amount = el("input", { placeholder: "Amount (ADA)", id: "amount" });
    const adaptor = el("input", {
      placeholder: "Adaptor's verification key (64-hex)",
      id: "adaptor",
    });
    const period = el("input", {
      placeholder: "Closing period (..s / ..min / ..h)",
      id: "period",
    });
    const btn = el("button", { text: "Open Channel" });
    const notice = el("div", { class: "notice" });

    btn.onclick = async () => {
      channel = {
        amount: null,
        adaptor: null,
        closePeriod: null,
        tag: null,
        status: "pending",
      };

      try {
        channel.amount = BigInt(Number.parseInt(amount.value, 10)) * 1000000n;
      } catch (e) {
        notice.textContent = e.message;
        return;
      }

      if (channel.amount <= 0n) {
        notice.textContent = "malformed/missing amount";
        return;
      } else if (channel.amount > BigInt(userBalance)) {
        notice.textContent = "insufficient balance";
        return;
      }

      channel.adaptor = adaptor.value.trim();
      if (!/^[0-9a-fA-F]{64}$/.test(channel.adaptor)) {
        notice.textContent = "malformed/missing adaptor";
        return;
      }
      channel.adaptor = asBuffer(channel.adaptor);

      try {
        if (period.value.endsWith("s")) {
          channel.closePeriod = BigInt(period.value.slice(0, -1));
        } else if (period.value.endsWith("min")) {
          channel.closePeriod = 60n * BigInt(period.value.slice(0, -3));
        } else if (period.value.endsWith("h")) {
          channel.closePeriod = 3600n * BigInt(period.value.slice(0, -1));
        } else {
          notice.textContent = "malformed/missing close period";
          return;
        }
      } catch (e) {
        notice.textContent = `malformed/missing close period: ${e.message}`;
        return;
      }

      state = "opening";
      render();

      try {
        await openChannel(connector, userSigningKey, channel);
        channel.status = "success";
        console.log(channel);
        render();
        state = "opened";
        localStorage.setItem("state", state);
        localStorage.setItem(
          "channel",
          JSON.stringify({
            ...channel,
            amount: channel.amount.toString(),
            closePeriod: channel.closePeriod.toString(),
            adaptor: asHexString(channel.adaptor),
            tag: asHexString(channel.tag),
          }),
        );
      } catch (e) {
        console.log(e);
        channel.status = "failed";
        notice.textContent = `❌ open failed: ${e.message}`;
        render();
        state = "not_opened";
      }

      await delay(1000);
      render();
    };

    app.append(header, amount, adaptor, period, btn, notice);
  }

  async function renderOpening() {
    const header = await renderHeader();
    const tx = el("div", { class: "tx" });
    tx.append(
      el("div", { text: `pending…` }),
      el("div", { class: "spinner" }),
      el("div", { text: `Status: ${channel.status}` }),
    );
    app.append(header, tx);
  }

  async function renderOpened() {
    const header = await renderHeader();
    const ch = el("fieldset", { class: "channel-info" });
    ch.append(
      el("legend", { text: "Channel" }),
      el("div", {
        text: `#${asHexString(channel.tag).slice(0, 12)} → ${asHexString(channel.adaptor).slice(0, 12)}`,
      }),
      el("div", {
        html: `<div style="display: flex; justify-content: center; align-items: center; gap: 0.5rem;"><img class="icons" src="icons/battery-charging.svg"><span>${showLovelace(channel.amount)}</span></div>`,
      }),
    );
    const notice = el("div", { class: "notice" });

    const close = el("button", { text: "Close Channel" });
    close.onclick = async () => {
      try {
        await closeChannel(connector, userSigningKey, channel);
        state = "not_opened";
        localStorage.setItem("state", state);
        localStorage.removeItem("channel");
      } catch (e) {
        console.log("CLOSE FAILED", e.toString());
        notice.textContent = `❌ close failed: ${e.message}`;
      }

      render();
    };

    app.append(header, notice, ch, close);
  }

  async function renderHeader() {
    if (userBalance === null) {
      userBalance = await fetchBalance(connector, userVerificationKey);
    }

    const header = el("header");
    header.append(
      el("p", {
        html: `<div style="display: flex; align-items: center; gap: 0.5rem;"><img class="icons" src="icons/credit-card.svg"><span>${showLovelace(userBalance)}</span></div>`,
      }),
    );
    header.append(
      el("p", {
        html: `<div style="display: flex; align-items: center; gap: 0.5rem;"><img class="icons" src="icons/user.svg"><span>${asHexString(userVerificationKey).slice(0, 12)}</span></div>`,
      }),
    );
    const exit = el("button", {
      html: `<img src="icons/log-out.svg" />`,
      class: "icon",
    });
    header.append(exit);

    exit.onclick = async () => {
      const confirmed = confirm("Exit and forget everything?");

      if (confirmed) {
        localStorage.removeItem("state");
        localStorage.removeItem("userSigningKey");
        localStorage.removeItem("userVerificationKey");
        localStorage.removeItem("balance");

        userSigningKey = null;
        userVerificationKey = null;
        userBalance = null;
        channel = null;

        state = "no_user";

        render();
      }
    };

    return header;
  }

  // -------------------- HELPERS -------------------- //

  function el(tag, props = {}) {
    const e = document.createElement(tag);
    for (attr in props) {
      if (attr === "text") {
        e.innerText = props[attr];
      } else if (attr === "html") {
        e.innerHTML = props[attr];
      } else if (attr === "class") {
        e.classList = props[attr];
      } else {
        e[attr] = props[attr];
      }
    }
    return e;
  }

  function delay(ms) {
    return new Promise((r) => setTimeout(r, ms));
  }

  function asBuffer(str) {
    return Uint8Array.from(str.match(/../g), (byte) => parseInt(byte, 16));
  }

  function asHexString(buf) {
    return [...buf].map((b) => b.toString(16).padStart(2, "0")).join("");
  }

  function showLovelace(n) {
    return `${(Number(n) / 1000000).toFixed(2)}₳`;
  }

  // -------------------- Async placeholders -------------------- //

  async function fetchBalance(connector, verificationKey) {
    const entry = localStorage.getItem("balance");

    if (entry != null) {
      const { balance, timestamp } = JSON.parse(entry);
      if (timestamp > Date.now()) {
        return BigInt(balance);
      }
    }

    const balance = await connector.balance(verificationKey);

    localStorage.setItem(
      "balance",
      JSON.stringify({
        balance: balance.toString(),
        timestamp: Date.now() + 30 * 1000,
      }),
    );

    return balance;
  }

  async function openChannel(connector, consumer, channel) {
    channel.tag = crypto.getRandomValues(new Uint8Array(32));

    const transaction = await wasm.open(
      // Cardano's connector backend
      connector,
      // tag: An (ideally) unique tag to discriminate channels and allow reuse of keys between them.
      channel.tag,
      // consumer: Consumer's verification key, allowed to *add* funds.
      wasm.toVerificationKey(consumer),
      // adaptor: Adaptor's verification key, allowed to *sub* funds
      channel.adaptor,
      // close_period: Minimum time from `close` to `elapse`, in seconds.
      channel.closePeriod,
      // deposit: Quantity of Lovelace to deposit into the channel
      channel.amount,
    );

    console.log("open", transaction.toString());

    await connector.signAndSubmit(transaction, consumer);
  }

  async function closeChannel(connector, consumer, channel) {
    const transaction = await wasm.close(
      connector,
      channel.tag,
      wasm.toVerificationKey(consumer),
      channel.adaptor,
      document.querySelector('meta[name="script_ref"]').content,
    );

    console.log("close", transaction.toString());

    await connector.signAndSubmit(transaction, consumer);
  }
});
