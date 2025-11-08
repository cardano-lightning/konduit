const wasm = wasm_bindgen;
wasm().then(async () => {
  wasm.enableLogs(wasm.LogLevel.Debug);

  const app = document.getElementById("app");
  let state = localStorage.getItem("state") ?? "no_user";
  let userSigningKey = localStorage.getItem("userSigningKey") ?? null;
  let userVerificationKey = localStorage.getItem("userVerificationKey") ?? null;
  let userBalance = null;
  let channel = null;

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
    const input = el("input", { placeholder: "Consumer's signing key (64-hex)", id: "cred" });
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
      localStorage.setItem("userVerificationKey", asHexString(userVerificationKey));
      userBalance = await fetchBalance(connector, userVerificationKey);
      state = "not_opened";
      render();
    };
    app.append(
      el("h2", { text: "Connect Wallet" }),
      input,
      btn
    );
  }

  async function renderNotOpened() {
    if (userBalance === null) {
      userBalance = await fetchBalance(connector, userVerificationKey);
    }

    const header = el("header", { text: `Balance: ${showLovelace(userBalance)}` });
    const amount = el("input", { placeholder: "Amount (ADA)", id: "amount" });
    const adaptor = el("input", { placeholder: "Adaptor's verification key (64-hex)", id: "adaptor" });
    const period = el("input", { placeholder: "Closing period (..s / ..min / ..h)", id: "period" });
    const btn = el("button", { text: "Open Channel" });
    const notice = el("div", { class: "notice" });

    btn.onclick = async () => {
      let a;
      try {
        a = BigInt(Number.parseInt(amount.value, 10)) * 1000000n;
      } catch (e) {
        notice.textContent = e.message;
        return;
      }

      if (a <= 0n) {
        notice.textContent = "malformed/missing amount";
        return;
      } else if (a > BigInt(userBalance)) {
        notice.textContent = "insufficient balance";
        return;
      }

      let k = adaptor.value.trim();
      if (!/^[0-9a-fA-F]{64}$/.test(k)) {
        notice.textContent = "malformed/missing adaptor";
        return;
      }
      k = asBuffer(k)

      let p;
      try {
        if (period.value.endsWith("s")) {
          p = BigInt(period.value.slice(0, -1));
        } else if (period.value.endsWith("min")) {
          p = 60n * BigInt(period.value.slice(0, -3))
        } else if (period.value.endsWith("h")) {
          p = 3600n * BigInt(period.value.slice(0, -1))
        } else {
          notice.textContent = "malformed/missing closing period";
          return;
        }
      } catch(e) {
          notice.textContent = `malformed/missing closing period: ${e.message}`;
          return;
      }

      state = "opening";
      channel = { amount: a, adaptor: k, period: period.value, status: "pending" };

      render();

      try {
        channel.tag = await openChannel(connector, userSigningKey, k, p, a);
        channel.status = "success";
        render();
        state = "opened";
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

  function renderOpening() {
    const header = el("header", { text: `Balance: ${showLovelace(userBalance)}` });
    const tx = el("div", { class: "tx" });
    tx.append(
      el("div", { text: `pending…` }),
      el("div", { class: "spinner" }),
      el("div", { text: `Status: ${channel.status}` })
    );
    app.append(header, tx);
  }

  function renderOpened() {
    const header = el("header", { text: `Balance: ${showLovelace(userBalance)}` });
    const ch = el("div", { class: "channel-info" });
    ch.append(
      el("div", { text: `Open Channel (${asHexString(channel.tag).slice(0, 12)})` }),
      el("div", { class: "amount", text: `${showLovelace(channel.amount)}` }),
    );
    app.append(header, ch);
  }

  // -------------------- HELPERS -------------------- //

  function el(tag, props = {}) {
    const e = document.createElement(tag);
    for (attr in props) {
      if (attr === "text") {
        e.innerText = props[attr];
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
    return Uint8Array.from(
      str.match(/../g),
      byte => parseInt(byte, 16)
    );
  }

  function asHexString(buf) {
    return [...buf].map(b => b.toString(16).padStart(2, "0")).join("");
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

    localStorage.setItem("balance", JSON.stringify({ balance: balance.toString(), timestamp: Date.now() + 60 * 1000 }));

    return balance;
  }

  async function openChannel(connector, consumer, adaptor, period, amount) {
    const tag = crypto.getRandomValues(new Uint8Array(32));

    const transaction = await wasm.open(
      // Cardano's connector backend
      connector,
      // tag: An (ideally) unique tag to discriminate channels and allow reuse of keys between them.
      tag,
      // consumer: Consumer's verification key, allowed to *add* funds.
      wasm.toVerificationKey(consumer),
      // adaptor: Adaptor's verification key, allowed to *sub* funds
      adaptor,
      // close_period: Minimum time from `close` to `elapse`, in seconds.
      period,
      // deposit: Quantity of Lovelace to deposit into the channel
      amount,
    );

    console.log(transaction.toString())

    await connector.signAndSubmit(transaction, consumer);

    return tag;
  }
});
